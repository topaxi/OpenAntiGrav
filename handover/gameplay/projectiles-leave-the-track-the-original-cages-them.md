---
status: ready-to-wire
titles: [pulse]
updated: 2026-11-13
summary: "Cage, launch normal, floor arms, own-craft box wired (box per title); open: HD box, opponent box, travel sweep, HD floor arms"
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

Ranked by what a player sees. Item 3 is a recovered law with a smaller effect; 7 is what
was not measured.

1. ~~The cage test in the probe and the travel sweep (the report itself).~~ **Wired 2026-11-11**
   (lane `projectile-cage-wire`, see Done). What stays open from it: the Missile's MAX_BOUNCES
   wall end carries `blast: true` while its cage end spends none (the `0x14` teardown reaches the
   explosion and no damage); no capture has produced a Missile's sweep `4`; `CageEffect_Spawn` on a
   projectile's cage hit is not drawn.
3. ~~The first probes of a projectile miss on the firing craft's box collider.~~ **Wired 2026-11-13**
   (lane `pulse-own-craft-box`, see Done). Open from it: the travel sweep's own box meeting (65: same
   function, no reading of its own), an opponent's box (75), HD's box at 65 (`ps3-hdfury-eu/own-craft-box.md`: the
   collider's `+0x74` filter and row-0 sign are open, so HD keeps the chosen 0.04 s Plasma window), and the 2,302-probe replay as a Rust test: it cannot be ported until someone re-captures the 56 files
   (the originals were scratch and are gone; `scripts/psp-projectile-sweeps.py --box ... --out FILE`,
   checked by `scripts/psp-craft-box-replay.py`). Keep them where a reap does not delete them, e.g.
   `data/traces/pulse-own-craft-box/` in the main checkout (the replay script's docstring names it).
   Until then the test is the page's seven records (`projectile/tests/craft_box.rs`).
7. **Not measured or not read.**
   - The Missile's travel sweep on code `4` (`flags |= 0x14`, no bounce): decompile only, no capture
     produced it. Fire a Missile across a jump lip (Outpost 7 sample 186) to see it.
   - `CageEffect_Spawn` on a projectile's cage hit (every weapon passes `flags == 0`): read, not
     watched; no breakpoint on `0x08832b0c` during a capture.
   - A second Cannon speed (parked craft: the law predicts 375.0 units/s).
   - Pulse PS2 (inherits PSP; unread). HD's own code `4` in its projectile switch (HD runs no cage
     off the track header; its sweep function is unnamed). The Disruptor's own `advance`
     (`projectile/disruptor.rs`) was not looked at.

## Done

- **The cage test in the probe and the travel sweep** (2026-11-11, `projectile-cage-wire`):
  `oag_physics::cage::Containment` (one point, the craft's sibling rule as `Located::outside`, none
  of `Cage_ResolveCraft`'s limits), implemented by `CageTrack`, passed through `projectile::step`
  and `Projectiles::advance` from `Spline::cage()` (`None` on Pure, HD, 2048, Omega). Rocket and
  Plasma end at the boundary on a probe `4`; Missile and Shuriken do nothing on one; every weapon
  ends at the boundary on a sweep `4`, with no bounce. Tests:
  `crates/raceplay/tests/projectile_cage_ground_truth.rs` (the 17 recorded queries and the four
  endings, each red without the cage) and `crates/weapons/src/projectile/tests/cage.rs`. The
  Shuriken test first re-seeded each update from the original's state because its speed drifted a
  chain off the kerb; it chains from update 50 since the floor arms (below). `rocket_floor_trace` Pulse `07`/`10`: "expired" 14 and 7 to 0.

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

- **The launch normal** (2026-11-12, `launch-normal-wire`): a Rocket, Missile, Shuriken or Plasma bolt
  is born carrying the up of the spline sample the craft is located on (`Ship::track_up`, written by
  `step_player` and `step_opponents` from the sample the force law reads; `Craft::launch_normal`),
  not `body.up()`; per title as `Weapons::launch_normal_is_track_up` (Pulse measured, Pure inherited
  from Pulse; HD chosen `false` and 2048, Omega inherited `false`: with `true` three HD/Omega field
  tests go red, see `a-projectile-rode-the-angle-not-the-surface-class.md`). The Cannon and the Disruptor keep the craft's up.
  Test: `projectile_cage_ground_truth::outpost_7_ramp_launch_normal_is_the_tracks_up` (a Plasma
  press from sample 186 with the capture's craft up: 0.05 degrees from the sample's `-down`, over 5
  from the craft's up) and `seed::a_plasma_bolt_is_released_riding_the_located_track_up`. Sample
  186's up is 0.26 degrees from `(0, 1, 0)`. Not observable per weapon in raceplay: a Rocket,
  Missile or Shuriken's first update replaces the seed with the floor's normal, so those three call
  sites are pinned only by the weapons crate's tests of the handed-in normal. Still open from it:
  HD's, 2048's and Omega's own `Rocket_Init` unread (reading HD's decides whether they take the
  law); the Disruptor's advance unread.

- **The floor arms** (2026-11-12, `pulse-floor-arms`): `Weapons::floor_arms_steer_to_ride_point`
  (Pulse measured, Pure inherited, HD, 2048 and Omega chosen `false`) selects `projectile::FloorArms`.
  Pulse and Pure: the Shuriken's probe arm and the Rocket's and Shuriken's sweep arm write
  `(ride point - start) / dt` raw (the blade's speed drifts), the Missile's probe arm steers to the
  ride point, the Plasma's sweep arm only moves. Tests: `projectile_cage_ground_truth::
  moa_therma_white_shuriken_ends_at_the_cage` (now chained from update 50, no re-seed, velocity within
  2 units/s of the original's at each update, ends at update 58) and `weapons/.../tests/floor_arms.rs`
  (each arm's formula on a flat floor, both ways). **Not disc-backed**: the Missile's per-update
  errors and the Rocket's ramp-face exit were never kept as recorded states, so those arms are unit
  tests; recording them (`psp-projectile-sweeps.py` on Outpost 7's ramp, a Missile on Moa Therma)
  would give them a ground truth. Still open from it: HD's, 2048's and Omega's own Rocket, Missile
  and Shuriken floor arms are unread (HD's Plasma sweep writes no floor arm at all, so it differs from
  Pulse's "only moves").

- **The own-craft box** (2026-11-13, `pulse-own-craft-box`): `projectile::own_box_blocks` and
  `OwnCraftProbe`, chosen per title by `Weapons::own_craft_box_hides_track` (Pulse measured; others
  `false`, where a Plasma keeps a chosen 0.04 s window in `oag_raceplay`). Every probing flyer of the
  firing craft only. `plasma::UNPINNED_SECONDS` deleted. Tests: `projectile/tests/craft_box.rs` (the
  page's seven records, reach 4.875 and 2.06, mag strip) and
  `projectile_cage_ground_truth::the_grid_craft_hides_the_floor_from_its_own_first_probes_on_pulse`.
- **The own-craft box on the other titles** (2026-11-13, `own-craft-box-other-titles`): each read on
  its own executable, `OwnCraftBox` per title: Pure `Holds` (84, Pulse's box and gate, a box hit runs
  no arm), 2048 and Omega `Absent` (84, mesh-only query, no window), HD left `PlasmaWindow` (65).
  [projectile-floor.md](../../docs/gameplay/projectile-floor.md), "The other titles".

## Next Steps

1. **HD's box** (`oag-capture`, RPCS3): break on the box call at `0x00036cac` and read `0x0007be58`'s return while a parked craft fires; if the box answers, set HD to `OwnCraftBox::Falls` with HD's own probe lengths (Plasma `6.0`). Then an opponent's box and the travel sweep (items above).
2. Read HD's `Rocket_Update`, `Missile_Update` and `Shuriken_Update` floor arms (`ps3-hdfury-eu`); if they match Pulse's, flip `Weapons::floor_arms_steer_to_ride_point` for HD (and Omega, 2048 by lineage).
