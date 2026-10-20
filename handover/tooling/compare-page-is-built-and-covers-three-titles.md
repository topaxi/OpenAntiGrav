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

- **Pulse PSP Talon's Junction White passes with `--pose-hold` (2026-10-20, `resting-yaw`)**: 0.000 units and 0.000
  degrees at 216.4 s, where the shipped pair was refused at 0.777 units (the resting turn's double count, below). The pair
  in `data/reference/compare-page/pulse-psp/talons-white` is still the refused one: `just compare-rerender` skips a
  `refused` pair, so re-make it with `psp-pulse-pair.py` and its recorded `argv` (run 2026-10-20 into a scratch copy only).
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
- **A resting craft turns in the original too (2026-10-20, `resting-yaw`); the correction compensates for posing at tick 0.**
  Ours unposed from its own grid, no input, lands within 0.101 degrees of the original's RAM heading on all 24 Pulse PSP
  pairs and 0.694 on all 13 HD pairs at the pair's clock; the live rates match (Pulse Amphiseum Black `avel_y` -0.04725
  against ours -0.0473; HD Talon's -0.00461 against -0.00454 degrees a tick). The pairs pose ours at the original's
  already-turned pose and simulate the clock again, so it turns twice, and each pair's heading correction equals ours'
  drift from GO in magnitude (Pulse to 0.033 degrees). The physics is unchanged; evidence and the table in
  [grid-state.md](../../docs/physics/grid-state.md#a-resting-craft-after-go-turns-in-the-original-too-2026-10-20-resting-yaw),
  reproducer `scripts/compare-resting-yaw.py`. **Fixed 2026-10-20:** the pair scripts pass `--pose-hold` (the craft back on the fed pose at the rendered tick, the
  clock still run) and the correction loops are gone; see [compare-page.md](../../docs/tools/compare-page.md) for the
  before and after. **Talon's Junction White's refused 0.78-unit position drift is the same double count**:
  unposed, ours is 0.198 units from the original there. HD keeps a 0.5 to 0.7 degree residual even where ours does not
  turn (Chenghou Project, Metropia, Ubermall): HD's heading at GO after `Restart Race`, unmeasured.
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
2. Re-make the Pulse PSP Talon's Junction White pair with `psp-pulse-pair.py` (it passes with `--pose-hold`; `compare-rerender` skips refused pairs), and re-make the rest so their `pair.json` records `pose_hold`.
3. HD's 0.5 to 0.7 degree heading residual on pairs where ours does not turn: read the original's heading at GO after `Restart Race`.
4. Make a Pure PSP grid state and locate its camera node; 2048 and Omega need a race reached first.
