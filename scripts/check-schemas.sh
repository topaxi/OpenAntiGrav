#!/usr/bin/env bash
# `just check-schemas`: every shipped TOML validates against its schema, schema and
# loader agree on every example, and the schemas stage to the paths oag_schema::BASE names.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo nextest run --workspace --no-tests=fail \
  -E 'package(oag-schema) | binary(/schema/)' --status-level fail

dist=$(mktemp -d)
trap 'rm -rf "$dist"' EXIT
scripts/stage-schemas.sh "$dist"
base=$(grep -oE 'https://[^"]*/mods/schemas' crates/schema/src/lib.rs | head -1)
path=${base#https://*/}
for file in schemas/v*/*.tosd; do
  staged="$dist/$path/${file#schemas/}"
  [ -f "$staged" ] || { echo "not staged where $base serves it: $staged" >&2; exit 1; }
done
grep -q '^/mods/schemas/\*' web/_headers \
  || { echo "web/_headers has no rule for /mods/schemas/*" >&2; exit 1; }
grep -q 'stage-schemas.sh' scripts/build-web.sh \
  || { echo "scripts/build-web.sh does not stage the schemas into dist" >&2; exit 1; }
echo "schemas: staged under $path/ ($(ls schemas/v*/*.tosd | wc -l) files)"
