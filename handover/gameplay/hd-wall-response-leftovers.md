---
status: measure
titles: [hd, 2048]
updated: 2026-10-20
summary: "HD floor contacts unmeasured; 2048 holds the body-local point rate; RPCS3 Zone friction hit; wall pose-walk test"
---

# HD's wall response: what the port left open

2026-10-20, `hd-wall-resolver`. HD's wall resolver is found and measured live on RPCS3
(`Body_ResolveContact`, `0x000f7770`), and four of its laws are ported for wall contacts:
restitution from the disc's `<Collision shipWallCofR>`, a one-sided normal impulse, a
steer-away yaw torque and a body-local point rate. Omega and 2048 inherit them; 2048 holds
the point rate. Evidence and the sweep:
[wall-response.md](../../docs/ghidra/functions/ps3-hdfury-eu/wall-response.md).

## Open

- **2048 holds the body-local point rate.** On, `vita_2048_original_ai_ground_truth::
  the_field_finishes_tower` times out on lap 3 at 300 s (off: 292.3 s). The noise method
  puts it within the bar (off `1 0 1 1 0 0 0 0 0`, on `1 0 1 0 1 0 0 0 0`), but the test
  has no row to regenerate. It turns on once 2048's field is flown against it.
- **HD's floor contacts are unmeasured.** Every live hit was material `3` (wall). HD's
  pair table gives the craft's `(0, 2)` and `(0, 4)` pairs restitution `1.0`; nothing shows
  HD's floors reaching this resolver, so floors keep Pulse's law. 2026-10-20 (`hd-hover-residual`):
  on Ubermall PHANTOM's 447 autopilot rows and Talon's Junction's 718 grid and ramp rows not one
  step put a floor contact (or a wall contact) through our resolver, and the grounded residual
  closes without one. A floor contact needs a craft that bottoms out.
- **The two-body resolver is unread.** `Body_ResolveContactPair` (`0x000f7bc0`,
  confidence 60) and `shipShipCofR = 1.0` drive craft-craft contacts; ours keeps Pulse's.
- **Mode-masked friction.** `Collision_PairFriction` (`0x000be068`) returns `0.025`
  under game-mode mask `0x206040` (Zone and kin, not run live); ours always uses `0.035`.
  Re-read 2026-10-20 (`hd-zone-handling`): the craft-wall pair `(0 or 1, 3)` takes `*(table + 0x38)`
  under the mask with the net byte `*0x008a83fc` clear, `*(table + 0x2c)` otherwise, `table` at
  `0x008a83ec`; still not run live (a mode-6 capture reached no isolated wall hit).
- **Shield drain with weapons off.** `0.035 * |p|` per record is measured; Pulse's
  weapons-off halving is not checked on HD.
- **The wall laws' two extra wrecks were never explained.** HD `races_others` went from 2 wrecks
  to 4 with the four laws on (wall-response.md, the sweep table); the sweep does not say what
  wrecked them. The next merge (`hd-hover-residual`, HD summing its hover escapes) brought the
  total to 1, so the count no longer shows a regression, but the cause of the two was not found.
- **The yaw kick lands one step late by loop order**, not by a capture of the step after.
- **`0x000f56d0`** is the body contact-record push (its float is the friction), not a ray
  probe. The rename belongs to the crossfade lane's page, where it is
  `Physics_QueueRayProbe_q`.

## Next Steps

1. A wall pose-walk ground truth for HD, extending `hd_air_pose_walk.rs` to a wall hit
   from the Z0 capture (step and compare the post-hit velocity and rate).
2. RPCS3: a hit on a Zone run, breakpoint `Collision_PairFriction`'s return, to measure
   the `0.025` branch.
3. RPCS3: breakpoint `0x000f7bc0` on a craft-craft bump; read restitution and the split.
