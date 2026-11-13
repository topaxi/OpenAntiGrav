---
status: ready-to-wire
titles: [omega, 2048]
updated: 2026-11-13
summary: "Pad focus, repeat, glide, pulse and ripple landed on the shared map; Omega's Grid Selection 2048 season chooser and the small leftovers are open"
size: M (40)
difficulty: medium (45)
---

# The 2048 map takes Omega's pad focus law

Landed in `oag_ui::frontend::map_focus` (law: `docs/ghidra/functions/ps4-omega-eu/campaign-map-pad.md`).
Maintainer decision: Omega filters by season, 2048 stays faithful (all three seasons, no chooser).

## Open

- **Omega's season chooser.** `Frontend::set_map_season` exists and is tested; nothing calls it.
  Wire `Grid Selection 2048` ahead of the hosted map: stop the grid reader skipping
  `grid_16`..`grid_18`, open it from the 2048 chooser entry, pass the season on confirm, Back
  returns to it. The flyer-to-season order is untraced (take list order, check one capture); `grid_16`..`grid_18` are `PI_Grid` stubs (`Campaign="2048"`, `FlyerName="flyer2048"`, no cells); the original keeps the season at `canvas+0x5f880` (0x800..0x802, written by `0x013db460`) and the remembered event in `DAT_01e06780`. Until then Omega's map is unfiltered, a labelled departure.
- Per-type footprints (7 x 4 etc.): needs the event type in `MapEvent`.
- Start focus (remembered event, last raced walked forward), Triangle records, Change craft.
- Unread: pulse base `DAT_01aeb688`, tile pitch for the ripple, `TouchScroll` easing, Cross launch path.

## Next Steps

1. Read the 3 stubs and the `Grid Selection 2048` screen through `GridSelection` (1 h).
2. Wire the season through `MapSeed::open` and test it (2 h).
