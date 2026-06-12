#!/usr/bin/env bash
# Rebuild the per-ABI libyrs_android.so under android/yrs-android/src/main/jniLibs
# from android/rust (the JNI shim over the `yrs` crate), via cargo-ndk.
#
# Consumers do NOT need this — the .so files are committed (like YrsFFI.xcframework).
# Only maintainers run this when bumping yrs/yffi or changing the JNI surface.
#
# Requires: rustup with the Android targets, cargo-ndk, and an Android NDK.
set -euo pipefail

# Prefer rustup's cargo: a conda/pixi cargo earlier on PATH lacks the Android
# targets (same reason build-xcframework.sh pins ~/.cargo/bin).
if [ -x "${HOME}/.cargo/bin/cargo" ]; then
  export PATH="${HOME}/.cargo/bin:${PATH}"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ANDROID_DIR="$(cd "${SCRIPT_DIR}/../android" && pwd)"

# Locate an NDK if ANDROID_NDK_HOME is unset.
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  export ANDROID_NDK_HOME="$(ls -d "${ANDROID_HOME:-$HOME/Library/Android/sdk}"/ndk/* 2>/dev/null | tail -1)"
fi
echo "==> NDK: ${ANDROID_NDK_HOME}"

rustup target add aarch64-linux-android armv7-linux-androideabi \
                  i686-linux-android x86_64-linux-android >/dev/null 2>&1 || true

cd "${ANDROID_DIR}/rust"
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86 -t x86_64 \
  -o "${ANDROID_DIR}/yrs-android/src/main/jniLibs" build --release

echo "==> Rebuilt:"
find "${ANDROID_DIR}/yrs-android/src/main/jniLibs" -name '*.so'
