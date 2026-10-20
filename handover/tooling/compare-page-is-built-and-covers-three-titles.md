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

HD scene light (**2026-10-20, `hd-scene-light`, part closed**): the whole-frame darkness on Modesto Heights and Tech de Ra was their tiny `Sun direction` (length 1e-4) read as an off switch, which dropped the whole authored rig; the engine normalises it and so does ours now (mean grey difference Modesto 42.1 to 26.3, Tech de Ra 54.5 to 40.9, the other eleven pairs byte-identical; [envsettings.md](../../docs/formats/envsettings.md)). **Still open on the scenery**, one class across Sebenco, Sol 2, Moa Therma, Tech de Ra and Modesto: the curved pit-wall barriers and some stands draw near-white in ours where the original's are dark and textured (sebenco's left arch, tech's left barrier), and a global sun scale does not move it (0.6x, 0.8x, 1.25x all worse or flat in total), so it is per-material (the sun mask or the lightmap on a `track_wall` family, not read). The Tech de Ra stands read peach-white where the original's are saturated green and yellow. Not looked at: the `Use Lens Flare` glare. Next: pick one such chunk and read its draw's sun, lightmap and mask constants in RPCS3. **Started 2026-10-20 (`hd-scenery-materials`, stopped early for a physics lane, nothing landed):** the white geometry on Sebenco is not `ds_solarwall_c`, and an albedo-only render of ours draws it pure white, so the cause is the material's inputs (albedo/texture selection or vertex colour), not the light sum. Next: skip each opaque material slot in turn on Sebenco and diff against the original to name the chunk, then read that draw in RPCS3.
HD (13 pairs, before that fix): ours is darker, with a flatter sky and ground and no glare (Modesto, Tech de Ra, Sol 2); the hull is a saturated orange
with blue wing tips where the original's is pearlescent (**2026-10-20, `hd-hull-pearl`, part closed**: on the shadowed Talon's Harimau pair the grey was the hull's sun gate, the occlusion map written linear where the original sRGB-encodes it, fixed - the wing tops are the original's warm khaki again, six facets 8.5 to 4.1 mean level; it **overshoots** over the whole hull by about 10 levels and the open-air circuits (Sebenco Climb, Amphiseum) still read whiter on sun-facing facets, see `docs/rendering/hd-ship-materials.md`, "The hull's sun gate"); the GO board is dark with the text faint (Modesto, Ubermall) where the
original's GO is bright; the original's `RECORD` panel reads `TOTAL` in ours (the profile has no record); the position hexagon and the
thrust bar match; the original draws a ground ring under the craft that ours does not (**the ring is a translucent white bracket with a chevron pair, the same place in all 13 frames; looked at 2026-10-20, not found**: not the crowd atlas (`0x0a440d00`), not the 64x64 flare sprite, not `hovercam.gtf` (the camera pod's texture), not the Fury concept `ship_ghost`; it sits at the **same screen position, size and shape in all 13 frames** and is drawn **over the hull's wing tips** (the wings show through it, in the first frame of the sheet it crosses both), so it is a screen-space overlay of the HUD pass (target `0x394000`, atlas `0x3de5380` of 1024x1024 DXT5, 7 to 63 index draws) and not a scene decal; which HUD element and what draws it (Time Trial only? the first seconds only? it persists at 0 km/h for at least 30 s) is unread, so it stays unwired. Next: the HUD pass's draws in a frame capture, matched against the `hud_*.xml` layouts).

PS2: the original's hull fills about 1.35 times the width ours does (the camera is ours, above), its HUD is larger and brighter than ours, the original shows `pos 8/8`; `(UT EDIT)` is gone from this frame.

2. ~~**Engine glow at rest**~~ **The same artifact on the PSP, closed**: ours draws the purple glow at tick 300 on the PSP pairs. **PS2 still differs**: the original's flame is a narrow magenta plume of sprites below a white-hot nozzle, ours the PSP-style purple star. **Looked at 2026-10-20 (`hull-points-ps2`), not closed**: the original draws one narrow four-fin spike on the centre line (two passes, grey and `(155,0,124)`, fading) under a white-hot plate; ours plays the `.POB` (halo, plate) and hides the plume at rest. The spike has `shipboost.vex`'s shape and size but sits on the centre line, where that file's anchors are at `+-0.9`; forcing the plume on draws its two side flares. Not identified: [trail-ribbon.md](../../docs/rendering/trail-ribbon.md) "Pulse PS2 at rest". Next: try the `engine_flare` locator as the anchor, and read the draw's colour source.

## Next Steps

1. Recover the PS2 camera (orientation and fov) so the PS2 pair needs no `--allow-own-camera`.
2. Re-make the Pulse PSP Talon's Junction White pair with `psp-pulse-pair.py` (it passes with `--pose-hold`; `compare-rerender` skips refused pairs), and re-make the rest so their `pair.json` records `pose_hold`.
3. HD's 0.5 to 0.7 degree heading residual on pairs where ours does not turn: read the original's heading at GO after `Restart Race`.
4. Make a Pure PSP grid state and locate its camera node; 2048 and Omega need a race reached first.
5. Pulse PS2 in widescreen (maintainer, 2026-10-20, lower priority): the game has its own widescreen option, which may need switching on in the state (and the PCSX2 window or screen resized to match). Take the PS2 pairs with it on, at 1280x720 rather than the 960x720 4:3 profile, together with step 1's camera.
6. **HD default camera view (2026-10-20, `hd-camera-parity`, no code changed).** The HD pairs feed the original's camera to ours (`--camera-pose`, `--camera-fov`) and it lands on the same pixels; the camera is not the gap on those pairs. The original's eye on all 13 HD time-trial pairs sits at `(0.00, 3.00, -12.38)` in the ship frame at fov 60, which is `<ExternalCameraFar>` x 0.75 (Harimau's 4.0 and 16.5); `talons-assegai-c1` alone sits at `-11.61`. Ours with no override defaults to the Close view (`CameraView::default()`, `crates/display/src/display.rs`, a Pulse claim HD inherits), so a player's default HD view is closer than these pairs. Not measured: whether Far is HD's default on a fresh profile or the maintainer's own choice in the profile the states came from. Next: one fresh-profile RPCS3 boot or a static read of HD's view initialisation, then the default as a `Title` field with provenance. Also: on `hd-race/talons-pack` (boost pull-back camera, fov 70.54, fed to ours) the original draws the player hull lower and larger with the same camera and body position - a hull offset at speed, or body and camera read at different instants; unresolved. The pairs carry no camera check (ours' trace has no camera columns).
