---
status: ready-to-wire
titles: [pulse]
updated: 2026-11-11
summary: "Wire the collision cage into the projectile probe and sweep (code 4); then Cannon speed x2.7, own-craft box, track-up launch normal"
size: M (60)
difficulty: medium (60)
---

# Projectiles leave the track on a bank or a ramp; the original ends them at the collision cage

2026-11-11, lane `weapon-bank-follow` (a measurement, nothing wired, stopped by the maintainer
before a wire lane was drawn). Maintainer report: "on tilted track our weapons (plasma and others)
fall off the track along gravity, instead of following the track as in the originals." **True.**
Evidence, numbers, the law and the starting states for tests:
[rocket-visuals.md](../../docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md#2026-11-11-code-4-is-the-collision-cage-and-what-a-bank-does-to-every-flyer-weapon-bank-follow)
(read that section first), with the per-weapon parts on `plasma.md`, `missile.md`, `shuriken.md`,
`cannon-quake-leachbeam.md` and `collision-cage.md` (each has a 2026-11-11 section), and the player
summary on [projectile-floor.md](../../docs/gameplay/projectile-floor.md).

What is settled:

- The ride laws agree. From the original's own states (PPSSPP, Moa Therma White's 85 degree bank),
  one update of ours lands on the original's next position to 0.000 unit: Rocket 48 of 48, Cannon
  24 of 24, Shuriken 57 of 57; Plasma within 0.012 once it rides.
- The fault is a missing law: `Collision_SweepSegment`'s second query (`Cage_TestSweepEnd`,
  `0x08832134`) tests the segment's end against the collision cage and returns code `4`. Our own
  ported cage reproduces all 1,187 recorded calls (17 of them `4`, boundary points to 0.000).
  Confidence 94.
- Capture tool: `scripts/psp-projectile-sweeps.py` (usage in its docstring; needs a private
  PPSSPP from `scripts/ppsspp-start.sh` and a grid save state from `data/saves/pulse-psp/`).

## Open

Ranked by what a player sees. Items 1 and 2 are ready to wire; 3 to 6 are recovered laws with a
smaller effect; 7 is what was not measured.

1. **The cage test in the probe and the travel sweep (the report itself).** Not wired. Ours'
   Plasma and Shuriken fired at Moa Therma sample 1416 fly ten seconds and two to three kilometres
   off the circuit, two of three rockets fired up Outpost 7's ramp fly their whole five seconds, a
   Cannon round 440 units; the original's end at the cage at updates 49, 58, 17 to 22 and 25.
2. **The Cannon's muzzle speed is 2.7 times too low** (maintainer's "double check the cannon").
   `Cannon_Init` writes `velocity = row2 * (craft km/h + 500)` with the row 0.75 long and **no
   divide by 3.6**: 750.60 units/s measured at a craft speed of 139.111 units/s (stored speed
   1000.801, rows 0.7500); `cannon::launch` flies `(craft km/h + 500) / 3.6` = 278.0. Confidence 92,
   one shot at one speed.
3. **The first probes of a projectile miss on the firing craft's own box collider**, not on an age.
   The query answers `0x7f` when the mesh raycast's nearest hit is a craft box (kind `3`, no
   collider skipped). 28 live probe records. `plasma::UNPINNED_SECONDS` (0.04 s, chosen) stands in
   for it and is wrong on the mag strip (the original's bolt rides on its first update there); the
   Rocket has no stand-in at all (ours rides from the first update, the original's falls two to
   four). **How far the box reaches ahead of `body+0x30` is not understood**: the brackets by speed
   are on the linked section and do not fit one box (parked 2.13 to 2.84, 100 units/s 3.91 to 5.01,
   125 units/s 4.80 to 4.93, 140 units/s 4.0 to 4.3, mag strip at 139 units/s under 2.83).
4. **The launch normal is the track's up at the craft's located point** (`craft+0xb10` is the
   located track point's down), not the craft's up. Ours seeds `body.up()` in `spawn_riding`,
   `spawn_guided`, `throw` and the Plasma's charge. 0.04 degrees from the spline sample's `-down`
   on five captures; the craft's own up was 5.24 degrees off on Outpost 7's ramp. Confidence 88.
5. **The Shuriken's speed is not constant.** Both of its floor arms write
   `(hit + normal * ride - prev) / dt` with no rescale (333.6 at the throw, 268.3 to 337.7 on the
   bank, 373.2 on the kerb); ours keeps the length. Position per update is exact either way; in
   free flight ours is 29 units ahead after 52 ticks. Confidence 88.
6. **Floor arms that differ from ours, decompile-level (84):** the Missile's probe floor arm is the
   Rocket's steer toward the ride point (ours turns parallel; 0.05 to 0.5 unit a tick);
   `Rocket_Update`'s travel-sweep floor arm writes `(hit + 3 n - prev) / dt` raw (seen live: a
   rocket landing on Outpost 7's ramp face left it at `(-65.0, 155.6, -7.1)`; ours turns parallel,
   chosen); `Plasma_Update`'s travel-sweep floor arm only moves (ours adopts the normal and turns).
7. **Not measured or not read.**
   - The Missile's travel sweep on code `4` (`flags |= 0x14`, no bounce): decompile only, no capture
     produced it. Fire a Missile across a jump lip (Outpost 7 sample 186) to see it.
   - `CageEffect_Spawn` on a projectile's cage hit (every weapon passes `flags == 0`): read, not
     watched; no breakpoint on `0x08832b0c` during a capture.
   - A second Cannon speed (parked craft: the law predicts 375.0 units/s).
   - Pulse PS2 (inherits PSP; unread). HD's own code `4` in its projectile switch (HD runs no cage
     off the track header; its sweep function is unnamed). The Disruptor's own `advance`
     (`projectile/disruptor.rs`) was not looked at.
   - What poses the craft's box collider against `body+0x30` when the weapons update (item 3).

## Next Steps

1. **Wire item 1.** `oag-weapons` depends on `oag-physics` only, and the track half of the cage is
   `oag_gameplay::cage::CageTrack`, so the test has to arrive as an argument:
   - Give `oag_physics::cage` a one-point query (a trait or a function the caller passes) that
     returns the boundary point and inward normal when a point is outside, with the sibling rule of
     `cage::resolve` (outside the located section and, at a fork, outside the sibling's; the nearer
     boundary) and **without** `MAX_POINT_DISTANCE` and `MAX_DEPTH`. Implement it on `CageTrack`
     (`locate`, then `Section::test`).
   - Thread it through `projectile::step` and `Projectiles::advance`
     (`crates/weapons/src/projectile/flight.rs`); the caller is `crates/raceplay/src/tick.rs:248`,
     which has `Spline::cage()` (`None` on HD, Omega and Pure, so only Pulse changes).
   - **Probe** (`flight.rs`, the `probe` match): the point to test is the raycast's hit point when
     there is one (any surface class), else `to - surface * probe_length`. Outside the cage is the
     original's code `4`, taken before the surface class: Rocket and Plasma end there (as their
     probe-wall arm does today, `blast: false`, the impact at the boundary point); Missile and
     Shuriken do nothing this tick, **neither ride nor fall**. A floor hit outside the cage is a
     `4`, not a ride.
   - **Travel sweep** (the `hit` match after `nearest_hit`): the point to test is the geometry
     hit's point, else `to`. Outside: Rocket, Plasma and Cannon end as on a wall (the Cannon with
     its sparks), the Missile and the Shuriken end **without a bounce** (not `may_bounce`). A hull
     hit stays what it is today.
   - Tests: seed a `Projectile` from the table "States a test can start from" on the linked
     section and step it with the listed `dt`: Plasma from update 45 ends at update 49 with the
     impact within 0.5 unit of `(421.77, 145.73, -284.12)` or `(424.66, 153.08, -292.16)`;
     Shuriken from update 50 ends at update 58 near `(495.83, 144.73, -278.27)`; the Cannon round
     ends at update 25 near `(553.28, 143.17, -280.29)`; the Outpost 7 rocket from update 6 ends at
     update 21 near `(-1003.03, 27.32, 324.72)`. Each fails today (the projectile is still alive).
     A cheaper pin: the 17 rows of "The cage test reproduced" through the one-point query.
     Walk `rocket_floor_trace.rs` afterwards: its "expired" column on Pulse `07` and `10` should
     reach zero.
   - Risk: `oag-weapons` is simulation. The slot layout does not change, but every Pulse race
     where a projectile reached the cage plays differently from that tick, so committed Pulse
     references and AI boards that count weapon deaths can move (regenerate in a separate commit,
     naming this law). The gameplay determinism reference flies a rocket over a synthetic floor
     with no cage and should not move; if it does, find out why first.
   - Update `docs/gameplay/weapon-lineage.md` (flight cells of Rocket, Missile, Plasma, Shuriken,
     Cannon: Pulse measured, HD/2048/Omega run no cage) and the stale comments naming `4` a craft
     hit (`flight.rs` near the probe's `match`, `projectile-floor.md`'s first table).
2. **Wire item 2**: `cannon::launch` speed `(craft km/h + BASE_SPEED_KMH) * 0.75` units/s (the 0.75
   is the craft's display scale, the same source as `rocket::LAUNCH_SPEED_SCALE`). Test: a craft at
   139.111 units/s gives 750.60; parked gives 375.0. It moves every Cannon test and the AI's Cannon
   use; HD shares the code and inherits it (its own `Cannon_Init` is unread). Worth one more
   capture first: `psp-projectile-sweeps.py --player-fire-frame 120 cannon` from a parked craft
   (the Talon's Junction grid state parks against a wall) and read `|velocity|` and `+0xdc`.
3. Item 4 next (small, one field): the craft's located track sample is already what the mag hold
   reads; the launch needs its `down`. Expected: on Outpost 7's ramp (sample 186) the carried
   normal at launch is `(0, 1, 0)` where the craft's up is `(-0.014, 0.996, -0.090)`.
4. Item 3 needs the box pose read before it is wired: break at a probe's return with the box
   collider's own pose beside `body+0x30` (the collider is `world + index * 0xc + 0x2454`, the index
   is in the capture's `mesh_collider`), on a floor at two speeds and on the mag strip. Then a probe
   that meets any craft's box first reports nothing, and `plasma::UNPINNED_SECONDS` goes.
5. Items 5 and 6 are one change each in `flight.rs`'s floor arms; the Shuriken capture's speed
   series and the Missile's per-update errors on the linked pages are their tests.
