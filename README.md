# YrsFFI

Precompiled binaries for the [y-crdt](https://github.com/y-crdt/y-crdt) Rust core, packaged for two platforms:

- **Apple** — the C ABI ([`yffi`](https://crates.io/crates/yffi)) as a SwiftPM `binaryTarget` (`YrsFFI.xcframework`).
- **Android** — a JNI binding to the [`yrs`](https://crates.io/crates/yrs) crate as a Gradle AAR (`android/`).

This package exists so apps can consume `y-crdt` (the canonical Yjs implementation in Rust) without each app needing its own Rust toolchain or build pipeline. Idiomatic wrappers (`YDoc`, `YXmlFragment`, `YText`, etc.) and any higher-level protocol implementation (e.g., Hocuspocus sync) are expected to live in downstream consumer packages — this repo intentionally stays a thin binary distribution layer.

## Why y-crdt and not y-swift

[y-swift](https://github.com/y-crdt/yswift) was abandoned in mid-2024. y-crdt's `yffi` is the actively maintained C binding in the y-crdt monorepo and exposes the full Yjs surface — Doc, Transaction, Text, XmlFragment / XmlElement / XmlText, plus the wire-protocol primitives needed for Hocuspocus sync.

## Usage (Apple / SwiftPM)

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

## Usage (Android / Gradle)

The Android AAR is served from GitHub via [JitPack](https://jitpack.io) — no separate publish/release pipeline, no Rust on the consumer side.

```kotlin
// settings.gradle.kts (or build.gradle.kts) in a consuming repo
dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
        maven("https://jitpack.io")
    }
}

// module build.gradle.kts
dependencies {
    implementation("com.github.Contio-AI:YrsFFI:0.25.1")
}
```

```kotlin
// Kotlin code — YrsNative is the raw JNI boundary (package ai.contio.yrs)
import ai.contio.yrs.YrsNative

val doc = YrsNative.docNew()
try {
    val txn = YrsNative.writeTransaction(doc)
    val text = YrsNative.textRoot(doc, "body")
    YrsNative.textInsert(text, txn, 0, "hello")
    YrsNative.commitTransaction(txn)
} finally {
    YrsNative.docDestroy(doc)
}
```

## What's included

Both binaries are committed to the repo so consumers don't need Rust installed.

**Apple — `YrsFFI.xcframework/`** (three slices):

| Slice | Architectures | Where it runs |
|-------|---------------|---------------|
| `ios-arm64` | aarch64 | iPhone / iPad device |
| `ios-arm64_x86_64-simulator` | aarch64 + x86_64 | iOS Simulator on Apple Silicon AND Intel Macs |
| `macos-arm64_x86_64` | aarch64 + x86_64 | macOS app on Apple Silicon AND Intel Macs |

**Android — `android/yrs-android/src/main/jniLibs/`** (four ABIs, packaged into the AAR):

| ABI | Where it runs |
|-----|---------------|
| `arm64-v8a` | modern Android devices (64-bit ARM) |
| `armeabi-v7a` | older 32-bit ARM devices |
| `x86_64` | Android emulator on Intel/AMD |
| `x86` | legacy 32-bit emulator |

## Versioning

Package versions match the upstream `yffi` version it wraps. Bumping yffi means a new major or minor release here. If we need to ship a fix that doesn't change yffi (e.g., a new slice, a build-script change), we bump the patch version.

| YrsFFI tag | yffi / yrs version | notes |
|------------|--------------------|-------|
| `0.25.0`   | 0.25.0 | initial Apple xcframework |
| `0.25.1`   | 0.25.0 | adds the Android AAR (no upstream change) |

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

# 3. Rebuild the Apple xcframework and the Android .so
rm -rf yffi-source build
./scripts/build-xcframework.sh
./scripts/build-aar.sh

# 4. Verify
swift test
xcodebuild test -scheme YrsFFI -destination "platform=iOS Simulator,id=$(./scripts/pick-simulator.sh)"
(cd android && ./gradlew :yrs-android:assembleRelease)

# 5. Commit + tag + push
git add include/ YrsFFI.xcframework/ android/ scripts/ Package.swift README.md
git commit -m "feat: bump to 0.26.0"
git tag 0.26.0
git push origin main 0.26.0
```

## Required local toolchain (only for maintainers rebuilding)

Consumers do NOT need Rust — the committed binaries are what get used.

For maintainers who need to rebuild:

```bash
# Apple
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios \
                  aarch64-apple-darwin x86_64-apple-darwin
# Android
rustup target add aarch64-linux-android armv7-linux-androideabi \
                  i686-linux-android x86_64-linux-android
cargo install cargo-ndk
```

Plus Xcode command line tools (`xcode-select --install`) for Apple, and an Android NDK
(`ANDROID_NDK_HOME`, or installed under `$ANDROID_HOME/ndk/`) for Android.

## Repo layout

```
YrsFFI/
├── Package.swift              # SwiftPM manifest, declares binaryTarget(path:)
├── YrsFFI.xcframework/        # committed Apple binary
├── include/                   # Source for rebuilding (vendored from upstream y-crdt)
│   ├── libyrs.h               # C header pinned to a y-crdt tag
│   ├── module.modulemap       # Clang module declaration: `module YrsFFI`
│   └── VERSION                # yffi version + header sha256
├── android/                   # Android Gradle library (consumed via JitPack)
│   ├── rust/                  # JNI shim crate over the `yrs` crate
│   ├── yrs-android/
│   │   ├── build.gradle.kts   # android-library + maven-publish
│   │   └── src/main/jniLibs/  # committed per-ABI libyrs_android.so
│   └── settings.gradle.kts
├── scripts/
│   ├── build-xcframework.sh   # Rebuilds YrsFFI.xcframework from yffi crates.io source
│   ├── build-aar.sh           # Rebuilds the Android .so via cargo-ndk
│   └── pick-simulator.sh      # CI helper: picks newest available iPhone sim UDID
├── jitpack.yml                # Builds the android/ Gradle project for JitPack
├── Tests/YrsFFITests/
│   └── YrsFFISmokeTests.swift # Lifecycle + Y.Text round-trip
└── .github/workflows/ci.yml   # PR CI: smoke tests + rebuild-from-source verification
```

## License

The vendored `libyrs.h` is MIT-licensed (from y-crdt). This wrapper inherits MIT.
