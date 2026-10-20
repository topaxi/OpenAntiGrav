# Our hull misses some wall hits, and HD's wall resolver is unread

2026-10-19, `contact-angular-term`. The angular half of Pulse's wall response is
confirmed against the original's own rotation
(`crates/trace/tests/wall_angular_pose_walk_ground_truth.rs`,
[contact-response.md](../../docs/ghidra/functions/psp-pulse-usa/contact-response.md#live-2026-10-19-the-angular-half-against-the-originals-own-rotation)).
Two things around it stay open.

## Open

- ~~**Contact generation misses a real wall hit**~~ and ~~**a second missed
  contact, airborne, on a rolled craft**~~: **resolved 2026-10-20**
  (`wall-contact-miss`). Neither is a hull contact. Both are Pulse's
  **collision cage**, an invisible tube along the AI track data resolved after
  `Body_StepWorld` (`Cage_ResolveCraft`, `0x08831aa4`); ported, and the race runs
  it for every craft. Our hull reproduces the original's own narrowphase
  contacts exactly at the original's pose (818/818, 1,466/1,466). See
  [collision-cage.md](../../docs/ghidra/functions/psp-pulse-usa/collision-cage.md#the-cage-is-a-contact-source-live-2026-10-20);
  what the cage leaves open is
  [the collision cage's leftovers](the-collision-cages-leftovers.md).
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
3. ~~The denominator on a rolled craft~~: Fort Gale's tick 1341 is one (a cage
   contact at `up.y 0.65`). It leans body-frame (share ratio `0.91` against
   `0.82`), confidence 55, not ported; see the cage leftovers thread.
4. Static fallback for HD: `Craft_IntegrateHull` (`0x000ef450`) past its
   `Collision_MarchSegment` calls, then `0x000f5e20` and `0x000f6870`.
