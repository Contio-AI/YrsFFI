# YrsFFI

Precompiled Apple binary for the [y-crdt](https://github.com/y-crdt/y-crdt) C ABI ([`yffi`](https://crates.io/crates/yffi)), packaged as a SwiftPM library.

This package exists so Apple apps (iOS, macOS) can consume `y-crdt` (the canonical Yjs implementation in Rust) without each app needing its own Rust toolchain or build pipeline. Idiomatic Swift wrappers (`YDoc`, `YXmlFragment`, `YText`, etc.) and any higher-level protocol implementation (e.g., Hocuspocus sync) are expected to live in downstream consumer packages — this repo intentionally stays a thin binary distribution layer.

## Why y-crdt and not y-swift

[y-swift](https://github.com/y-crdt/yswift) was abandoned in mid-2024. y-crdt's `yffi` is the actively maintained C binding in the y-crdt monorepo and exposes the full Yjs surface — Doc, Transaction, Text, XmlFragment / XmlElement / XmlText, plus the wire-protocol primitives needed for Hocuspocus sync.

## Usage

```swift
// Package.swift in a consuming repo
.package(url: "https://github.com/Contio-AI/YrsFFI", from: "0.25.0")

// Then in a target:
.target(
    name: "MyLib",
    dependencies: [
        .product(name: "YrsFFI", package: "YrsFFI")
    ]
)
```

```swift
// Swift code
import YrsFFI

let doc = ydoc_new()
defer { ydoc_destroy(doc) }
// ... call into the C API
```

## What's included

The `YrsFFI.xcframework/` is committed to the repo so consumers don't need Rust installed. Three slices:

| Slice | Architectures | Where it runs |
|-------|---------------|---------------|
| `ios-arm64` | aarch64 | iPhone / iPad device |
| `ios-arm64_x86_64-simulator` | aarch64 + x86_64 | iOS Simulator on Apple Silicon AND Intel Macs |
| `macos-arm64_x86_64` | aarch64 + x86_64 | macOS app on Apple Silicon AND Intel Macs |

## Versioning

Package versions match the upstream `yffi` version it wraps. Bumping yffi means a new major or minor release here. If we need to ship a fix that doesn't change yffi (e.g., a new slice, a build-script change), we bump the patch version.

| YrsFFI tag | yffi version |
|------------|--------------|
| `0.25.0`   | 0.25.0 (initial) |

## Maintainers — bumping the yffi version

```bash
# 1. Edit include/VERSION:
#       yffi: 0.26.0
#       header_source: https://github.com/y-crdt/y-crdt/blob/v0.26.0/tests-ffi/include/libyrs.h
#       header_sha256: <new sha>

# 2. Re-vendor the C header from the matching upstream tag
curl -fsSL -o include/libyrs.h \
  https://raw.githubusercontent.com/y-crdt/y-crdt/v0.26.0/tests-ffi/include/libyrs.h
shasum -a 256 include/libyrs.h
# update include/VERSION with the new sha

# 3. Rebuild the xcframework
rm -rf yffi-source build
./scripts/build-xcframework.sh

# 4. Verify
swift test
xcodebuild test -scheme YrsFFI -destination "platform=iOS Simulator,id=$(./scripts/pick-simulator.sh)"

# 5. Commit + tag + push
git add include/ YrsFFI.xcframework/ scripts/ Package.swift README.md
git commit -m "feat: bump yffi to 0.26.0"
git tag 0.26.0
git push origin main 0.26.0
```

## Required local toolchain (only for maintainers rebuilding)

Consumers do NOT need Rust. The committed xcframework is what gets used.

For maintainers who need to rebuild:

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios \
                  aarch64-apple-darwin x86_64-apple-darwin
```

Plus Xcode command line tools (`xcode-select --install`).

## Repo layout

```
YrsFFI/
├── Package.swift              # SwiftPM manifest, declares binaryTarget(path:)
├── YrsFFI.xcframework/        # committed binary (~96 MB unstripped, 35 MB zipped)
├── include/                   # Source for rebuilding (vendored from upstream y-crdt)
│   ├── libyrs.h               # C header pinned to a y-crdt tag
│   ├── module.modulemap       # Clang module declaration: `module YrsFFI`
│   └── VERSION                # yffi version + header sha256
├── scripts/
│   ├── build-xcframework.sh   # Rebuilds YrsFFI.xcframework from yffi crates.io source
│   └── pick-simulator.sh      # CI helper: picks newest available iPhone sim UDID
├── Tests/YrsFFITests/
│   └── YrsFFISmokeTests.swift # Lifecycle + Y.Text round-trip
└── .github/workflows/ci.yml   # PR CI: smoke tests + rebuild-from-source verification
```

## License

The vendored `libyrs.h` is MIT-licensed (from y-crdt). This wrapper inherits MIT.
