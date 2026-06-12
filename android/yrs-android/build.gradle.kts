// yrs-android — thin JNI binding to the vendored `yrs` Rust CRDT core, the
// Android sibling of this repo's committed YrsFFI.xcframework. The per-ABI
// libyrs_android.so under src/main/jniLibs is COMMITTED (consumers need no
// Rust); maintainers regenerate it with ../../scripts/build-aar.sh.
//
// Published as a Maven artifact so it can be consumed from GitHub via JitPack:
//   repositories { maven("https://jitpack.io") }
//   implementation("com.github.Contio-AI.YrsFFI:yrs-android:<tag>")
plugins {
    // AGP 9.0 provides Kotlin support natively — the org.jetbrains.kotlin.android
    // plugin must NOT be applied (it errors as "no longer required").
    id("com.android.library")
    id("maven-publish")
}

android {
    namespace = "ai.contio.yrs"
    compileSdk = 36

    defaultConfig {
        minSdk = 26
        // Ship only the ABIs we build the yrs .so for.
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    // Expose the release variant to maven-publish.
    publishing {
        singleVariant("release") {
            withSourcesJar()
        }
    }
}

// JitPack runs `publishToMavenLocal`; the group/version are injected by JitPack
// (com.github.<owner>.<repo>), so we only declare the component + artifactId.
publishing {
    publications {
        register<MavenPublication>("release") {
            artifactId = "yrs-android"
            afterEvaluate { from(components["release"]) }
        }
    }
}
