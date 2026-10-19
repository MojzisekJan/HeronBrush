#!/usr/bin/env bash
# Builds a debug (or release) APK of Heron Brush for Android, without Gradle: cargo-ndk
# compiles crates/heron-brush-android for each ABI, then the SDK's build tools (aapt2,
# zipalign, apksigner) package it with android/AndroidManifest.xml, the launcher icons
# and the bundled data (assets/, gamedata/). See docs/android.md for the setup.
#
#   scripts/build-android.sh                 # debug APK, arm64-v8a + x86_64
#   scripts/build-android.sh --abi arm64-v8a # one ABI (a phone)
#   scripts/build-android.sh --abi x86_64    # one ABI (an x86_64 emulator image)
#   scripts/build-android.sh --release       # optimised (still signed with the debug key)
#   scripts/build-android.sh --install       # then adb install -r
#   scripts/build-android.sh --run           # then install and start it
#   scripts/build-android.sh --check         # cargo check only: needs no SDK or NDK
#   scripts/build-android.sh --bundle-only   # only stage the bundled data
#
# Needs: rustup targets aarch64-linux-android / x86_64-linux-android, cargo-ndk, an
# Android SDK (ANDROID_HOME) with build-tools and a platform (android-34 or newer), an NDK
# (ANDROID_NDK_HOME, or the newest under $ANDROID_HOME/ndk) and a JDK 17+ for apksigner.
# The APK lands in target/android/heron-brush-<profile>.apk.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PACKAGE="io.github.mojzisekjan.HeronBrush"
MIN_SDK=26
TARGET_SDK=34
PROFILE=debug
ABIS=()
INSTALL=0
RUN=0
CHECK=0
BUNDLE_ONLY=0

while [ $# -gt 0 ]; do
  case "$1" in
    --release) PROFILE=release ;;
    --abi) shift; ABIS+=("$1") ;;
    --install) INSTALL=1 ;;
    --run) INSTALL=1; RUN=1 ;;
    --check) CHECK=1 ;;
    --bundle-only) BUNDLE_ONLY=1 ;;
    -h|--help) sed -n '2,22p' "$0"; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
  shift
done
[ ${#ABIS[@]} -eq 0 ] && ABIS=(arm64-v8a x86_64)

rust_target() {
  case "$1" in
    arm64-v8a) echo aarch64-linux-android ;;
    x86_64) echo x86_64-linux-android ;;
    armeabi-v7a) echo armv7-linux-androideabi ;;
    x86) echo i686-linux-android ;;
    *) echo "unknown ABI $1" >&2; exit 2 ;;
  esac
}

OUT="$ROOT/target/android"
STAGE="$OUT/stage-$PROFILE"

if [ "$CHECK" = 1 ]; then
  for abi in "${ABIS[@]}"; do
    cargo check -p heron-brush-android --target "$(rust_target "$abi")"
  done
  exit 0
fi

# The data a desktop package ships beside the executable, copied out of the APK on the
# first run after each install (crates/heron-brush/src/android.rs, unpack_bundle). The
# index lists every file; its first line is a stamp that changes with every build.
stage_bundle() {
  local bundle="$STAGE/assets/heron-bundle"
  rm -rf "$STAGE/assets"
  mkdir -p "$bundle"
  rsync -a --exclude '*.py' --exclude '*.md' --exclude '__pycache__' assets/ "$bundle/assets/"
  mkdir -p "$bundle/games"
  rsync -a gamedata/ "$bundle/games/"
  {
    echo "heron-brush $(git describe --always --dirty 2>/dev/null || echo unknown) $(date -u +%Y%m%dT%H%M%SZ)"
    (cd "$bundle" && find assets games -type f | LC_ALL=C sort)
  } > "$bundle/index.txt"
  echo "bundled $(($(wc -l < "$bundle/index.txt") - 1)) files ($(du -sh "$bundle" | cut -f1)) in $bundle"
}

stage_bundle
[ "$BUNDLE_ONLY" = 1 ] && exit 0

# Where Android Studio and sdkmanager put the SDK by default: ~/Library/Android/sdk on
# macOS, ~/Android/Sdk on Linux.
if [ "$(uname -s)" = Darwin ]; then DEFAULT_SDK="$HOME/Library/Android/sdk"; else DEFAULT_SDK="$HOME/Android/Sdk"; fi
: "${ANDROID_HOME:=${ANDROID_SDK_ROOT:-$DEFAULT_SDK}}"
if [ ! -d "$ANDROID_HOME" ]; then
  echo "No Android SDK at $ANDROID_HOME: set ANDROID_HOME (docs/android.md)." >&2
  exit 1
fi
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  ANDROID_NDK_HOME="$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort -V | tail -1 || true)"
fi
if [ -z "$ANDROID_NDK_HOME" ] || [ ! -d "$ANDROID_NDK_HOME" ]; then
  echo "No NDK: install one with sdkmanager \"ndk;27.2.12479018\" or set ANDROID_NDK_HOME." >&2
  exit 1
fi
export ANDROID_HOME ANDROID_NDK_HOME
BUILD_TOOLS="$(ls -d "$ANDROID_HOME"/build-tools/* 2>/dev/null | sort -V | tail -1 || true)"
PLATFORM_JAR="$(ls -d "$ANDROID_HOME"/platforms/android-* 2>/dev/null | sort -V | tail -1 || true)/android.jar"
if [ -z "$BUILD_TOOLS" ] || [ ! -f "$PLATFORM_JAR" ]; then
  echo "The SDK needs build-tools and a platform: sdkmanager \"build-tools;35.0.0\" \"platforms;android-35\"." >&2
  exit 1
fi
command -v cargo-ndk >/dev/null || { echo "cargo-ndk is missing: cargo install cargo-ndk" >&2; exit 1; }
# A JDK that is not on PATH (a tarball under ~/.local/opt, say) is found through JAVA_HOME.
[ -n "${JAVA_HOME:-}" ] && [ -x "$JAVA_HOME/bin/java" ] && export PATH="$JAVA_HOME/bin:$PATH"
command -v java >/dev/null && java -version >/dev/null 2>&1 || { echo "apksigner needs a JDK 17+ (JAVA_HOME or java on PATH)." >&2; exit 1; }

# 1. The native library for each ABI, into the APK's lib/<abi>/.
rm -rf "$STAGE/lib"
NDK_ARGS=()
for abi in "${ABIS[@]}"; do
  rustup target add "$(rust_target "$abi")" >/dev/null
  NDK_ARGS+=(-t "$abi")
done
CARGO_PROFILE_ARGS=()
[ "$PROFILE" = release ] && CARGO_PROFILE_ARGS=(--release)
cargo ndk "${NDK_ARGS[@]}" --platform "$MIN_SDK" -o "$STAGE/lib" build -p heron-brush-android ${CARGO_PROFILE_ARGS[@]+"${CARGO_PROFILE_ARGS[@]}"}
# A debug build carries its debug info inside the library (hundreds of MB); the device
# does not need it.
if [ "$PROFILE" = debug ]; then
  STRIP="$(ls "$ANDROID_NDK_HOME"/toolchains/llvm/prebuilt/*/bin/llvm-strip | head -1)"
  find "$STAGE/lib" -name '*.so' -exec "$STRIP" --strip-debug {} \;
fi

# 2. Resources, manifest and assets.
VERSION_NAME="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
VERSION_CODE="$(git rev-list --count HEAD 2>/dev/null || echo 1)"
"$BUILD_TOOLS/aapt2" compile --dir android/res -o "$OUT/res-$PROFILE.zip"
UNSIGNED="$OUT/heron-brush-$PROFILE-unsigned.apk"
"$BUILD_TOOLS/aapt2" link -o "$UNSIGNED" \
  -I "$PLATFORM_JAR" \
  --manifest android/AndroidManifest.xml \
  --min-sdk-version "$MIN_SDK" --target-sdk-version "$TARGET_SDK" \
  --version-code "$VERSION_CODE" --version-name "$VERSION_NAME" \
  -A "$STAGE/assets" \
  "$OUT/res-$PROFILE.zip"

# 3. The libraries, then alignment and the signature.
(cd "$STAGE" && zip -qr "$UNSIGNED" lib)
ALIGNED="$OUT/heron-brush-$PROFILE-aligned.apk"
"$BUILD_TOOLS/zipalign" -f -p 4 "$UNSIGNED" "$ALIGNED"
KEYSTORE="${HERON_ANDROID_KEYSTORE:-$HOME/.android/debug.keystore}"
if [ ! -f "$KEYSTORE" ]; then
  mkdir -p "$(dirname "$KEYSTORE")"
  keytool -genkeypair -keystore "$KEYSTORE" -storepass android -keypass android -alias androiddebugkey \
    -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=Android Debug,O=Android,C=US"
fi
APK="$OUT/heron-brush-$PROFILE.apk"
"$BUILD_TOOLS/apksigner" sign --ks "$KEYSTORE" --ks-pass pass:android --key-pass pass:android --out "$APK" "$ALIGNED"
rm -f "$UNSIGNED" "$ALIGNED"
echo "built $APK ($(du -h "$APK" | cut -f1)) for ${ABIS[*]}"

ADB="$ANDROID_HOME/platform-tools/adb"
command -v adb >/dev/null && ADB=adb
if [ "$INSTALL" = 1 ]; then
  "$ADB" install -r "$APK"
fi
if [ "$RUN" = 1 ]; then
  "$ADB" shell am start -n "$PACKAGE/android.app.NativeActivity"
  echo "logs: $ADB logcat -s HeronBrush RustStdoutStderr wgpu_hal:* '*:E'"
fi
