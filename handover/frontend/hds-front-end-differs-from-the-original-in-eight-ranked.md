# HD's front end differs from the original in eight ranked ways; Ship Select's craft is closed

2026-10-08, measured against RPCS3 at HD's resolution, matched pairs of every
page to a race. The ranked list, evidence and sheets are in
`docs/ui/campaign-screens.md`, "Wipeout HD/Fury: what still differs from the
original, ranked". Closed in this lane: the 3-D craft on Ship Select (race
hull, `FrontEnd::ship_preview_hull`, pose chosen, not measured).

## Open

- Ship Select: bracket corners, loyalty value; the hex column draws
  (2026-10-08, `picker::hd::hex`); the `nitro` row is open where the original's
  fresh profile locks it (this build offers `_n1`); compose the
  authored `ShipModel` `RotX`/`RotY` instead of the fitted `hull_orbit`; the
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
  `hex::draw_cells`; direction glyphs and ring thickness open). Shaded circuit
  model: the asset is located (`fe/track0N.rcsmodel` on DATA02, `zone_N` on DATA00,
  8 of 12 base circuits; fresnel-blend material unread; name table at
  `renderer.md` `+0x10`); the moving fly-by picture (Bink)
- Not reached: a Tournament cell, loading and EndRace pairs, Omega (no capture path)

## Next Steps

0. Fresh-profile RPCS3 boot at the default `Safe Area Setting`: does the 0.904
   scale survive? If yes, one HD-only presentation transform on `Space::HD`.
1. Compose `ShipModel`'s authored pose and compare against a Feisar frame on a
   fresh-profile RPCS3 boot.
2. Add a campaign-page tween measured off `trans-campsel-original.png`.
