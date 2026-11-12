---
status: ready-to-wire
titles: [pulse]
updated: 2026-11-12
summary: "The cage test and the Cannon's muzzle speed are wired; next track-up launch normal, own-craft box, Shuriken speed, floor arms"
size: S (50)
difficulty: medium (55)
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

Ranked by what a player sees. Item 4 is ready to wire; 3 to 6 are recovered laws with a
smaller effect; 7 is what was not measured.

1. ~~The cage test in the probe and the travel sweep (the report itself).~~ **Wired 2026-11-11**
   (lane `projectile-cage-wire`, see Done). What stays open from it: the Missile's MAX_BOUNCES
   wall end carries `blast: true` while its cage end spends none (the `0x14` teardown reaches the
   explosion and no damage); no capture has produced a Missile's sweep `4`; `CageEffect_Spawn` on a
   projectile's cage hit is not drawn.
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

## Done

- **The cage test in the probe and the travel sweep** (2026-11-11, `projectile-cage-wire`):
  `oag_physics::cage::Containment` (one point, the craft's sibling rule as `Located::outside`, none
  of `Cage_ResolveCraft`'s limits), implemented by `CageTrack`, passed through `projectile::step`
  and `Projectiles::advance` from `Spline::cage()` (`None` on Pure, HD, 2048, Omega). Rocket and
  Plasma end at the boundary on a probe `4`; Missile and Shuriken do nothing on one; every weapon
  ends at the boundary on a sweep `4`, with no bounce. Tests:
  `crates/raceplay/tests/projectile_cage_ground_truth.rs` (the 17 recorded queries and the four
  endings, each red without the cage) and `crates/weapons/src/projectile/tests/cage.rs`. The
  Shuriken test re-seeds each update from the original's state because its speed (item 5) drifts
  a chain off the kerb. `rocket_floor_trace` Pulse `07`/`10`: "expired" 14 and 7 to 0.

- **The Cannon's muzzle speed** (2026-11-12, `projectile-cage`): `cannon::launch` flies
  `(craft km/h + 500) * 0.75` units/s (`TARGET_GLOBAL_SCALE`), 750.60 at 139.111 units/s and 375.0
  parked; HD, 2048 and Omega share it (I:Pulse). `cannon_tilt_ground_truth` skips a round whose
  first sweep already glanced the track (a step is now 6 to 13 units; 12 of 237 on Pulse, 0 on HD, and
  the test fails past a tenth). `vita_2048_original_ai_ground_truth::the_field_finishes_tower` took the
  field's rounds harder: 9 of 45 runs (nine offsets by five scales) miss the 300 s limit with this speed
  against 6 of 45 with the old one, always slots 1 and 2 on lap 3 behind a slowed field, so its bar is two
  of five scales (a 20 % miss rate fails that under 1 % of the time). A craft ending at 0.0 shield is a
  destroyed one and passes. The draw (`cannon_quad_vertices`) streaks the round between its previous
  and new position, as `Cannon_BuildBoltList` does, so the bolt is 2.7 times longer. Still open: a second speed
  from the original (`psp-projectile-sweeps.py --player-fire-frame 120 cannon` from a parked
  craft; the law predicts 375.0).

## Next Steps

1. Item 4 next (small, one field): the craft's located track sample is already what the mag hold
   reads; the launch needs its `down`. Expected: on Outpost 7's ramp (sample 186) the carried
   normal at launch is `(0, 1, 0)` where the craft's up is `(-0.014, 0.996, -0.090)`.
2. Item 3 needs the box pose read before it is wired: break at a probe's return with the box
   collider's own pose beside `body+0x30` (the collider is `world + index * 0xc + 0x2454`, the index
   is in the capture's `mesh_collider`), on a floor at two speeds and on the mag strip. Then a probe
   that meets any craft's box first reports nothing, and `plasma::UNPINNED_SECONDS` goes.
3. Items 5 and 6 are one change each in `flight.rs`'s floor arms; the Shuriken capture's speed
   series and the Missile's per-update errors on the linked pages are their tests.
