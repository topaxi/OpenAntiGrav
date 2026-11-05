---
status: measure
titles: [hd]
updated: 2026-11-05
summary: "The menu's final composite reads half a texel off on both axes; a race runs the same present path but its quad was not read live"
---

# Does a race's final composite read half a texel off, as the menu's does

The menu's draw 25 carries uv `(0.00026, 0.000463)` at the top left, `0.5 / 1920` and `0.5 / 1080`, filter linear
(`docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`, "Three more readings"). Statically a race is the same quad: the resolve
on the menu is the race's program, and `FUN_003e3268` is the sole caller of the chain runner (`renderer.md`, "The exposure is
read"), so the likely answer is yes, confidence 60. Not live: on 2026-11-05 `walk_to_race` from a Fury Main Menu left every press
`(dropped)` on a server that had been poked and paused for an hour, and the lane stopped there.

`oag_post::hd_bloom`'s `fs_encode` reads the scene with no offset. If the quad is the same, each race pixel is four scene texels
averaged in linear light in the original and one here, which is half a texel of softness on every race pair.

## Open

1. Read the race's display draw live. Size S (55).

## Next Steps

1. Fresh `rpcs3-drive.py serve` (`OAG_COMPARE_PLATFORM=hd-fury`), `walk_to_race()`, `HOOK_RING_ONLY=1 HOOK_MIN=60
   scripts/hd-draw-dump.py`, then the last draw on the display buffer in `rsx-pass-events.py`: its immediate vertices' `TEXCOORD0`
   and unit 0's filter word.
2. If it matches, add the tap to `fs_encode` as a title law (`Title` data, ADR-0058), with the backdrop's own present pass moved
   onto it (`handover/frontend/fury-backdrop-pass-graph-is-measured-and-ours-differs.md`, Open 4). Re-render the HD pairs and quote
   the sharpness change.
