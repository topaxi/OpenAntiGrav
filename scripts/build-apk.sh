#!/usr/bin/env bash
# Builds the Android APK: oag-game as a NativeActivity cdylib, no Java, no game
# content. Output: target/apk/OpenAntiGrav-<version>-android-arm64.apk
#
#   scripts/build-apk.sh [--version V] [--debug]
#
# Needs: the aarch64-linux-android rust target, cargo-ndk, and an Android SDK
# (ANDROID_HOME) with build-tools and one platform, plus an NDK (ANDROID_NDK_HOME,
# or the newest under $ANDROID_HOME/ndk). docs/tools/android.md has the install
# commands. The APK is signed with a sideload key, never committed: it is a
# sideload build, not a store build. The key lives outside the checkout
# ($OAG_APK_KEYSTORE, default ~/.android/oag-debug.keystore) so every checkout
# and worktree signs alike - Android refuses `adb install -r` over an APK
# signed with a different key, and a key under target/ died with its worktree.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

version=""
profile=release
while [ $# -gt 0 ]; do
    case "$1" in
        --version) version="$2"; shift 2 ;;
        --debug) profile=debug; shift ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done
if [ -z "$version" ]; then
    version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
fi

sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
[ -n "$sdk" ] || { echo "ANDROID_HOME is not set" >&2; exit 1; }
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
    ANDROID_NDK_HOME="$(find "$sdk/ndk" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort -V | tail -1)"
fi
[ -d "${ANDROID_NDK_HOME:-}" ] || { echo "no NDK: install with sdkmanager \"ndk;27.3.13750724\"" >&2; exit 1; }
export ANDROID_NDK_HOME

build_tools="$(find "$sdk/build-tools" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -1)"
platform="$(find "$sdk/platforms" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -1)"
android_jar="$platform/android.jar"
[ -f "$android_jar" ] || { echo "no android.jar under $platform" >&2; exit 1; }
llvm_strip="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip"

# API 26 is the floor: cpal's AAudio backend links libaaudio, which arrived there.
api=26
out="$root/target/apk"
stage="$out/stage"
rm -rf "$stage"
mkdir -p "$stage/lib/arm64-v8a"

cargo_profile=(--release)
[ "$profile" = release ] || cargo_profile=()
cargo ndk -t arm64-v8a -P "$api" build -p oag-game --example oag_android --features android "${cargo_profile[@]}"
so="$root/target/aarch64-linux-android/$profile/examples/liboag_android.so"
"$llvm_strip" --strip-all -o "$stage/lib/arm64-v8a/liboag_android.so" "$so"

# versionCode: 0.1.0 -> 100, so a later tag always installs over an earlier one.
IFS=. read -r major minor patch <<<"${version%%-*}"
code=$(( ${major:-0} * 1000000 + ${minor:-0} * 1000 + ${patch:-0} + 1 ))

cat > "$stage/AndroidManifest.xml" <<MANIFEST
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="org.openantigrav.game"
    android:versionCode="$code"
    android:versionName="$version">
    <uses-sdk android:minSdkVersion="$api" android:targetSdkVersion="34" />
    <uses-feature android:name="android.hardware.vulkan.version" android:version="0x400003" android:required="true" />
    <uses-feature android:name="android.hardware.touchscreen" android:required="false" />
    <uses-feature android:name="android.hardware.gamepad" android:required="false" />
    <application android:label="OpenAntiGrav" android:hasCode="false" android:extractNativeLibs="true">
        <activity android:name="android.app.NativeActivity"
            android:exported="true"
            android:label="OpenAntiGrav"
            android:launchMode="singleTask"
            android:screenOrientation="sensorLandscape"
            android:configChanges="orientation|keyboardHidden|keyboard|navigation|screenSize|smallestScreenSize|screenLayout|uiMode|density"
            android:theme="@android:style/Theme.NoTitleBar.Fullscreen">
            <meta-data android:name="android.app.lib_name" android:value="oag_android" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
MANIFEST

base="$out/base.apk"
"$build_tools/aapt2" link -I "$android_jar" --manifest "$stage/AndroidManifest.xml" \
    --min-sdk-version "$api" --target-sdk-version 34 -o "$base" --auto-add-overlay
(cd "$stage" && zip -q -D "$base" lib/arm64-v8a/liboag_android.so)

aligned="$out/aligned.apk"
rm -f "$aligned"
"$build_tools/zipalign" -f -P 16 4 "$base" "$aligned"

key="${OAG_APK_KEYSTORE:-$HOME/.android/oag-debug.keystore}"
if [ ! -f "$key" ]; then
    mkdir -p "$(dirname "$key")"
    keytool -genkeypair -keystore "$key" -storepass android -keypass android \
        -alias oag -keyalg RSA -keysize 2048 -validity 10000 \
        -dname "CN=OpenAntiGrav sideload,O=OpenAntiGrav" >/dev/null 2>&1
fi
apk="$out/OpenAntiGrav-$version-android-arm64.apk"
"$build_tools/apksigner" sign --ks "$key" --ks-pass pass:android --key-pass pass:android \
    --ks-key-alias oag --out "$apk" "$aligned"
"$build_tools/apksigner" verify "$apk"

# No game content may ride in the APK: the same extension audit the other
# release artifacts get, over the unpacked archive.
audit="$out/unpacked"
rm -rf "$audit"
mkdir -p "$audit"
unzip -q "$apk" -d "$audit"
python3 -I scripts/check-leakage.py --dir "$audit"
rm -rf "$audit" "$base" "$aligned" "$apk.idsig"

ls -l "$apk"
echo "$apk"
