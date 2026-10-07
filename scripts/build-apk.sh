#!/usr/bin/env bash
# Builds the Android APK: oag-game as a NativeActivity cdylib, no Java, no game
# content. Output: target/apk/OpenAntiGrav-<version>-android-arm64.apk
#
#   scripts/build-apk.sh [--version V] [--debug] [--arm64-only]
#
# The APK also carries an x86_64 library when the x86_64-linux-android rust target
# is installed (and `--arm64-only` is not given): a phone ignores it, and it is
# what lets the APK run in Waydroid on a development machine. See
# docs/tools/android.md, "Testing without a phone".
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
x86=auto
while [ $# -gt 0 ]; do
    case "$1" in
        --version) version="$2"; shift 2 ;;
        --debug) profile=debug; shift ;;
        --arm64-only) x86=no; shift ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done
if [ -z "$version" ]; then
    version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
fi
# What `oag-game --version` prints on desktop, so the installed app reports the
# same thing (`just deploy-android` reads it back off the device).
version_name="$version ($(git -C "$root" rev-parse --short=7 HEAD 2>/dev/null || echo unknown))"

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
ndk_targets=(-t arm64-v8a)
if [ "$x86" = auto ] && rustup target list --installed 2>/dev/null | grep -qx x86_64-linux-android; then
    x86=yes
    ndk_targets+=(-t x86_64)
    mkdir -p "$stage/lib/x86_64"
fi
cargo ndk "${ndk_targets[@]}" -P "$api" build -p oag-game --example oag_android --features android "${cargo_profile[@]}"
so="$root/target/aarch64-linux-android/$profile/examples/liboag_android.so"
"$llvm_strip" --strip-all -o "$stage/lib/arm64-v8a/liboag_android.so" "$so"
if [ "$x86" = yes ]; then
    "$llvm_strip" --strip-all -o "$stage/lib/x86_64/liboag_android.so" \
        "$root/target/x86_64-linux-android/$profile/examples/liboag_android.so"
fi

# The launcher icon: the same rasteriser the AppImage and the window icon use
# (`oag_game::icon`), at the five densities' launcher sizes.
res="$stage/res"
host_game="$root/target/debug/oag-game"
cargo build -q -p oag-game
for density in mdpi:48 hdpi:72 xhdpi:96 xxhdpi:144 xxxhdpi:192; do
    mkdir -p "$res/mipmap-${density%%:*}"
    "$host_game" --write-icon "$res/mipmap-${density%%:*}/ic_launcher.png" --icon-size "${density##*:}" >/dev/null
done

# versionCode: the build's UTC time as yydddHH (2628017 = 2026, day 280, 17h).
# Local builds, nightlies and tags share the phone, so the code has to rise with
# time across all three, or Android refuses the newer APK as a downgrade (an
# x.y.z code put every local build and nightly at the same number).
# $OAG_APK_VERSION_CODE overrides it.
code="${OAG_APK_VERSION_CODE:-$(date -u +%y%j%H | sed 's/^0*//')}"

cat > "$stage/AndroidManifest.xml" <<MANIFEST
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="org.openantigrav.game"
    android:versionCode="$code"
    android:versionName="$version_name">
    <uses-sdk android:minSdkVersion="$api" android:targetSdkVersion="34" />
    <uses-feature android:name="android.hardware.vulkan.version" android:version="0x400003" android:required="true" />
    <uses-feature android:name="android.hardware.touchscreen" android:required="false" />
    <uses-feature android:name="android.hardware.gamepad" android:required="false" />
    <application android:label="OpenAntiGrav" android:icon="@mipmap/ic_launcher" android:roundIcon="@mipmap/ic_launcher" android:hasCode="false" android:extractNativeLibs="true" android:enableOnBackInvokedCallback="false">
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
flat="$stage/flat"
mkdir -p "$flat"
"$build_tools/aapt2" compile --dir "$res" -o "$flat"
"$build_tools/aapt2" link -I "$android_jar" --manifest "$stage/AndroidManifest.xml" \
    --min-sdk-version "$api" --target-sdk-version 34 -o "$base" --auto-add-overlay "$flat"/*.flat
(cd "$stage" && zip -q -D "$base" lib/*/liboag_android.so)

aligned="$out/aligned.apk"
rm -f "$aligned"
"$build_tools/zipalign" -f -P 16 4 "$base" "$aligned"

key="${OAG_APK_KEYSTORE:-$HOME/.android/oag-debug.keystore}"
# A CI build passes a stored key and its password (repository secrets); a
# local one generates its own once, with the sideload password `android`.
pass="${OAG_APK_KEYSTORE_PASS:-android}"
if [ ! -f "$key" ]; then
    mkdir -p "$(dirname "$key")"
    keytool -genkeypair -keystore "$key" -storepass "$pass" -keypass "$pass" \
        -alias oag -keyalg RSA -keysize 2048 -validity 10000 \
        -dname "CN=OpenAntiGrav sideload,O=OpenAntiGrav" >/dev/null 2>&1
fi
apk="$out/OpenAntiGrav-$version-android-arm64.apk"
OAG_APK_PASS="$pass" "$build_tools/apksigner" sign --ks "$key" --ks-pass env:OAG_APK_PASS --key-pass env:OAG_APK_PASS \
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
