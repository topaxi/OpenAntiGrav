---
status: measure
titles: [2048, omega]
updated: 2026-11-16
summary: "2048/Omega arena: the line jumps a gap onto a deck at ring 4520-4560; every craft follows the road's dive and drives the lower road instead"
size: M (35)
difficulty: medium (35)
---

# The field misses `arena`'s deck jump

2026-11-16, lane `arena-rescue`. Evidence is in `docs/gameplay/ai.md`, "The race's beneath
rescue asks the same". That lane stopped the beneath rescue from firing on the lower road.
This thread is what is left: why the field drives under the deck at all.

On 2048's and Omega's `arena`, the authored line leaves the road at ring sample 4520 and
crosses a gap (4520-4550, unsupported) onto a deck. The deck's floor is 4 units under the
line, from 4560 to 4790. The road itself dives at 4520 (normal 0.6 along up) to a lower road
25 units under the line, which ramps back up by 4810. Every craft, every class, follows the
dive. The race no longer rescues them there, and the field finishes. But no opponent ever
drives the line the artists authored.

## Open

- Whether the original's opponents (or a player at speed) take the deck. Not observed.
- Whether 4 unsupported samples are too few for `oag_ai::Line::is_takeoff` here, or the
  craft simply cannot leave a 0.6 dive at the speeds the plan drives.

## Next Steps

1. Watch the original on 2048 `arena` at sample ~4520 (Vita3K, player input held at
   full throttle, not Autopilot). Which level does a craft end up on? 1-2 hours.
2. If the deck is the road: read why the plan and driver do not take the takeoff there
   (`route_verify_probe`, `OAG_ROUTE_PROBE_RING=1`), opus. About 3 hours.
