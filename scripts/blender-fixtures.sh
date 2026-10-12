#!/usr/bin/env bash
# `just blender-fixtures`: rebuild the Blender add-on's committed fixtures with a
# headless Blender and fail if they differ from the committed bytes.
#
#   blender_loop.*          a loop authored from scratch with the add-on
#   oval.reexport.*         the committed oval, imported by the add-on and exported again
#
# Needs Blender 4.2 or later on PATH (or $BLENDER). The gate does not run this (CI has
# no Blender); crates/trackfile/tests/blender.rs and trackfile_ground_truth.rs read the
# committed output. `--update` rewrites the fixtures instead of comparing.
# Nothing is installed: the add-on is registered from the repository, so the Blender
# user directory is untouched.
set -euo pipefail
cd "$(dirname "$0")/.."

blender=${BLENDER:-blender}
fixtures=crates/trackfile/tests/fixtures
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT

run() {
  "$blender" -b --factory-startup --python-exit-code 1 \
    --python scripts/blender/roundtrip.py -- "$@"
}
"$blender" --command extension validate scripts/blender/oag_track
run loop "$out"
run reexport "$fixtures/oval.track.toml" "$out"

status=0
for f in blender_loop.track.toml blender_loop.glb oval.reexport.track.toml oval.reexport.glb; do
  if [ "${1:-}" = "--update" ]; then
    cp "$out/$f" "$fixtures/$f"
  elif ! cmp -s "$out/$f" "$fixtures/$f"; then
    echo "stale: $fixtures/$f (run: just blender-fixtures --update)" >&2
    status=1
  fi
done
[ "$status" = 0 ] && echo "blender fixtures: up to date"
exit $status
