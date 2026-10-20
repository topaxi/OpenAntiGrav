# The compare page is built; HD and Pulse PSP give pairs, four titles are partial, publishing is unlocked

`just compare-page` builds a static page of original-vs-ours pairs (split slider,
onion skin, side by side, difference) from manifests under
`data/reference/compare-page/`, and `--serve` puts it on the LAN. Usage, the manifest
and per-title recipes with their status: [`docs/tools/compare-page.md`](../../docs/tools/compare-page.md).
Landed 2026-10-20 (`compare-page` lane). Each image carries a credit (game, platform, original or engine) and the page footer carries ADR-0074's rights-holder statement, read from `legal.md` and checked against the ADR by `just check-docs` (`compare-page-credits` lane, 2026-10-20); `--publish` builds strictly and refuses an entry with no `platform`.

## Open

**The fresh set of 2026-10-20 (`compare-parity`).** The old pairs, manifests and page are archived under
`data/archive/compare-2026-10-20/` and are no reference. Every pair now comes out of `scripts/compare-environment.toml`
and its checks ([`docs/tools/compare-page.md`](../../docs/tools/compare-page.md), "Environment parity"): 24 of 25 Pulse PSP
(480x272), 13 of 13 HD (1920x1080) and one Pulse PS2 (960x720, camera not matched), 37 pairs in
`data/reference/compare-page/{pulse-psp,hd,pulse-ps2}/`, manifest `fresh.toml` from `compare_env.py manifest`.
`just compare-rerender` re-makes them from the savestates after a renderer change. The overlap score
(`pair.json` `overlap`, `edge_ncc` 0.07 to 0.44 on PSP, 0.14 to 0.31 on HD) is on the page.

- **Pulse PSP Talon's Junction White is refused**: its state sits 216 s into the race and ours drifts 0.78 units from the
  original's body over that clock (the heading is corrected, the position is not). Needs a fresher state or a position correction.
- **PS2: the camera is not matched** (pair shipped with `--allow-own-camera`, edge correlation 0.01). A camera node whose
  position fits the PSP's follow geometry is in the state (a 4x4 at several EE addresses, e.g. `0x609bf0`, 11.64 units behind
  and 3.0 above the craft); its pitch sign is the PSP's opposite and the 4:3 fov is unmeasured, a fov sweep gives only an
  ambiguous best fit (48 degrees). Next: read the PS2 `Camera_UpdatePlayerView` counterpart and the projection setup.
- **PS2 is 960x720 (the maintainer's "720p" at the game's own 4:3)** because the state has the 16:9 option off; a widescreen state
  and 1280x720 is the alternative. PS2 clock address `0x70EE9C` was read on two states of one run only.
- **Pure (PSP), 2048 (Vita3K), Omega (shadPS4): not reachable** (unchanged: no grid state or camera node for Pure, Vita3K stops at
  an intro, shadPS4 reaches the main menu only). Their profile rows exist.
- **Speed class and mode are declared by the state's file name, not read from RAM** (PSP and HD), PS2's class is assumed Venom.
- **`just compare-rerender` was run once each on PSP, HD and PS2.** HD drifts 10 degrees in the ship's heading and 1.8 units in the
  camera between two captures of one restored state (the craft's heading is not stable in the first seconds of a restart), a
  warning in the output; the clock and pose are re-read each time.
- **A resting craft in ours turns** on HD (Talon's: 0.26 degrees a second, 26 degrees in 6,000 ticks) and on some Pulse grids
  (Amphiseum Black 1.8 degrees over 14 s), the original's does not. `rpcs3-hd-pair.py` and `psp-pulse-pair.py` correct the pose they feed
  to ours until the heading measured at the rendered tick is the original's; the cause in the simulation is not investigated.
- **Publishing**: `just publish-compare` is unlocked and has never been run (the maintainer runs it); a new manifest entry needs `platform`.

### Visible rendering differences on the fresh set (leads, nothing fixed here)

Read at 1:1 on the built page with the environment equal (size, clock, GO board, HUD, pose, camera). **Struck from the old
list** (they were pairing artifacts of the unmatched clock or the 960x544 and 1760x990 sizes): the dark GO board, "engine glow at
rest" on the PSP, the record panel and the HUD clock, the GO fade, HD lead A (ride height after the pose, replaced by the
turning lead above) and the HD brightness caveat (RPCS3 now has `Write Color Buffers` on in both configs).

PSP:
1. **Moa Therma Black**: the original's craft is not drawn (the camera sits at the pit wall and the original shows no hull), ours draws the wing tips; edge correlation 0.00.
2. **The lap glyph**: the original draws `1 ∫ 3` with the slash glyph, ours `1 ... 3` with the separator drawn smaller and lower.
3. **Rail and light streak**: the cyan rail strip on the left wall and the horizontal light streak are brighter and whiter in ours (Amphiseum Black, De Konstruct Black, Platinum Rush Black, Fort Gale Black).
4. Still open from before and re-seen only where noted: the lit slab at the top left (old lead 9) and Vertica's panel (old 5); not rechecked on all 24.

HD (13 pairs): ours is darker, with a flatter sky and ground and no glare (Modesto, Tech de Ra, Sol 2); the hull is a saturated orange
with blue wing tips where the original's is pearlescent; the GO board is dark with the text faint (Modesto, Ubermall) where the
original's GO is bright; the original's `RECORD` panel reads `TOTAL` in ours (the profile has no record); the position hexagon and the
thrust bar match; the original draws a ground ring under the craft that ours does not.

PS2: the original's hull fills about 1.35 times the width ours does (the camera is ours, above), its HUD is larger and brighter than ours, the original shows `pos 8/8`; `(UT EDIT)` is gone from this frame.

2. ~~**Engine glow at rest**~~ **The same artifact on the PSP, closed**: ours draws the purple glow at tick 300 on the PSP pairs. **PS2 still differs**: the original's flame is a narrow magenta plume of sprites below a white-hot nozzle, ours the PSP-style purple star. **Looked at 2026-10-20 (`hull-points-ps2`), not closed**: the original draws one narrow four-fin spike on the centre line (two passes, grey and `(155,0,124)`, fading) under a white-hot plate; ours plays the `.POB` (halo, plate) and hides the plume at rest. The spike has `shipboost.vex`'s shape and size but sits on the centre line, where that file's anchors are at `+-0.9`; forcing the plume on draws its two side flares. Not identified: [trail-ribbon.md](../../docs/rendering/trail-ribbon.md) "Pulse PS2 at rest". Next: try the `engine_flare` locator as the anchor, and read the draw's colour source.

## Next Steps

1. Recover the PS2 camera (orientation and fov) so the PS2 pair needs no `--allow-own-camera`.
2. A fresher Talon's Junction White state (or a position correction in `psp-pulse-pair.py`).
3. Investigate why a resting craft turns in ours (HD and some Pulse grids).
4. Make a Pure PSP grid state and locate its camera node; 2048 and Omega need a race reached first.
