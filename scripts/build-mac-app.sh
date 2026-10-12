#!/usr/bin/env bash
# Wraps a built `oag-game` into OpenAntiGrav.app and zips it. No game content.
#
#   scripts/build-mac-app.sh [--version V] [--binary PATH] [--out ZIP]
#
# Defaults: --binary target/dist/oag-game (`cargo build --profile dist -p
# oag-game` on the Mac), --version the workspace version, --out
# target/macos/OpenAntiGrav-<version>-macos-arm64.zip. `just mac-app` runs it.
#
# The steps that need macOS tooling (iconutil, codesign, ditto, plutil) are
# skipped when the tool is missing, so the bundle layout can be assembled and
# audited on Linux with any binary; only a run on a Mac gives a bundle that
# launches. docs/tools/macos.md is the reference.
#
# Signing: the bundle is always ad-hoc signed (`codesign -s -`), which Apple
# Silicon requires of arm64 code. A real Developer ID signature plus
# notarization is a separate, off-by-default hook in release.yml (rcodesign).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

version=""
binary="target/dist/oag-game"
out=""
while [ $# -gt 0 ]; do
    case "$1" in
        --version) version="$2"; shift 2 ;;
        --binary) binary="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done
[ -x "$binary" ] || { echo "no binary at $binary (cargo build --profile dist -p oag-game)" >&2; exit 1; }
cargo_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
[ -n "$version" ] || version="$cargo_version"
arch="$(uname -m)"
[ "$arch" = "x86_64" ] && arch_name=x86_64 || arch_name=arm64
[ -n "$out" ] || out="target/macos/OpenAntiGrav-${version}-macos-${arch_name}.zip"
case "$out" in /*) ;; *) out="$root/$out" ;; esac

work="$root/target/macos/work"
app="$work/OpenAntiGrav.app"
rm -rf "$work"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$(dirname "$out")"

install -m 755 "$binary" "$app/Contents/MacOS/oag-game"
cp LICENSE-MIT LICENSE-APACHE "$app/Contents/Resources/"
cp -r licences "$app/Contents/Resources/licences"

# CFBundleVersion and CFBundleShortVersionString must be dotted integers, so
# they carry the Cargo version; the full build name (nightly-<date>-<sha>) goes
# in CFBundleGetInfoString where Finder's Get Info shows it.
python3 -I - "$app/Contents/Info.plist" "$cargo_version" "$version" <<'PY'
import plistlib, sys
path, short, full = sys.argv[1:4]
plist = {
    "CFBundleName": "OpenAntiGrav",
    "CFBundleDisplayName": "OpenAntiGrav",
    "CFBundleIdentifier": "org.openantigrav.game",
    "CFBundleExecutable": "oag-game",
    "CFBundleIconFile": "oag-game",
    "CFBundlePackageType": "APPL",
    "CFBundleInfoDictionaryVersion": "6.0",
    "CFBundleShortVersionString": short,
    "CFBundleVersion": short,
    "CFBundleGetInfoString": f"OpenAntiGrav {full}",
    "LSMinimumSystemVersion": "11.0",
    "LSApplicationCategoryType": "public.app-category.games",
    "NSHighResolutionCapable": True,
    "NSHumanReadableCopyright": "MIT OR Apache-2.0. No game content is included.",
}
with open(path, "wb") as f:
    plistlib.dump(plist, f)
PY
if command -v plutil >/dev/null; then plutil -lint "$app/Contents/Info.plist"; fi

# The icon is the engine's own rasterizer (the same one the window icon uses),
# at the sizes an .icns wants. The binary runs natively on the Mac.
if command -v iconutil >/dev/null; then
    iconset="$work/oag-game.iconset"
    mkdir -p "$iconset"
    for size in 16 32 128 256 512; do
        "$binary" --write-icon "$iconset/icon_${size}x${size}.png" --icon-size "$size"
        "$binary" --write-icon "$iconset/icon_${size}x${size}@2x.png" --icon-size "$((size * 2))"
    done
    iconutil -c icns "$iconset" -o "$app/Contents/Resources/oag-game.icns"
else
    echo "iconutil not found: the bundle has no icon (expected off macOS)" >&2
fi

# Nothing extracted can ride along: the same audit every release artifact gets.
python3 scripts/check-leakage.py --dir "$app"

if command -v codesign >/dev/null; then
    codesign --force --deep -s - "$app"
    codesign --verify --deep --strict "$app"
fi

rm -f "$out"
if command -v ditto >/dev/null; then
    ditto -c -k --keepParent "$app" "$out"
else
    echo "ditto not found: zipping with zip (expected off macOS; do not ship this one)" >&2
    (cd "$work" && zip -qry "$out" OpenAntiGrav.app)
fi
echo "wrote $out"
