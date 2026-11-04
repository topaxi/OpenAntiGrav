---
status: measure
titles: [hd]
updated: 2026-11-04
summary: "With HD's inherited launch boost on, the Plasma ride test's worst height error reads 2.37 against its 2.0 tolerance; the test now pins the boost off"
---

# Plasma ride height under HD's launch boost

Since HD flies the launch boost it inherits from Pulse PSP (`docs/physics/launch-boost.md`),
`crates/raceplay/tests/projectile_tilt_ground_truth.rs` sets `start_boost = None`: the
ride law is judged on the line the reference shots were taken on, and the boosted launch
moves where the autopilot fires. With the boost on, the worst height error read 2.37
against the test's 2.0 tolerance. Nobody has looked at why.

## Open

- Whether 2.37 is only a different firing point (a bank the shots never covered) or a
  real error in the Plasma's ride over a bank at the higher launch speed.

## Next Steps

1. Run the test with the boost on and log, per tick, where the bolt fires and where its
   worst error falls; compare against the same bank without the boost.
2. If the error tracks the bank and not the boost, the ride law over banks needs a
   capture on that stretch; if it tracks the boost alone, keep the pin and say why on
   the test.
