#!/usr/bin/env bash
# Builds the guest and host, runs every scenario on Cranelift and on Pulley, each
# twice in a separate process, and diffs everything but the timing lines.
# Run from this directory, under the shared build lock:
#   flock -o "$HOME/.cache/oag/gate.lock" ./run.sh
set -euo pipefail
out=${OUT:-target/runs}
mkdir -p "$out"
cargo build --release -p spike-guest --target wasm32-unknown-unknown
cargo build --release -p spike-host
guest=target/wasm32-unknown-unknown/release/spike_guest.wasm
for engine in cranelift pulley; do
  for run in 1 2; do
    target/release/spike-host "$engine" "$guest" > "$out/$engine-$run.txt"
  done
done
rc=0
for f in cranelift-2 pulley-1 pulley-2; do
  if diff <(grep -v '^time' "$out/cranelift-1.txt") <(grep -v '^time' "$out/$f.txt"); then
    echo "identical: cranelift-1 vs $f"
  else
    echo "DIFFERENT: cranelift-1 vs $f"; rc=1
  fi
done
cat "$out/cranelift-1.txt"
grep '^time' "$out/pulley-1.txt"
exit $rc
