#!/usr/bin/env bash
# Build YrsFFI.xcframework from the upstream yffi crate.
#
# yffi declares crate-type = ["staticlib", "cdylib"] (no rlib), so it cannot
# be a regular Rust dependency. This script fetches yffi's source from
# crates.io and invokes cargo on it directly via --manifest-path. yffi's
# [lib].name is "yrs", so the output is libyrs.a, which we lipo into
# universal slices and assemble into an XCFramework.
#
# Output: ./YrsFFI.xcframework/  (replaces in-place; commit when bumping yffi)
# Scratch: ./build/ (gitignored, intermediate)
#
# Targets covered:
#   - aarch64-apple-ios          (iPhone/iPad device, ARM64)
#   - aarch64-apple-ios-sim      (iOS simulator on Apple Silicon)
#   - x86_64-apple-ios           (iOS simulator on Intel — legacy name)
#   - aarch64-apple-darwin       (macOS, Apple Silicon)
#   - x86_64-apple-darwin        (macOS, Intel)

set -euo pipefail

# Prefer rustup's toolchain when present. Some environments (e.g., conda /
# pixi) ship a cargo without Apple iOS targets; rustup is the standard.
if [ -x "${HOME}/.cargo/bin/cargo" ]; then
  export PATH="${HOME}/.cargo/bin:${PATH}"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${PROJECT_DIR}"

BUILD_DIR="${PROJECT_DIR}/build"
SOURCE_DIR="${PROJECT_DIR}/yffi-source"
LIB_NAME="libyrs.a"
FRAMEWORK_NAME="YrsFFI"
FINAL_FRAMEWORK="${PROJECT_DIR}/${FRAMEWORK_NAME}.xcframework"

# Read pinned yffi version from VERSION file.
YFFI_VERSION="$(grep '^yffi:' "${PROJECT_DIR}/include/VERSION" | awk '{print $2}')"
if [ -z "${YFFI_VERSION}" ]; then
  echo "❌ Could not read yffi version from include/VERSION"
  exit 1
fi

echo "==> Building YrsFFI.xcframework from yffi v${YFFI_VERSION}"

# --- Step 1: fetch yffi source if not already present -----------------------

if [ ! -f "${SOURCE_DIR}/Cargo.toml" ]; then
  echo "==> Downloading yffi v${YFFI_VERSION} from crates.io"
  rm -rf "${SOURCE_DIR}"
  mkdir -p "${BUILD_DIR}"
  curl -fsSL \
    "https://crates.io/api/v1/crates/yffi/${YFFI_VERSION}/download" \
    -o "${BUILD_DIR}/yffi-${YFFI_VERSION}.tar.gz"
  tar xzf "${BUILD_DIR}/yffi-${YFFI_VERSION}.tar.gz" -C "${PROJECT_DIR}"
  mv "${PROJECT_DIR}/yffi-${YFFI_VERSION}" "${SOURCE_DIR}"
  rm -f "${BUILD_DIR}/yffi-${YFFI_VERSION}.tar.gz"
else
  echo "==> Reusing existing yffi source at ${SOURCE_DIR}"
fi

# --- Step 2: build all 5 Apple targets --------------------------------------

mkdir -p "${BUILD_DIR}"

echo "==> Building yffi for all 5 Apple targets (release profile)"
for TARGET in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios \
              aarch64-apple-darwin x86_64-apple-darwin; do
  echo "  -> ${TARGET}"
  cargo build \
    --manifest-path "${SOURCE_DIR}/Cargo.toml" \
    --target "${TARGET}" \
    --release \
    --locked
done

# --- Step 3: lipo simulator and macOS slices --------------------------------

echo "==> Combining iOS simulator slices (arm64 + x86_64) with lipo"
mkdir -p "${BUILD_DIR}/ios-sim"
lipo -create \
  "${SOURCE_DIR}/target/aarch64-apple-ios-sim/release/${LIB_NAME}" \
  "${SOURCE_DIR}/target/x86_64-apple-ios/release/${LIB_NAME}" \
  -output "${BUILD_DIR}/ios-sim/${LIB_NAME}"

echo "==> Combining macOS slices (arm64 + x86_64) with lipo"
mkdir -p "${BUILD_DIR}/macos"
lipo -create \
  "${SOURCE_DIR}/target/aarch64-apple-darwin/release/${LIB_NAME}" \
  "${SOURCE_DIR}/target/x86_64-apple-darwin/release/${LIB_NAME}" \
  -output "${BUILD_DIR}/macos/${LIB_NAME}"

echo "==> Verifying universal binaries"
lipo -info "${BUILD_DIR}/ios-sim/${LIB_NAME}"
lipo -info "${BUILD_DIR}/macos/${LIB_NAME}"

# --- Step 4: assemble XCFramework into ./build/ scratch ---------------------

echo "==> Creating XCFramework (intermediate)"
rm -rf "${BUILD_DIR}/${FRAMEWORK_NAME}.xcframework"
xcodebuild -create-xcframework \
  -library "${SOURCE_DIR}/target/aarch64-apple-ios/release/${LIB_NAME}" \
    -headers "${PROJECT_DIR}/include" \
  -library "${BUILD_DIR}/ios-sim/${LIB_NAME}" \
    -headers "${PROJECT_DIR}/include" \
  -library "${BUILD_DIR}/macos/${LIB_NAME}" \
    -headers "${PROJECT_DIR}/include" \
  -output "${BUILD_DIR}/${FRAMEWORK_NAME}.xcframework"

# --- Step 5: replace committed xcframework in place -------------------------

echo "==> Replacing ./${FRAMEWORK_NAME}.xcframework with newly-built copy"
rm -rf "${FINAL_FRAMEWORK}"
cp -R "${BUILD_DIR}/${FRAMEWORK_NAME}.xcframework" "${FINAL_FRAMEWORK}"

echo ""
echo "✅ Built and installed: ${FINAL_FRAMEWORK}"
echo "✅ Run 'swift test' to verify, then commit ${FRAMEWORK_NAME}.xcframework/"
