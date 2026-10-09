#!/usr/bin/env bash
# Builds the browser version into target/web/dist: the page in web/, the
# wasm-bindgen glue and the optimised module. See docs/tools/web.md.
#
#   scripts/build-web.sh            # the `dist` profile (fat LTO) + wasm-opt -O
#   scripts/build-web.sh --dev      # a debug build, no wasm-opt: quick to iterate
#   scripts/build-web.sh --serve    # ... then serve dist on 127.0.0.1:8000
#
# Needs the wasm32-unknown-unknown target, `wasm-bindgen` matching the crate
# version in Cargo.lock, and `wasm-opt` (binaryen) unless --dev.
set -euo pipefail

cd "$(dirname "$0")/.."
profile=dist
serve=
port=${OAG_WEB_PORT:-8000}
for arg in "$@"; do
  case $arg in
    --dev) profile=dev ;;
    --serve) serve=1 ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done

target_dir=${CARGO_TARGET_DIR:-target}
out=target/web/dist
want=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/"/, "", $3); print $3; exit }' Cargo.lock)
have=$(wasm-bindgen --version 2>/dev/null | awk '{ print $2 }')
if [[ "$have" != "$want" ]]; then
  echo "wasm-bindgen $want is needed (found: ${have:-none}): cargo install wasm-bindgen-cli --version $want" >&2
  exit 1
fi

cargo build --target wasm32-unknown-unknown --profile "$profile" \
  -p oag-game --example oag_web --features web
dir=$profile
[[ $profile == dev ]] && dir=debug
wasm="$target_dir/wasm32-unknown-unknown/$dir/examples/oag_web.wasm"

rm -rf "$out"
mkdir -p "$out/pkg"
wasm-bindgen --target web --no-typescript --out-dir "$out/pkg" "$wasm"
if [[ $profile != dev ]]; then
  # The `dist` profile's own goal (docs/tools/packaging.md), for the module:
  # wasm-bindgen's output is not size-optimised, and binaryen's -O pass takes a
  # few percent more off after LLVM's. Bulk memory and the other post-MVP
  # features rustc already emits are enabled so binaryen keeps them.
  wasm-opt -O --enable-bulk-memory --enable-nontrapping-float-to-int \
    --enable-sign-ext --enable-mutable-globals --enable-reference-types \
    --enable-multivalue \
    -o "$out/pkg/oag_web_bg.wasm" "$out/pkg/oag_web_bg.wasm"
fi
# The module and its glue go under a directory named for their content, so a
# host may cache them forever (`_headers`) and a deploy never pairs new glue
# with an old module: the page's one import is rewritten to name it.
hash=$(cat "$out/pkg/oag_web.js" "$out/pkg/oag_web_bg.wasm" | sha256sum | cut -c1-16)
mv "$out/pkg" "$out/$hash" && mkdir "$out/pkg" && mv "$out/$hash" "$out/pkg/$hash"
cp web/index.html web/style.css web/_headers "$out/"
sed "s#\"./pkg/oag_web.js\"#\"./pkg/$hash/oag_web.js\"#" web/main.js > "$out/main.js"
grep -q "./pkg/$hash/oag_web.js" "$out/main.js" || { echo "main.js import not rewritten" >&2; exit 1; }
# The same notices every other release artifact carries (release.yml).
cp LICENSE-MIT LICENSE-APACHE "$out/"
cp -r licences "$out/licences"
# GitHub Pages runs Jekyll over the artifact unless this file is there.
touch "$out/.nojekyll"
python3 scripts/check-leakage.py --dir "$out"
ls -la "$out" "$out/pkg/$hash"

if [[ -n $serve ]]; then
  echo "serving $out on http://127.0.0.1:$port/"
  exec python3 -m http.server --bind 127.0.0.1 --directory "$out" "$port"
fi
