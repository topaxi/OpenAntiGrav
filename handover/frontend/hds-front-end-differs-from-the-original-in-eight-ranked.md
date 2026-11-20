---
status: ready-to-wire
titles: [hd]
updated: 2026-11-20
summary: "HD front end: Ship Select matches RAM, swap gap wired; Team Selection banner vwipe wired; craft input unwired"
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
  (measured), craft absent 0.9 s on opening, 50 ms (3 frame) gap and 0.45 s settle on a
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
  0. ~~**The rest pair cannot score the settle start, the turn or the zoom**~~ (MEASURED 2026-11-19, `hd-settle-step`): a step
     pair, the original's frame N game frames after the selection moved with N read from RAM (`scripts/rpcs3-hd-ship-step.py`,
     `scripts/hd_ship_step.py`; `docs/ghidra/functions/ps3-hdfury-eu/team-selection-model.md`, "The step against RAM"). Settle start,
     ease and fit order: **exact** (14 steps on two boots, depth to 1e-4, slide to 0.06; `ship_model::tests::a_step_follows_the_screens_own_frames_in_ram`);
     turn, pan and zoom: **zero** through every step; the fit's model pointer moves with the selection (the "fit keeps the old box
     through the gap" candidate is false); the craft box after the gap sits at the rest floor (3.85 against 3.82). **The one term that differs is the cut**: the original's craft is
     absent at game frames 3, 4 and 5 after the selection moved (once also 6), ours was 4 frames (0.06 s); ours absent
     where the original shows the new craft is craft box 20 to 30. **WIRED 2026-11-19** (`hd-step-gap`): `hull_swap::GAP_FRAMES` = 3 screen frames from the selection
     (the original's craft goes with the header dots: 13 of 14 empty grabs show the new dots, none shows the old craft with new dots), test
     `hull_swap::tests::a_step_cuts_the_craft_for_three_frames`; the frame-6 step pair's craft box 30.52 to 2.57 (picture lag 3; at the old lag of 4 ours is
     absent at the pair's 2 frames either way, so `PICTURE_LAG_FRAMES` is 3 now and the pairs are to be re-made with it, or re-rendered by flags with `--menu-picker-step-seconds (k-3)/60`). **The rest of the page's step MEASURED 2026-11-20** (`hd-step-banner`, `team-selection-model.md`, "The banner and the hex column on a step"): the hex column's half-row phase follows the team index (odd teams lower; **WIRED**, `HexGrid::phase_by_entry`, `Team Selection` only: rest hex box 13.08 to 4.12 over the twelve rest pairs, k19 22.09 to 3.99, k10 27.69 to 14.05, k08 27.62 to 22.63); the step's lattice slide (old phase to frame 3, new at 11), the displaced pulsing ring and the banner logo (old to frame 2, bare 3 to 7, a 12 px slab sweeping down 8 to 15 and up 25 to 29, whole at 45, ours whole from frame 0): the lattice slide and ring are open; **the banner is RECOVERED and READY TO WIRE (2026-11-20, `hd-banner-transition`, `docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`)**: `Image_Render` `vwipe` turns the logo widget's alpha `s` into a 0.1 H slab (row `2.25 s H` for `s` 0 to 0.4, `(0.9 - 2.25 (s - 0.4)) H` for 0.4 to 0.8) then a quad of height `(0.1 + 4.5 (s - 0.8)) H` growing from the top, `s` a linear ramp over 0.7 s (the class default: `enebletransition` matches no key), restarted with no delay on every team step; wire it as a function of `s` in `oag_ui`'s banner image, `s = 0` at the step frame plus 0 to 2 frames (depends on the quad height, see the page); the banner panel is 31 in the original and 50 in ours (the `HD_LightGrey` alpha item). Open: the cold first visit of a team (craft absent past frame 10 in two of two grabs), `FitMotion`'s
     alternation phase at the step (ours fixed, the original's the press's frame, 0.1 of depth at most). The craft box colour bias (-2.5, -1.5, -1.9) is item 1's.
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

- Ring and lattice slide on a step (`hd-step-banner`, opus RE lane, size large 60): `HexSelection`'s update (class name `0x0078b348`; ring pulse and displacement, the lattice slide over game frames 4 to 10); targets in `team-selection-model.md`: hex box on k04 to k10 (16.8, 21.5, 22.6, 14.1) down to the 4 floor, the panel fill 31 against 50. **The banner logo's `vwipe` is wired** (`hd-banner-wipe`: `oag_ui::screen::Vwipe`, replayed by `picker::hd::logo_draw` from `Picker::seconds_since_step`; banner box 29.66, 19.27, 18.76, 20.54, 17.15, 25.97, 15.41 on k04 to k29 now 16.07, 16.07, 16.07, 16.83, 16.22, 16.07, 16.02, the bare-panel floor, see `image-widget.md`, "Wired"). Open on it: the ramp starts at the press, the original's 0 to 2 frames later (sensitivity in `image-widget.md`). **The other pages' wipes are MEASURED and READY TO WIRE (2026-11-20, `hd-vwipe-pages`, `image-widget.md`, "Page entry")**: `Widget_ArmTransition` (`0x00165018`) sets each widget's countdown `+0xa0` to its authored `delay` (`+0x74`) the frame its page becomes present, `s` stays 0 (nothing drawn) until the countdown crosses 0, then ramps the in-time (0.5 s, the Team Selection Logo 0.7, Cell Selection's emblems 0.3). Read on Controls, Credits, Race Records, Racebox, Track Select, Team Selection, Grid Selection Fury and Cell Selection (2 to 4 entries each; four pages again on a second boot): start = `delay` game-seconds after entry, so wire `Screens::vwipe_from_node` to read `delay` as well and start every page's Images from the page-entry clock (Team Selection's Logo 0.5 s, Track Select's emblem 0.6, Race Records 0.6 to 1.2); Cell Selection's four emblems add a 0.37 s hold. Not reached: the network pages (44 widgets; the ONLINE tab needs a PSN session), Show Unlocks (flashes through after a race) and SoundTest. The ramp-start offset after a team step and the Track Select circuit emblem's restart (no delay, `Widget_ResetTransition`) are unchanged.
- The step pairs (`rpcs3-hd-ship-step.py pairs`, one per frame count, lag 3) can go on the compare page (k=6 scores the gap, k=8 and later the settle); re-render the seven recorded pairs at lag 3 first (`--menu-picker-step-seconds` is `(k-3)/60`).

0. Fresh-profile RPCS3 boot at the default `Safe Area Setting`: does the 0.904
   scale survive? If yes, one HD-only presentation transform on `Space::HD`.
1. Compose `ShipModel`'s authored pose and compare against a Feisar frame on a
   fresh-profile RPCS3 boot.
2. Add a campaign-page tween measured off `trans-campsel-original.png`.
