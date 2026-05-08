#!/usr/bin/env bash
# Print the UDID of the first available iPhone simulator on the newest installed
# iOS runtime. Mirrors the dynamic-discovery approach used in
# .github/workflows/ios-ci.yml so simulator names rotating each Xcode release
# don't break local + CI smoke tests.
#
# Usage:  DEVICE_ID=$(./scripts/pick-simulator.sh)
#         xcodebuild test -destination "platform=iOS Simulator,id=${DEVICE_ID}"

set -euo pipefail

xcrun simctl list devices available --json | python3 -c "
import json, sys, re
data = json.load(sys.stdin)
runtimes = [k for k in data['devices'].keys() if 'iOS' in k]
def ver(rt):
    m = re.search(r'iOS-(\d+)-(\d+)', rt)
    return (int(m.group(1)), int(m.group(2))) if m else (0, 0)
for rt in sorted(runtimes, key=ver, reverse=True):
    for d in data['devices'][rt]:
        if d.get('isAvailable') and 'iPhone' in d['name']:
            print(d['udid'])
            sys.exit(0)
sys.exit('no available iPhone simulator found')
"
