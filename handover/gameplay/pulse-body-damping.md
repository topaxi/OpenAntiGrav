---
status: measure
titles: [pulse]
updated: 2026-11-12
summary: "Pulse flies its 0.01 body damping; left: Talon's reseeded crest a tick late, and the Autopilot port's 139.9 u/s at de Konstruct's ramp"
size: S (40)
difficulty: medium (40)
---

# Pulse body damping: the residuals after the flip

`Laws::PULSE` flies `BodyDamping::PULSE` (`docs/physics/body-damping.md`). De Konstruct Black's
first jump, which held the flip, was captured under player input: the original falls short on
the authored magstrip line up to 145 units/s, ours with the damping matches it
(`dekonstruct_takeoff_ground_truth`), and the Autopilot port's rescue there is the original's
physics (doc page, "De Konstruct Black's first jump, under player input").

## Open

- **The reseeded crest of `talons-junction-clean-lap.csv`** lifts off at tick 1596 against the
  original's 1595 (`hover_contact_ground_truth`, `RESEEDED_LIFTOFF_LAG`); undamped, tick 1595
  already read half a tick late. Per frame the damped approach is closer to the original.
- **The Autopilot's port arrives at de Konstruct's ramp at about 139.9 units/s and holds it up
  the ramp**, where held throttle settles at about 130 on the flat run-up (VENOM, Assegai). Not
  a physics question (the port flies the player's physics); whether the original's Autopilot
  drives the same thrust is not measured.

## Next Steps

1. The crest: `psp-trace.py --place` at the clean lap's 1590 pose and a pose walk of ticks
   1590-1596 with the damping on, probe heights per tick against ours; 2 hours.
2. The port's speed: log `oag_ai::original::Autopilot`'s thrust and the engine's output on the
   run-up against a held-throttle run from the same pose; if the thrust is above 100 %, trace it
   to the port's source page. 1 hour.
