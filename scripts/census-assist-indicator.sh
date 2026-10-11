#!/usr/bin/env bash
# Census: does any pre-HD disc author a Pilot Assist indicator?
#
# For each image and each WAD it carries: (1) is any entry's name hash one of the
# names HD, 2048 or a plausible port would use for the layout, the texture or the
# art, (2) does any decompressed entry mention "assist" (other than Assistant, the credits' word) in text. "pilot" is left out on purpose: it is the drivers and Autopilot, in every language. A WAD
# stores only name hashes, so (2) finds layout and string text, never a texture's
# own name; (1) is the check for names this project can guess.
#
#   scripts/census-assist-indicator.sh <scratch-dir> <image>...
set -euo pipefail
scratch=$1
shift
wad=target/debug/oag-wad
unpack=target/debug/oag-unpack
names=(
  'Data\XML\HUD_assist_indicator.xml' 'Data\HUD\Textures\hdHUD.mip'
  'Data\HUD\Textures\HUD_Components.mip' 'Data\HUD\Textures\Pilot_Assist.mip'
  'Data\HUD\Textures\AssistIndicator.mip' 'Data\HUD\PilotAssist.mip'
  'Data\HUD\assist.vex' 'Data\HUD\AssistIndicator.vex'
)
for image in "$@"; do
  base=$(basename "$image" .chd)
  for path in $("$unpack" list "$image" 2>&1 | awk '{print $3}' | grep -iE '(data|fe|wads2|wadsp)\.wad$'); do
    out="$scratch/$base/$(basename "$path")"
    mkdir -p "$out"
    "$wad" extract "$image:$path" -o "$out" >/dev/null 2>&1 || continue
    listing=$("$wad" list "$image:$path" 2>&1)
    hash_hits=0
    for n in "${names[@]}"; do
      h=$("$wad" hash "$n" | tail -1 | awk '{print $1}')
      if grep -qi "$h" <<<"$listing"; then hash_hits=$((hash_hits + 1)); fi
    done
    files=$(find "$out" -type f | wc -l)
    text_hits=$({ grep -rliaP 'assist(?!ant|enz)' "$out" || true; } | wc -l)
    echo "$base $(basename "$path"): $files entries, $hash_hits candidate-name hash hit(s), $text_hits file(s) mentioning "assist" (not Assistant, which the credits use)"
  done
done
