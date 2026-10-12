#!/usr/bin/env bash
# Wraps a built iOS `oag-game` into OpenAntiGrav.app and an unsigned-for-the-store
# .ipa. No game content.
#
#   scripts/build-ios-app.sh [--version V] [--binary PATH] [--host-game PATH] [--out IPA]
#
# Defaults: --binary target/aarch64-apple-ios/dist/oag-game (`cargo build
# --profile dist --target aarch64-apple-ios -p oag-game`), --host-game
# target/debug/oag-game (a host build; an iOS binary cannot run here, so the
# icons come from the host one), --version the workspace version, --out
# target/ios/OpenAntiGrav-<version>-ios-arm64.ipa. `just ios-app` runs it.
#
# The .ipa is a zip of Payload/OpenAntiGrav.app, ad-hoc signed (`codesign -s -`)
# when codesign exists. An ad-hoc signature does not install on a stock iPhone:
# AltStore, SideStore or Xcode re-sign it with the player's own Apple ID, or a
# paid account signs it for TestFlight. docs/tools/ios.md is the reference.
# Steps that need Apple tooling (plutil, codesign) are skipped when it is
# missing, so the layout can be assembled and audited on Linux.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

version=""
binary="target/aarch64-apple-ios/dist/oag-game"
host_game="target/debug/oag-game"
out=""
while [ $# -gt 0 ]; do
    case "$1" in
        --version) version="$2"; shift 2 ;;
        --binary) binary="$2"; shift 2 ;;
        --host-game) host_game="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done
[ -f "$binary" ] || { echo "no binary at $binary (cargo build --profile dist --target aarch64-apple-ios -p oag-game)" >&2; exit 1; }
[ -x "$host_game" ] || { echo "no host oag-game at $host_game (cargo build -p oag-game), needed for the icons" >&2; exit 1; }
cargo_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
[ -n "$version" ] || version="$cargo_version"
[ -n "$out" ] || out="target/ios/OpenAntiGrav-${version}-ios-arm64.ipa"
case "$out" in /*) ;; *) out="$root/$out" ;; esac

work="$root/target/ios/work"
app="$work/Payload/OpenAntiGrav.app"
rm -rf "$work"
mkdir -p "$app" "$(dirname "$out")"

install -m 755 "$binary" "$app/oag-game"
cp LICENSE-MIT LICENSE-APACHE "$app/"
cp -r licences "$app/licences"

# Home screen icons: the engine's own rasteriser, the same one the window icon
# uses. 120 and 180 are the iPhone's 60pt at 2x and 3x, 152 and 167 the iPad's,
# 1024 the marketing size a store upload wants.
for size in 120 152 167 180 1024; do
    "$host_game" --write-icon "$app/AppIcon-$size.png" --icon-size "$size" >/dev/null
done

# CFBundleVersion and CFBundleShortVersionString must be dotted integers, so
# they carry the Cargo version; the full build name goes in OAGBuildName.
python3 -I - "$app/Info.plist" "$cargo_version" "$version" <<'PY'
import plistlib, sys
path, short, full = sys.argv[1:4]
icon_files = ["AppIcon-120", "AppIcon-152", "AppIcon-167", "AppIcon-180", "AppIcon-1024"]
plist = {
    "CFBundleName": "OpenAntiGrav",
    "CFBundleDisplayName": "OpenAntiGrav",
    "CFBundleIdentifier": "org.openantigrav.game",
    "CFBundleExecutable": "oag-game",
    "CFBundlePackageType": "APPL",
    "CFBundleInfoDictionaryVersion": "6.0",
    "CFBundleShortVersionString": short,
    "CFBundleVersion": short,
    "OAGBuildName": full,
    "CFBundleSupportedPlatforms": ["iPhoneOS"],
    "MinimumOSVersion": "16.0",
    "UIDeviceFamily": [1, 2],
    "UIRequiredDeviceCapabilities": ["arm64", "metal"],
    "CFBundleIcons": {"CFBundlePrimaryIcon": {"CFBundleIconFiles": icon_files}},
    "CFBundleIcons~ipad": {"CFBundlePrimaryIcon": {"CFBundleIconFiles": icon_files}},
    # An empty dict is what makes iOS run the app full screen at the device's
    # own size; without a launch screen it runs letterboxed in compatibility mode.
    "UILaunchScreen": {},
    "UIRequiresFullScreen": True,
    "UIStatusBarHidden": True,
    "UIViewControllerBasedStatusBarAppearance": False,
    "UISupportedInterfaceOrientations": [
        "UIInterfaceOrientationLandscapeLeft",
        "UIInterfaceOrientationLandscapeRight",
    ],
    "UISupportedInterfaceOrientations~ipad": [
        "UIInterfaceOrientationLandscapeLeft",
        "UIInterfaceOrientationLandscapeRight",
    ],
    # Documents/ shows in the Files app, which is how a player puts a disc
    # image into Documents/data/images (oag_mobile::ios::prepare).
    "UIFileSharingEnabled": True,
    "LSSupportsOpeningDocumentsInPlace": True,
    "LSApplicationCategoryType": "public.app-category.games",
    "NSHumanReadableCopyright": "MIT OR Apache-2.0. No game content is included.",
}
with open(path, "wb") as f:
    plistlib.dump(plist, f)
PY
if command -v plutil >/dev/null; then plutil -lint "$app/Info.plist"; fi

# Nothing extracted can ride along: the same audit every release artifact gets.
python3 -I scripts/check-leakage.py --dir "$app"

if command -v codesign >/dev/null; then
    codesign --force --deep -s - "$app"
    codesign --verify --deep --strict "$app"
else
    echo "codesign not found: the bundle is unsigned (expected off macOS)" >&2
fi

rm -f "$out"
(cd "$work" && zip -qry "$out" Payload)
echo "wrote $out"
