---
status: re-first
titles: [hd]
updated: 2026-11-20
summary: "Fury flyer cards go through the front end's bloom chain; light binding, swing and widget pose (0x001d3f08) open"
---

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
- ~~**The bloom and glow** RPCS3 shows around every card.~~ ~~**`FURY_GAIN` 2.0**, mechanism unread.~~ LANDED
  2026-11-18 (`hd-menu-cell`): both are the front end's bloom chain. The original draws a card into the scene
  `FunkLayer` reads, under the block the screen loads (`fe.fury.EnvSettings` on every campaign page, read live), so
  its level is the adapted exposure plus the bloom (2.5 on `Cell Selection` of `09_blitzed`, 2.2 to 3.3 on Fury's
  grids, the floor of 1 on the base cards and `Campaign Selection`). Ours draws the card into the Fury backdrop's
  finished field before its chain (`flyer::Layer::Scene`, `render::SceneLayer`); `FURY_GAIN` is the stand-in where
  no chain runs. Evidence and the frames: `docs/ghidra/functions/ps3-hdfury-eu/front-end-model-lighting.md`, "The
  flyer card is drawn before the resolve". Open from it:
  - ~~**The black hexagon field is in the original's scene and not in ours**~~ CLOSED (maintainer, 2026-11-20:
    in HD these are cards, not a hex field to move). `bBg_x_y` is authored on `Cell Selection` only and
    `Grid Selection` has no hexagon (`hd-menu-grid`, 2026-11-19); both pairs are now under 5 grey
    (`menu-grid-select` 2.86, `menu-cell-select` 2.81). No lane.
  - **Ship Select's backdrop runs the track block.** The read there is the model screens' block too; the hull is
    not in the scene (its level is a constant exposure). Putting the hull into the scene layer would give it the
    chain's bloom and retire `FrontEndTone::adapted_fury`. Medium (40). Measured 2026-11-18 (`hd-menu-ship`,
    `front-end-model-lighting.md`, "Ship Select, per region"): the hull is 1.01 of the pair's 5.29 grey, 4 % dark
    (red 0.91), and mostly texture detail; the backdrop inside the box is 0.83 and is the unrecorded cloud pick. So
    this item can move about a third of the hull's share, not the pair.
  - **Ship Select's pair was drawn with every team open, and its honeycomb is not flat.** LANDED 2026-11-18
    (`hd-menu-ship`): the HD menu pair script now locks ours' profile (5.72 to 5.09) and the honeycomb fades with the
    column's distance, each cell shaded across (the widget's own ramp; `campaign-screens.md`, "The
    honeycomb fades"). Open from the same frame: the team card's fill, 31 in the original against ours' 50 (alpha about `0x50` against the authored `0x7f`).
    Small (60).
  - Track Select, Racebox and Options: which block their screens load is unread (the loader's other callers).
- **The card's clock** (2026-11-19, `hd-grid-card-phase`): `--menu-model-seconds` poses the Fury card at the original's clock for a pair, read from RAM
  with the game stopped (`scripts/hd_flyer_clock.py`, `ps3-hdfury-eu/flyer-item.md`). Open: the **live loop** (the card plays `[3, 6)` s for ever in the
  original, ours is static at 3.0; needs per-frame geometry rewrite, `clip_to_shape` changes the triangles), the **flash near 5.6 s** (hooked arrows
  for about 0.05 s, ours matches only at the exact second) and the five hidden glitch textures. Omega's pair is not re-shot with the flag. The lazy build, the flag and the `[3, 6)` wrap are pinned (`campaign-screens.md`, "The card's clock").
- **The back card** draws on `Cell Selection` face-on (`CELL_POSE`, rectangle
  measured, `BACK_STRETCH` chosen); its wordmark glow draws (the chain's bloom). The wordmark stands 6 pixels right
  of the original's and the stripes reach the card's right edge where the original leaves a margin (the pair
  `hd/menu-cell-select`).
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
