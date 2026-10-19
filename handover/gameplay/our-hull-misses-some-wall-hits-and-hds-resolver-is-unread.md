# Our hull misses some wall hits, and HD's wall resolver is unread

2026-10-19, `contact-angular-term`. The angular half of Pulse's wall response is
confirmed against the original's own rotation
(`crates/trace/tests/wall_angular_pose_walk_ground_truth.rs`,
[contact-response.md](../../docs/ghidra/functions/psp-pulse-usa/contact-response.md#live-2026-10-19-the-angular-half-against-the-originals-own-rotation)).
Two things around it stay open.

## Open

- **Contact generation misses a real wall hit.** On
  `talons-junction-clean-lap.csv` the original hits a `16_Track` wall
  between ticks 1583 and 1584 (`|v|` 108.6 to 99.2, `dv` 2 degrees from the
  wall triangle's normal, rotation rate +0.77 rad/s). Our `step` from tick 1583
  makes no contact: no centre-to-sample segment crosses any triangle, even
  with the box scaled 1.33x. Five of the 135 significant angular ticks on the
  three captures disagree in direction; the one examined (time-trial lap
  410/411) is a one-tick onset shift. The other four (time-trial 2458, clean
  lap 1257, 1681, 2724) are unexamined. `wall_contact_ground_truth.rs` already records
  first contacts 3-5 ticks late. This is probably the same residual.
- **A second missed contact, airborne, on a rolled craft (2026-10-19,
  `airborne-capture`).** Fort Gale Black (`14_Track`) PHANTOM, the original's
  autopilot, capture 3 of
  [airborne-ground-truth.md](../../docs/physics/airborne-ground-truth.md): over
  the second jump's crest, between ticks 1341 and 1342 at about
  `(-296, 127, 691)`, the airborne craft (up.y `0.65`, no stun, rail blend 0)
  takes a one-tick `dv` of `(-21.4, -15.5, -16.0)`, 31 units/s, then two
  decaying corrections. One `oag_physics::step` from tick 1341 makes no contact
  (velocity residual `26` units/s, pitch `-35` rad/s²). This is where our
  autopilot flies off Fort Gale every lap: without the redirect it carries
  straight on and is rescued `Airborne`. Seen once: captures 1 and 2 end
  before it. It is also the rolled-craft wall contact step 3 below wants, once
  our hull finds it.
- **HD's wall resolver is not found.** It is not in `Collision_ProcessPairs`,
  not on the craft's OBB collision object and not in craft vtable slot `+0x40`.
  HD's hull queries are SPU ray jobs (`0x0005fa28`), so the response consumes
  their results later. See
  [ps3-hdfury-eu/physics.md](../../docs/ghidra/functions/ps3-hdfury-eu/physics.md#the-wall-response-is-not-in-the-pair-narrowphase-2026-10-19).
  HD, Omega and 2048 run Pulse's response, **inherited, not measured**.

## Next Steps

1. PPSSPP: replay the clean lap's inputs (`oag-trace drive`) and breakpoint
   `Collision_AddContact` (`0x08816864`) on tick 1583-1584. Read the sample
   point, the triangle and the collider it crossed. Compare with our ten
   sample points, which the scratch sweep `gates_at_the_clean_lap_redirect`
   prints (`OAG_SWEEP=1 OAG_GATE_TICK=1583`).
2. RPCS3: on a wall-hit frame, set a write breakpoint on the player body's
   velocity quad to name HD's response writer. Then read it against Pulse's
   law (restitution, friction, lever arm, `0.1` angular scale) and add a
   `Title` axis only where they differ.
3. The wall-angular test cannot tell the world-axis denominator from the
   body-frame one (both pass). A capture with a wall contact on a pitched or
   rolled craft would.
4. Static fallback for HD: `Craft_IntegrateHull` (`0x000ef450`) past its
   `Collision_MarchSegment` calls, then `0x000f5e20` and `0x000f6870`.
