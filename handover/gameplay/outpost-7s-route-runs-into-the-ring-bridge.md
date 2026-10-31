---
status: ready-to-wire
titles: [pulse]
updated: 2026-10-31
summary: "07's route line runs through the ring bridge's ramp foot and wedges; a route line of ours beside them is the candidate fix"
size: M (40)
difficulty: medium (40)
---

# Outpost 7's route runs into the ring's bridge

2026-10-31, lane `route-coverage`. Mechanism, numbers and the probe commands are in `docs/gameplay/ai.md`,
"Where each undriven route fails its verification". In short: Pulse `07_Track`'s route (path 1, a lower road)
has the ring's bridge structure standing on its authored line (racing-line offset 0). At the ramp foot (route
samples 196-208) the magstrip hold, fed the route's own lower-road sample, pulls the craft 3.89 units down
through the ramp (mismatch 3.97 under the 5.0 limit). Inside, `wall::pre_integration_clip` holds it at the
wedge at route 264 at full speed with no contact and no stall, until the verification's 25,728-tick cap. This
happens at every class, both ways. Past it, the far wedge (713-724) and the descending ramp's foot (756-791)
fail too.

## Open

- **Is the original's opponent on this road at all, and where across it?** The original locates an AI craft
  with its excluded path (path 0 here), so its hold reads the same lower-road section and would snap it the
  same way if it rides the ramp foot. Nobody has watched an original opponent take Outpost 7's lower road.
  A PPSSPP capture (Outpost 7, an opponent that drew path 1, its lateral offset through route 190-800) settles
  whether a line beside the structure is faithful or only ours. Not Ghidra.
- **The verification cannot see a craft parked at speed.** The stall test reads forward velocity and the
  progress test is off (`NEVER_STUCK`). A no-progress limit on route plans only (learning and
  verification) would end 07's parked runs early and change no verdict, since a parked run never laps.
  Do not extend it to ring plans: `01_Track` PHANTOM's note on `Run::best` says why.

## Next Steps

1. Build the candidate (ours, chosen): give the route line a lateral offset clear of the structure
   wherever a non-floor surface stands within the hull's reach of the line point. Map with
   `OAG_ROUTE_PROBE_FLOOR=2400..3030 OAG_ROUTE_PROBE_ACROSS=1` on `route_verify_probe` (the hull-reach map,
   offsets -30..+30). The clear lane is not one constant offset (isolated walls at +10, +14 and -8), so it
   has to follow the map rather than a fixed shift. Gate it on that condition so no other line moves.
2. Before building, run the same condition as a census over every line on every title (ring and routes).
   If only 07 and 07-rev hit, the prediction is every other plan bit-identical.
3. Measure: `route_verify_probe` on `pulse/07_Track,pulse/07_Track-rev` at four classes (target: lap
   `Some`, 0 failures), then the no-regression board (`route-plans-cost-a-whole-lap-each.md`, Next Steps
   1-3), plus `route_original_probe` for the far wedge at 715.
