// swift-tools-version: 5.10
import PackageDescription

// YrsFFI — precompiled Apple binary for the y-crdt C ABI (yffi).
//
// Wraps the upstream `yffi` crate from y-crdt/y-crdt as a SwiftPM-consumable
// binary target. The `YrsFFI.xcframework` directory is committed to this
// repo so consumers get a working artifact without needing Rust installed
// locally.
//
// Build the xcframework: ./scripts/build-xcframework.sh
// Bump yffi version: edit include/VERSION, rebuild, commit.
let package = Package(
    name: "YrsFFI",
    platforms: [
        .iOS(.v17),
        .macOS(.v14),
    ],
    products: [
        .library(name: "YrsFFI", targets: ["YrsFFI"]),
    ],
    targets: [
        .binaryTarget(
            name: "YrsFFI",
            path: "YrsFFI.xcframework"
        ),
        .testTarget(
            name: "YrsFFITests",
            dependencies: ["YrsFFI"]
        ),
    ]
)
