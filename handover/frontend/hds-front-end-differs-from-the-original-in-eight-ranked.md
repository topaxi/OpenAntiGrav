# HD's front end differs from the original in eight ranked ways; Ship Select's craft is closed

2026-10-08, measured against RPCS3 at HD's resolution, matched pairs of every
page to a race. The ranked list, evidence and sheets are in
`docs/ui/campaign-screens.md`, "Wipeout HD/Fury: what still differs from the
original, ranked". Closed in this lane: the 3-D craft on Ship Select (race
hull, `FrontEnd::ship_preview_hull`, pose chosen, not measured).

## Open

- Ship Select: the team hex column, bracket corners, loyalty value; compose the
  authored `ShipModel` `RotX`/`RotY` instead of the fitted `hull_orbit`; the
  original's default team is the maintainer's save (Feisar), not a gap
- Cell Selection: the flyer card face-on, the hex field, event icons, the
  next-grid logo, upper-case values (needs the flyer pose thread)
- Frame chrome on the campaign pages: `SCREEN TITLE` caption, top rule,
  `NAVIGATION` footer entry
- Page transitions: none on Campaign Select/Grid/Cell (`--menu-anim-phase`
  draws identical pixels)
- Main Menu and Racebox setup are our own tree, not HD's pages
- Track Select: hex grid, shaded circuit model (no `FE` mesh on the disc), the
  moving fly-by picture
- Not reached: a Tournament cell, loading and EndRace pairs, Omega (no capture path)

## Next Steps

1. Compose `ShipModel`'s authored pose and compare against a Feisar frame on a
   fresh-profile RPCS3 boot.
2. Flyer face-on pose for Cell Selection (see the flyer thread).
3. Add a campaign-page tween measured off `trans-campsel-original.png`.
