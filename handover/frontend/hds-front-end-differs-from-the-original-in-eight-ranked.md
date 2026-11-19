---
status: ready-to-wire
titles: [hd]
updated: 2026-11-19
summary: "HD front end: Ship Select camera, settle and hull lighting wired; craft turn and zoom, a step pair open; Omega pose wired (zoom fitted)"
difficulty: medium (30)
---

# HD's front end differs from the original in eight ranked ways; Ship Select's craft is closed

2026-10-08, measured against RPCS3 at HD's resolution, matched pairs of every
page to a race. The ranked list, evidence and sheets are in
`docs/ui/campaign-screens.md`, "Wipeout HD/Fury: what still differs from the
original, ranked". Closed in this lane: the 3-D craft on Ship Select (race
hull, `FrontEnd::ship_preview_hull`, pose chosen, not measured).

## Open

- Ship Select variants (LANDED 2026-10-09, `hd-ship-variants`): the preview
  follows the selected variant's hull directory and a Fury variant's added
  rating is drawn in `HD_Blue` red (`docs/ui/campaign-screens.md`). Open: the
  `hull_orbit` pose is fitted to the classic Feisar hull only; no `_n1` frame;
  a variant rated below the base draws no marker (no frame shows one)
- Ship Select swap (LANDED 2026-10-10, `hd-ship-select-anim`): no turntable
  (measured), craft absent 0.9 s on opening, 60 ms gap and 0.45 s settle on a
  step (`oag_game::preview::hull_swap`; the settle's start per team LANDED
  2026-11-12, item 2 below). Open: the step's burst (not a `.pob`: it is the Fury backdrop
  point cloud, `BackgroundAnimFury_Item`). Backdrop (LANDED 2026-10-10,
  `fury-backdrop-ship-select`): Ship Select and Track Select draw it under the
  `default` tint, applied in linear light (`fs_composite` lifts a dim tint); the
  `OnEnable` pulse fires on screen changes, not team steps. Open: the step's
  thickening has no located mechanism (not `OnEnable`); the pulse's amplitude
  and a screen-change signal into `oag_ui::backdrop::Fury`; ours still reads
  fainter and browner than the original at matched screens (the cloud path
  differs, and this build's music pulse sits at 0.23 where RPCS3's reaches 1.2);
  `--team Feisar` does not preselect the Ship Select picker, so captures show
  the default team instead (a CLI bug, found 2026-10-10). **The pairs hide the backdrop on both sides since 2026-11-19
  (`hd-fury-backdrop`)**: no grab of it can be reproduced (a new cloud and path every boot, two paths in three in modes
  ours does not draw, music-driven, RPCS3's index wrap), so `rpcs3-hd-menu-pair.py` zeroes the row's tint for the grab
  and ours takes `--fury-tint 0`; `hd/menu-track-select` 7.83 to 5.84, `hd/menu-ship-select` 4.51 to 3.05
  (`docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`, "Hidden for a compare shot"). So the "fainter and browner"
  item above is no longer scored by any pair and stays open as a live-play item. **Landed since (`hd-fury-backdrop-2`)**: ours takes the
  adapted luma the pair records (`original.backdrop.adapted_luma`) through `--fury-adapted-luma`, which sets the craft's
  exposure and pins the backdrop chain's adaptation, and Campaign Select, grid and cell are hidden too (`hd/menu-campaign-select`
  4.77 to 3.32, `hd/menu-cell-select` 3.00 to 2.81, `hd/menu-grid-select` 7.28 to 7.75: its whole-pair number rose, the card
  animates and two grabs of the original differ by 15.9 over the card, the page region fell 1.50 to 0.07). Open: `WALK`'s
  Ship Select state depends on the save and on the cursor the walk left on an earlier run (`--taps-before ship-select=down`
  from the save's Fury hull; a second walk on the same boot needs `up` or none); the grid card's animation phase is not
  lined up between the two sides
- Ship Select framing (LANDED 2026-11-11, `hd-worst-pairs-2`): the screen's
  own camera, read off the executable, replaces the authored pose and the
  fitted lens, pitch and slide. The update writes rest `RotX 0.25`, `RotY -0.6`,
  fov 57.3, origin `(1220, 500)` over the widget every frame and fits the depth
  to the model's `<FE length>`; wired as `oag_title::ShipFit` with the widget's
  `ObeySafeZone` (`docs/ghidra/functions/ps3-hdfury-eu/team-selection-model.md`).
  Five teams' silhouettes are within 4 px of their RPCS3 rest frames; the
  pair's hull box (sigma 7) went 20.62 to 14.33, the frame 9.35 to 7.35. Open,
  in the order a player would notice:
  0. **The rest pair `hd/menu-ship-select` cannot score the settle start, the turn or the zoom** (checked 2026-11-19,
     `hd-ship-craft`): its original is grabbed 3 s after the last tap (settled), the turn, pan and zoom are zero at rest, and
     the hull silhouette is within 4 px on five teams, so what is left in its craft box (2.74 on an unblurred, 5 px blurred
     scale; the lighting item below) is colour. A claim about those three needs a **step pair**: grab N frames after a press through
     `grab_with_clock` with `st+0x1014` (depth target), `st+0x1020` (drawn depth) and `st+0x194` (the fit's model pointer) read from RAM
     per frame, which would also settle the drawn depth at open and whether the fit keeps the old box through the 4-frame gap.
     Not made.
  1. ~~**The hull is darker than the original** in that box~~ (LANDED 2026-11-12, `hd-ship-select-light-wire`):
     the craft is lit by the front end's own `.envsettings` (`oag_game::preview::model_light`, from
     `FrontEnd::model_lighting`; Fury style `fe.fury.EnvSettings`, HD style `fe.envsettings`, Omega its own
     `fe.EnvSettings`) at HD's `0.5` output times `oag_post::hd_bloom::exposure` at the front end's adapted luma,
     on the capture and the live picker. The live read of `FunkLayer + 0x25c` on Ship Select (compare profile) is
     **0.0163**, exposure 3.43, against the predicted 0.016; the hull box goes 14.34 to 4.67 (bias +2.1, +0.7,
     -1.7). Open: the left of the box and the darkest pixels still read brighter in the original (the parameter 8
     reflection, `Ambient false direction`, which `Light` has no field for, or the front end's bloom, which the
     preview draws after); the HD style's adapted luma is unread and taken at the clamp (exposure 1, from the law and a white page; no frame of it seen, it needs the plain HD disc); Omega's rig is its own
     file's with no tone stage and no frame to compare it with (`docs/ghidra/functions/ps3-hdfury-eu/
     front-end-model-lighting.md`). Small (20).
  2. ~~**The swap's settle is recovered and not played**~~ (LANDED 2026-11-12,
     `hd-ship-swap-settle`): `ship_model::FitMotion` on the picker stage starts
     the new craft at the last model's rest depth and eases it at 0.125 a frame
     (the slide at 0.0625); the chosen 1.06 and the measured curve are gone, and
     `--menu-picker-step-seconds` steps from the entry above (chosen). Open: the
     60 fps film holds its first size about six frames longer than the law and
     the team it stepped to is unidentified (`docs/ghidra/functions/ps3-hdfury-eu/
     team-selection-model.md`, "The swap against the 60 fps film"); Omega plays HD's law
     as a scale of its borrowed pose (`ship_preview_settle`, inherited; its own rates
     and pose are item 5). Found on the live screen
     (`--drive`): after a press the previous team's hull stays on screen until
     the new mesh's worker read lands (Assegai's hull under the MIRAGE label at
     tick 4), drawn at the new team's fit; the gap is not tied to the read.
  3. **The player's own turn, pan and zoom of the craft** (right stick, the
     idle return after 181 frames): read at confidence 65, not exercised, not
     wired; a new input on a screen needs pointer support in the same change.
     Medium (40).
  4. **The Detonator box** (`-2.0..0`) is Title data nobody uses: the preview
     does not draw the Detonator hull. Small (50).
  5. ~~**Omega** keeps its borrowed pose~~ (LANDED 2026-11-19, `hd-ship-craft`): the update's rest
     rotation, position, scale and lens are wired (`oag_title::ShipPose`), and the update's fit
     result is never read, so Omega has **no fit and no settle** (the "settle inherited from HD" reading is
     withdrawn, `ship_preview_settle` removed). `omega/menu-ship-select` re-made on the original's selection (Feisar `concept1`, locked profile): 26.13 to 20.03 whole
     frame, `SHIP MODEL` frame 32.7 to 13.2, craft 43.4 to 19.6; edges 1029, 1357, 351, 691 against 1030, 1357, 358, 685
     (`docs/ghidra/functions/ps4-omega-eu/team-selection-pose.md`). Open: **the 2.5 picture zoom** (`ShipPose::zoom`)
     is fitted to that one frame, the lens and scale as read draw the craft 2.5 times too small and no literal explains it
     (`FeModel_DrawAtOrigin`, `0x01618bb0`, searched, not read through); the original's Safe Area on that frame is unrecorded
     (a fresh shadPS4 grab at a known one would pin the zoom); the 7-pixel vertical excess; the old pair's team mismatch is fixed in the
     pair's record (`ours.race`, `unlock_everything = false`), the lead adopts the re-made pair
  `hull_orbit` remains only for Pulse-shaped fallbacks
- Ship Select: ~~bracket corners, loyalty value~~ (LANDED 2026-10-27,
  `hd-select-chrome`: corner marks, block arrows, `000`; loyalty value LANDED 2026-11-13, `hd-shipselect-loyalty`: raw total `%03d`, bar `total/100000`; open: the `middle`
  marks, `UnlockNobble` hearts (law read, threshold source unread, `team-selection-loyalty.md`), the page `Squares`); the hex column draws
  (2026-10-08, `picker::hd::hex`); **the selected variant's cursor ring LANDED 2026-11-19
  (`hd-ship-variant-outline`)**: a 6-unit pale band, box metric 0.0800 to 0.0459
  (`docs/ui/campaign-screens.md`, "The cursor ring"); open: HD's own HD style (white page)
  ring colour (Omega's red is measured, HD's is not), the band's real draw (executable not
  read); the `nitro` row is open where the original's
  fresh profile locks it (this build offers `_n1`); the
  original's default team is the maintainer's save (Feisar), not a gap
- Cell Selection (closed 2026-10-08, hd-cell-select: back card face-on, 32-hex
  field, four `_bw` icons, `corner2` brackets, next-grid logo, barcode, rule,
  capitals; 2026-11-18, `hd-menu-cell`: the card's level, the wordmark's glow and the bloom are the front end's
  bloom chain, the pair `hd/menu-cell-select` 5.06 to 3.00 mean grey, walked live with the pointer). Open, by
  share of the pair's remaining 3.00: the footer legend's spacing (0.63: ours spreads the prompts wider), the
  wordmark 6 pixels right of the original's (0.58 in its box), the black hexagon field in the scene (about 0.25:
  the flyer thread), `EVENT 01/08`'s letter spacing (0.11), the selected cell's white outline (0.05: ours draws a
  grey fill; a different widget from Ship Select's ring: `CellMode`'s `Bg_x_y`/`Outline_x_y` layers), the title rule's fade at its right end (the original's is solid). Also open: the base campaign's page
  (a first frame of `01_uplift` exists since 2026-11-18: values and the selected outline are cyan `0, 178, 231`,
  headings `82, 82, 82`, the card `233, 234, 233`; ours keeps `TitleColor`), the Y-joint and mark margins (one
  frame), a Tournament cell and the `NitroBattle` icon (no `_bw` file named for it)
- Frame chrome (closed 2026-10-08): the title's size and offset against the
  0.904 layout scale remain; the quad-buffer bug that dropped the rules on every
  flyer page is fixed (`Renderer::renew_quad_buffer`)
- Page transitions: none on Campaign Select/Grid/Cell (`--menu-anim-phase`
  draws identical pixels). The screens author per-widget `transition`/`delay`
  keys and `transitiontype="vwipe"` on the emblems, not curves
  (`docs/ui/campaign-screens.md`, "Still open")
- Main Menu and Racebox setup are our own tree, not HD's pages
- Layout scale: the original is drawn at about 0.904 of the authored space,
  offset (+150, -8); ours is 1:1. Not applied: may be the maintainer's
  `Safe Area Setting`. Re-measure on a fresh RPCS3 profile first
- Track Select (2026-10-27: corner marks and header arrows landed; open: the
  circuit thumbnail landed 2026-11-19, page `Squares`): hex grid LANDED 2026-10-08 (`hd-track-select`, shared
  `hex::draw_cells`; direction glyphs open; ring thickness LANDED 2026-11-19). Circuit model
  LANDED 2026-10-09 (`hd-track-model`: all twelve scenes from the executable's
  record, `cf_fetracks` ramp by `N.V`, `oag_game::preview::track_model`). Omega
  ported 2026-10-09 (`omega-track-model`, its own fresnel material).
  **Turntable and pose LANDED 2026-11-19 (`hd-track-select`)**: the screen's update
  (`TrackSelectionScreen_Update`, `0x0025d558`) takes each circuit's own `RotX`,
  `PosY` and `PosZ` from its `PI_Track` and turns one global angle 0.01 rad a frame
  (kept across visits; ours restarts the table at zero per visit: open); the lens is
  the constructor's 57.3 degrees, as ours was. Omega: checked, applies, not wired
  (its update is unread). The pair `hd/menu-track-select` is drawn at the RAM angle
  read at the grab. Open: nearZ/farZ (authored 1 and 1000, ours 10 and 2000), the
  multiplier `0x81db67ea`, the backdrop's brightness and cloud path (hidden on the pair since 2026-11-19, above), Zone circuits (`zone_1..4` carry a scene no record names),
  Omega's own table. **The fly-by LANDED 2026-11-19 (`hd-track-flyby`)**: `preview.bik` plays in the panel at (500,190),
  box difference 59.6 to 2.5, pair 5.84 to 4.65; open: the clip's frame is fitted on the pair, not read from the
  emulator's counter (the Bink widget's struct is unread), a first visit waits for the transcode (the prefetch does not
  warm the sixteen `preview.bik`), the Zone clips' sound, Omega ships no clip (its `TrackPreview` stills are unwired)
- Fury page content (2026-11-05, `fury-menu-content`, `docs/ui/campaign-screens.md`, "Fury page content against the original"):
  landed the Grid Selection counter, arrows, dotted unlock panel and its corner marks, and the Track Select record rows (fill
  alpha, `TextColor`). Open, each its own RE: ~~Ship Select hull pose convention~~ (closed 2026-11-11, above), page `Squares`,
  the record fill (38 against 26), the Grid footer spacing, the ship
  column's lock states; the card art and circuit model are loop phase
- Not reached: a Tournament cell, loading and EndRace pairs, Omega (no capture path)

## Next Steps

- The step pair above (RAM trace of `st+0x1014/0x1020/0x194` across a press), then the settle start and the swap's gap.

0. Fresh-profile RPCS3 boot at the default `Safe Area Setting`: does the 0.904
   scale survive? If yes, one HD-only presentation transform on `Space::HD`.
1. Compose `ShipModel`'s authored pose and compare against a Feisar frame on a
   fresh-profile RPCS3 boot.
2. Add a campaign-page tween measured off `trans-campsel-original.png`.
