---
status: ready-to-wire
titles: [hd]
updated: 2026-11-12
summary: "HD front end: Ship Select camera and settle wired; hull lighting read, wire with the tone scale open; settle start, craft turn and zoom, Omega pose"
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
  the default team instead (a CLI bug, found 2026-10-10)
- Ship Select framing (LANDED 2026-11-11, `hd-worst-pairs-2`): the screen's
  own camera, read off the executable, replaces the authored pose and the
  fitted lens, pitch and slide. The update writes rest `RotX 0.25`, `RotY -0.6`,
  fov 57.3, origin `(1220, 500)` over the widget every frame and fits the depth
  to the model's `<FE length>`; wired as `oag_title::ShipFit` with the widget's
  `ObeySafeZone` (`docs/ghidra/functions/ps3-hdfury-eu/team-selection-model.md`).
  Five teams' silhouettes are within 4 px of their RPCS3 rest frames; the
  pair's hull box (sigma 7) went 20.62 to 14.33, the frame 9.35 to 7.35. Open,
  in the order a player would notice:
  1. **The hull is darker than the original** in that box (mean bias R -7.6,
     G -11.7, B -11.1). READ 2026-11-12 (`hd-ship-select-lighting`), ready to
     wire with one value open (`docs/ghidra/functions/ps3-hdfury-eu/
     front-end-model-lighting.md`): the original lights the preview hull (the
     race `ship.vex` and its programs) with the race's light parameters, filled
     from `data/fe/fe.fury.EnvSettings` (Fury style; `fe.envsettings` HD style)
     by `FrontEnd_LoadEnvSettings` (`0x003aa0e8`, Ship Select calls it with 0 at
     `0x0025379c`). Ours draws `mesh.wesl`'s stand-in rig. Confidence 85.
     Change: the preview's scene block takes `Light::authored` built through
     raceplay's `envsettings_light` path from that file (not hand-copied),
     `with_output_scale(0.5)`, then the front end's tone scale: the menu frame
     through `oag_post::hd_bloom`'s resolve with the FE constants (boost 35,
     clamp 3, max 4). Open value: the FE's adapted luma. One frame fits a scale
     of about 3.45 in linear light (55), predicting `adapted` of about 0.016;
     read it at `FunkLayer + 0x25c` as `docs/rendering/hd-track-tone.md` did.
     Box today 14.33; rig with `/2` 14.32 (the linear ratio goes
     channel-neutral, 3.53/3.46/3.47); rig with `/2` times 3.45 gives 4.62
     (bias +2.1, +0.7, -1.7). Acceptance: near 4.6, each channel's bias within
     about 2. The FE's black fog is refused at both levels (6.48 with it).
     Also open: the left of the box and the darkest pixels still read brighter
     in the original (parameter 8 reflection, ambient false direction or bloom).
     Omega: runs the HD style, whose `fe.EnvSettings` differs from HD's
     (ambient 0.2 0.3 0.32): checked, differs. Medium (45).
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
  5. **Omega** keeps its borrowed pose and the fitted lens: its own update
     carries the same fit with other rest angles (`+0.4307`, `-0.4438`), a
     blended position and a scale, not followed (the page's "Omega" section;
     `FUN_015a3b80`). Its pair reads 27.11, a far larger craft than the
     original's. Medium (35); needs Omega's item layout read first.
  `hull_orbit` remains only for Pulse-shaped fallbacks
- Ship Select: ~~bracket corners, loyalty value~~ (LANDED 2026-10-27,
  `hd-select-chrome`: corner marks, block arrows, `000`; open: the `middle`
  marks, `UnlockNobble` hearts, the page `Squares`); the hex column draws
  (2026-10-08, `picker::hd::hex`); the `nitro` row is open where the original's
  fresh profile locks it (this build offers `_n1`); the
  original's default team is the maintainer's save (Feisar), not a gap
- Cell Selection (closed 2026-10-08, hd-cell-select: back card face-on, 32-hex
  field, four `_bw` icons, `corner2` brackets, next-grid logo, barcode, rule,
  capitals): the wordmark's glow and bloom, the card's `FURY_GAIN` mechanism
  (which materials carry the 2x, unread; the base campaign's cards and their
  backs have no measurement), the Y-joint and mark margins (one
  frame), a Tournament cell and the `NitroBattle` icon (no `_bw` file named for
  it), and the live pad/pointer walk (headless stills only)
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
  circuit thumbnail is the `FlyByMovie` Bink widget, page `Squares`): hex grid LANDED 2026-10-08 (`hd-track-select`, shared
  `hex::draw_cells`; direction glyphs and ring thickness open). Circuit model
  LANDED 2026-10-09 (`hd-track-model`: all twelve scenes from the executable's
  record, `cf_fetracks` ramp by `N.V`, `oag_game::preview::track_model`). Omega
  ported 2026-10-09 (`omega-track-model`, its own fresnel material).
  Open: the turntable's rate, pitch and the field of view are chosen (an RPCS3
  capture of the same circuit over time would fix the rate), the multiplier
  `0x81db67ea`, Zone circuits (`zone_1..4` carry a scene no record names),
  Omega's own table; the moving fly-by picture (Bink)
- Fury page content (2026-11-05, `fury-menu-content`, `docs/ui/campaign-screens.md`, "Fury page content against the original"):
  landed the Grid Selection counter, arrows, dotted unlock panel and its corner marks, and the Track Select record rows (fill
  alpha, `TextColor`). Open, each its own RE: ~~Ship Select hull pose convention~~ (closed 2026-11-11, above), page `Squares`,
  the Track Select thumbnail (Bink `FlyByMovie`), the record fill (38 against 26), the Grid footer spacing, the ship
  column's lock states; the card art and circuit model are loop phase
- Not reached: a Tournament cell, loading and EndRace pairs, Omega (no capture path)

## Next Steps

0. Fresh-profile RPCS3 boot at the default `Safe Area Setting`: does the 0.904
   scale survive? If yes, one HD-only presentation transform on `Space::HD`.
1. Compose `ShipModel`'s authored pose and compare against a Feisar frame on a
   fresh-profile RPCS3 boot.
2. Add a campaign-page tween measured off `trans-campsel-original.png`.
