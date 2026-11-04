---
status: parked
titles: [hd]
updated: 2026-11-04
summary: "Measured: the 2.37 is a reattach ripple over a dip the unboosted craft also crosses, not a ride-law error; pin stays, thread can be deleted"
---

# Plasma ride height under HD's launch boost

Since HD flies the launch boost it inherits from Pulse PSP (`docs/physics/launch-boost.md`),
`crates/raceplay/tests/projectile_tilt_ground_truth.rs` sets `start_boost = None`: the
ride law is judged on the line the reference shots were taken on, and the boosted launch
moves where the autopilot fires. With the boost on, the worst height error read 2.37
against the test's 2.0 tolerance. Nobody has looked at why.

## Open

- Nothing in the ride law. 2.37 is a reattach ripple over a dip (the same one the
  unboosted craft crosses, hidden by the settling window); evidence in
  `docs/gameplay/projectile-floor.md`, "HD Plasma, launch boost on". Optional: an HD
  capture over that stretch (x ~770, y -131, `01_Vineta_K`) to show the original rings
  the same way; not needed to keep the pin.

## Next Steps

1. Delete this thread when the maintainer accepts the verdict.
