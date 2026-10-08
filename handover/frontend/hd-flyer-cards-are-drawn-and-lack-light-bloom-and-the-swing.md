# HD/Fury's flyer cards are drawn; light, bloom, the swing and the widget's own pose are open

All sixteen `Grid Selection` cards and `Campaign Selection`'s two draw, through
one path (`oag_game::flyer`): a flyer is composed for its own camera
(`oag_vex::camera`), flattened through it (`flyer::clip::flatten`), cut to the
placeholder `00_flyer.vex`'s outline and given its reflection (`flyer::shell`),
and stood at a pose. Evidence, the fits and every chosen-versus-measured line
are in `docs/ui/campaign-screens.md`, "The flyer behind `Grid Selection`" and
"The two cards"; the inline vertex layout in `docs/formats/rcsmodel.md`.

## Open

- **Fury's light.** `simpletexture*` compute `(constantAmbientColour +
  saturate(N.L) * directionalLight0Colour) * texture` (`0x81db67ea`,
  `0x02df31e5`, `0x2dba643d`). Nobody found the writer: the names sit in a
  table at `0x007b3658`, `0x007b36b8`, `0x007b36e0` referenced only by another
  table, and the front end loads no settings file. Cards draw at texture colour,
  so Fury's ship silhouette is flat white where RPCS3 shades its facets and its
  reds are duller. Try: watch the three parameters' patch slots in RPCS3.
- **The bloom and glow** RPCS3 shows around every card.
- **`FURY_GAIN` 2.0** (2026-10-08, hd-cell-select) doubles a Fury grid card's and
  its back's texels: measured (63 to 130, 197 to 255, Grid's dominant red 198 to
  255), mechanism unread. The base campaign's eight cards and the two `Campaign
  Selection` cards keep gain 1.0 (clamped flat at 2.0). Find which materials carry it (`basicnonalpha`'s
  fragment block multiplies by a patched `float1`, slot `0x2c`) and whether the
  campaign cards' materials differ.
- **The back card** draws on `Cell Selection` face-on (`CELL_POSE`, rectangle
  measured, `BACK_STRETCH` chosen); its wordmark glow is open with the rest of the
  bloom.
- **The five effect textures hidden by name** (`UNREAD_EFFECTS`: `loops`,
  `flashes`, `noise_bar`, `failscreen`, `crash_screen`): materials that take UV
  offset/scale or alpha from native-animated parameters. Drawn at rest they are
  solid discs and bars. Needs the parameter driver, and the idle loop
  (`[3, 6)`, node `LoopEnd` 4.0 on Fury) to animate at all.
- **The widget's own pose.** `Model_Item`'s update (`0x001d3f08`, AltiVec-heavy)
  builds the matrix from `x y z` (`+0xac..`), three rotations (`+0xb8..`) and the
  pivot (`+0xc4`). On `Campaign Selection` the authored numbers apply directly
  (`orthoScale` scales z, pivot and offsets; see the docs) but the selected
  card's `RotY 0` is chosen. On `Grid Selection` they do not: x=80, z=-200,
  RotY=1.5 read as a start pose; the settled pose (yaw -0.289, centre 22, 0,
  -111) is fitted. The transition between them is the "swing".
- **The window per card**: 0.346 base, 0.321 Fury and campaign (chosen; the
  camera's `+0x20` word tracks the first two and fails on the campaign cards).
  `CAMPAIGN_STRETCH` 1.048 is chosen. `+0x1c` reads as the aspect.
- **The card's body colour** under the picture (the dummy textures are a green
  debug swatch swapped at runtime), the flip between the two models (the back is drawn on `Cell Selection` face-on; the front-to-back swing is not; its camera
  is 97.15 away on `01_uplift`, not 92.5), tier-change animation.
- **The 26-vertex back face** of `cardShape` has no recoverable stride (stride 18
  fits); the inline layout is read on one file only - check the other 2,488
  inline chunks (`Mesh::inline_colours`).
- **`ShipModel` on HD's Team Selection** is unblocked (`preview::model` reads a
  PS3 pair, `draw_matrices` takes a camera), not wired. Omega's PS4
  `.rcsmodel` is the omega lane's.
- Camera nodes are on 121 of the disc's `.vex` (billboards, ships, tracks, the
  front-end scene), not only flyers; only flyers' are read.

## Next Steps

1. Find the light writer (RPCS3 patch-slot watch on `0x81db67ea`), then set
   `lit` per material in `Flyers::load` and bind an authored `Light`.
2. Decompile `0x001d3f08` in Ghidra `/hdfury/EBOOT-ps3-hdfury-eu.elf` for the
   pose composition; replace `GRID_POSE` with the settled state if it falls out.
3. Drive the effect materials' parameters and animate the card's `[3, 6)` loop
   instead of one frozen moment.
