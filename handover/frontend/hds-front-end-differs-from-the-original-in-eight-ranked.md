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
  step (`oag_game::preview::hull_swap`). Open: the settle's start size is per
  team (chosen 1.06), the step's burst (not a `.pob`: it is the Fury backdrop
  point cloud, `BackgroundAnimFury_Item`; what a step does to it is unlocated)
- Ship Select framing (LANDED 2026-10-10, `hd-ship-framing`): the authored
  `ShipModel` pose (one for all teams) composed on raw hull coordinates
  (`oag_game::preview::ship_model`). Open: the lens (0.41 rad), pitch (0.22 vs
  authored 0.4) and slide are fitted, not authored - find the original's
  rotation convention to drop the pitch override; `hull_orbit` remains only
  for Omega's hull and Pulse-shaped fallbacks
- Ship Select: bracket corners, loyalty value; the hex column draws
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
- Track Select: hex grid LANDED 2026-10-08 (`hd-track-select`, shared
  `hex::draw_cells`; direction glyphs and ring thickness open). Circuit model
  LANDED 2026-10-09 (`hd-track-model`: all twelve scenes from the executable's
  record, `cf_fetracks` ramp by `N.V`, `oag_game::preview::track_model`). Omega
  ported 2026-10-09 (`omega-track-model`, its own fresnel material).
  Open: the turntable's rate, pitch and the field of view are chosen (an RPCS3
  capture of the same circuit over time would fix the rate), the multiplier
  `0x81db67ea`, Zone circuits (`zone_1..4` carry a scene no record names),
  Omega's own table; the moving fly-by picture (Bink)
- Not reached: a Tournament cell, loading and EndRace pairs, Omega (no capture path)

## Next Steps

0. Fresh-profile RPCS3 boot at the default `Safe Area Setting`: does the 0.904
   scale survive? If yes, one HD-only presentation transform on `Space::HD`.
1. Compose `ShipModel`'s authored pose and compare against a Feisar frame on a
   fresh-profile RPCS3 boot.
2. Add a campaign-page tween measured off `trans-campsel-original.png`.
