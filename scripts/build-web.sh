#!/usr/bin/env bash
# Builds the browser version into target/web/dist: the page in web/, the
# wasm-bindgen glue and the optimised module. See docs/tools/web.md.
#
#   scripts/build-web.sh            # the `dist` profile (fat LTO) + wasm-opt -O
#   scripts/build-web.sh --dev      # a debug build, no wasm-opt: quick to iterate
#   scripts/build-web.sh --serve    # ... then serve dist on 127.0.0.1:8000
#                                   # with the COOP/COEP headers threads need
#
# The module is threaded (docs/tools/web.md, "Threads"): built with a pinned
# nightly, `-Z build-std` and the atomics target feature, which the prebuilt
# std is not compiled with. Needs that toolchain with `rust-src` and the
# wasm32-unknown-unknown target, `wasm-bindgen` matching the crate version in
# Cargo.lock, and `wasm-opt` (binaryen) unless --dev. The rest of the
# workspace keeps the pinned stable toolchain.
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

# The nightly the threaded module is built with; pages.yml installs the same.
toolchain=${OAG_WEB_TOOLCHAIN:-nightly-2026-10-08}
# A target directory of its own: these flags rebuild std and every crate, and
# sharing `target/` would throw the stable build's artifacts away each time.
target_dir=${CARGO_TARGET_DIR:-target}/web-threads
out=target/web/dist
want=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/"/, "", $3); print $3; exit }' Cargo.lock)
have=$(wasm-bindgen --version 2>/dev/null | awk '{ print $2 }')
if [[ "$have" != "$want" ]]; then
  echo "wasm-bindgen $want is needed (found: ${have:-none}): cargo install wasm-bindgen-cli --version $want" >&2
  exit 1
fi

if ! rustup run "$toolchain" rustc --version >/dev/null 2>&1; then
  echo "the $toolchain toolchain is needed: rustup toolchain install $toolchain --component rust-src --target wasm32-unknown-unknown" >&2
  exit 1
fi
# Shared, imported memory up to wasm32's whole 4 GiB, and the TLS exports
# wasm-bindgen sets each thread up with. rustc does not pass these for the
# atomics feature alone (nightly-2026-10-08 linked an unshared memory).
link=""
for arg in --shared-memory --import-memory --max-memory=4294967296 \
  --export=__wasm_init_tls --export=__tls_size --export=__tls_align --export=__tls_base; do
  link+=" -C link-arg=$arg"
done
CARGO_TARGET_DIR=$target_dir \
  RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals $link" \
  cargo "+$toolchain" build -Z build-std=std,panic_abort \
  --target wasm32-unknown-unknown --profile "$profile" \
  -p oag-game --example oag_web --features web
dir=$profile
[[ $profile == dev ]] && dir=debug
wasm="$target_dir/wasm32-unknown-unknown/$dir/examples/oag_web.wasm"

rm -rf "$out"
mkdir -p "$out/pkg"
wasm-bindgen --target web --no-typescript --out-dir "$out/pkg" "$wasm"
# wasm-bindgen only emits thread support for a module on shared memory; a
# build that lost the atomics flags would boot and then fail on its first
# worker, so it fails here instead.
grep -q thread_stack_size "$out/pkg/oag_web.js" || { echo "the module has no shared memory: the threads flags did not apply" >&2; exit 1; }
# OAG_WEB_NO_OPT=1 skips wasm-opt, so the module keeps its function names for
# a browser CPU profile (`web-screenshot.py --profile`).
if [[ $profile != dev && -z ${OAG_WEB_NO_OPT:-} ]]; then
  # The `dist` profile's own goal (docs/tools/packaging.md), for the module:
  # wasm-bindgen's output is not size-optimised, and binaryen's -O pass takes a
  # few percent more off after LLVM's. Bulk memory and the other post-MVP
  # features rustc already emits are enabled so binaryen keeps them.
  wasm-opt -O --enable-threads --enable-bulk-memory --enable-nontrapping-float-to-int \
    --enable-sign-ext --enable-mutable-globals --enable-reference-types \
    --enable-multivalue \
    -o "$out/pkg/oag_web_bg.wasm" "$out/pkg/oag_web_bg.wasm"
fi
# The module and its glue go under a directory named for their content, so a
# host may cache them forever (`_headers`) and a deploy never pairs new glue
# with an old module: the page's one import is rewritten to name it.
hash=$(cat "$out/pkg/oag_web.js" "$out/pkg/oag_web_bg.wasm" | sha256sum | cut -c1-16)
mv "$out/pkg" "$out/$hash" && mkdir "$out/pkg" && mv "$out/$hash" "$out/pkg/$hash"
cp web/index.html web/style.css web/worker.js web/audio.js web/audio-worklet.js web/movie.js web/_headers "$out/"
sed "s#\"./pkg/oag_web.js\"#\"./pkg/$hash/oag_web.js\"#" web/main.js > "$out/main.js"
grep -q "./pkg/$hash/oag_web.js" "$out/main.js" || { echo "main.js import not rewritten" >&2; exit 1; }
# The same notices every other release artifact carries (release.yml).
cp LICENSE-MIT LICENSE-APACHE "$out/"
cp -r licences "$out/licences"
python3 scripts/check-leakage.py --dir "$out"
ls -la "$out" "$out/pkg/$hash"

if [[ -n $serve ]]; then
  exec python3 scripts/serve-web.py --dir "$out" --port "$port"
fi
