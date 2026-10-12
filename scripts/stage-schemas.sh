#!/usr/bin/env bash
# Copies the schemas to where the Pages site serves them: <dist>/mods/schemas/v<N>/<kind>.tosd,
# the address oag_schema::BASE names. Used by build-web.sh, and by `just check-schemas`
# against a scratch folder so a layout mistake fails the gate rather than the deploy.
set -euo pipefail
dist=${1:?usage: stage-schemas.sh <dist>}
cd "$(dirname "$0")/.."
mkdir -p "$dist/mods/schemas"
cp -r schemas/v* "$dist/mods/schemas/"
