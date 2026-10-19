# HD's hull wash on boost is the original's; what is left on the hull is the Harimau livery and the `_n1` wing rims

2026-09-20, reported from play: "the light work looks amazing, but it
exaggerates the bloom". **Closed 2026-10-12 (`hd-hull-wash-2`, confidence 85):
the original washes the same panels at a boost.** Assegai gets a white housing
rim and Harimau a white inner channel. Our engine light, its hull binding and
its per-vertex law are the original's. Nothing was ported. Do not tune the
bloom and do not drop the craft binding to hide the wash.

The evidence is in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "The
original washes `Assegai` and `Harimau` at a real boost the way ours does":

- the flare node matches our locator on `Assegai`, `Assegai_c1` and `Harimau`;
- an injected light lights the same panels on both sides, with matched pixel counts;
- in a real boosted race frame on the original, holding the player's light at its
  rest colour removes the original's own wash.

The earlier "clean" reads (2026-10-02, 2026-10-07) were the same rim, read at
player size as nozzle glow. The earlier sections of that page carry the full
history: the hull binding, the `EdgeGeom` space, the `SVC1` combine and the
"no clamp" result.

The capture tool is `scripts/rpcs3-hd-hull-light.py` (`team`, `flare`,
`inject`, `boost`). The three Time Trial states it was run on are listed in
`docs/reverse-engineering/emulator-recipes.md`.

## Open

- **The pale, washed hull is the original's IBL variant, and ours lacks the environment map it reads
  (`hd-hull-reflection`, 2026-10-20, confidence 85 for the variant, 80 for what the maps are).** The livery is right
  (`harimau_livery.gtf` matched byte for byte on the live hull draws). The original draws the hull with
  `HalfBrightIBLSunShadowMapSpot0SVC1` (word `0x847`) where we resolve `HalfBrightAmbientSunSpot0SVC0` (`0x001`):
  its ambient is `4 * paraboloidIblTex[n]`, a 64x32 dual paraboloid of the surroundings, and a 128x64 sharper one
  (`paraboloidReflectionTex`) for the specular-from-alpha chunks. Both maps are rebuilt at runtime, in main memory,
  with no draw into them in the frames caught. Law, units, numbers, the matching recipe and the cross-title answers:
  [hd-ship-materials.md](../../docs/rendering/hd-ship-materials.md), "Finding 4". Nothing is wired: the variant
  switch and a probe have to land together (an IBL variant with no probe draws a black ambient), and a probe pasted
  from the RPCS3 dump would be a stand-in. The nozzle "jewel" is the `emissive_bloom` chunk drawn with a texel of
  `colours_flashing_glow.gtf` (ours draws that material and shows black; the `TC4` coordinate is unread).
- **The white wing rims on the `_n1` skins** are not the SPU term: they stay
  with the craft binding removed. Whether the original shows them is unread.

## Next Steps

1. **Environment probe (the big one, a renderer lane).** Build a dual-paraboloid 128x64 of our scene at the craft
   (cube or two hemispheres, every frame or every few: the original's maps match 83-87 % of their bytes after 0.4-0.8 s
   and 50 % after 3-7 s), a convolved 64x32 copy, bind them as `0x9edd3243`/`0x872ed3bd`, resolve the `IBL` words
   (`0x007`/`0x047`, `0x807`/`0x847` for the craft light) in `skin::variants` for ships, scale by 4. Find the producer
   first: break on a write into `0x4348b000` or draw-capture a frame that holds `liveRsxPanXForm_CubemapToDualParaboloide_fp`
   (`rsx-fp-names.py` names it) to read what the original puts in the cube. Compare at the rest state (original hull
   luma 0.60-0.74, saturation 0.21-0.31; ours 0.24-0.29 and 0.67-0.79 before any change).
2. **Nozzle jewel.** Read the vertex program the live `emissive_bloom` draws (521, 522 of the Harimau rest frame) use,
   find what `TC4` is, and compare with ours.
3. `_n1` rims: walk Team Selection to an `_n1` row, take a state as in
   emulator-recipes.md, and read whether the wing rims are white at rest in
   the original.
