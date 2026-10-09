# A projectile follows the floor, and what decides wall from floor

How a Rocket (and the Missile, Plasma and Shuriken with it) rides the track,
why it used to leave the circuit through the barrier or blow up on a dip, and
what the fix was. Implemented in
[`oag_weapons::projectile`](../../crates/weapons/src/projectile.rs)
(`Projectiles::advance`) and
[`projectile/geometry.rs`](../../crates/weapons/src/projectile/geometry.rs)
(`nearest_hit`); the recovered model is on
[rocket-visuals.md](../ghidra/functions/psp-pulse-usa/rocket-visuals.md#flight-follows-the-track).

## Read this first: what is recovered and what is ours

| Piece | Status | Confidence |
| --- | --- | --- |
| Probe `6.0` (Rocket) / `12.0` (Missile, Plasma, Shuriken) along the ridden normal, ride `3.0` above a hit, fall `50.0` units/s^2 on a miss | **recovered** | 82-92 |
| **The branch is on the surface class of the hit**: wall or craft ends the flight, floor or mag floor is ridden, nothing means fall | **recovered 2026-09-13**, `Collision_SweepSegment` (`0x0883198c`) read at decompiler level | 85 |
| A floor met across the travel segment is *landed on* (`hit + normal * 3.0`), for all four weapons | **recovered 2026-09-13**, `Rocket_Update` and `Missile_Update` decompiles | 85 |
| On that landing the Missile moves and nothing else - no normal adopted, no velocity written - where the Rocket and Shuriken adopt the normal and recompute velocity | **recovered 2026-09-13**, the same two decompiles | 85 |
| A wall found by the *probe* detonates a Rocket or a Plasma and does nothing to a Missile or a Shuriken | **recovered 2026-09-13** | 80 |
| Keeping the speed when the Rocket, Plasma or Shuriken's velocity is turned onto a floor met across the segment | **ours** - the original writes `(next - prev) / dt` and rescales on the next probe | - |
| The fall is along world `-Y` for the Rocket, Missile and Shuriken, along the carried normal for the Plasma, and absent for the Cannon | Rocket and Shuriken **recovered** (`velocity.y -= dt * 50`); the Plasma **recovered** as `velocity -= surface * (dt * 50)` (`plasma.md`) and **wired 2026-10-08** - it fell along world `-Y` before, a different direction on any bank. The Missile's fall axis is not separately read | 85 |
| The normal a projectile is born riding: the firing craft's up for the Rocket, Missile, Shuriken and Plasma (`Rocket_Init`, `Missile_Init` `+0xd0`, `Shuriken_Init` `+0x150`, `Plasma_Launch` `+0x110` all store `-(craft+0xb10)`) | **recovered 2026-10-07**, `Missile_Init`/`Shuriken_Init` decompiled; wired through `spawn_riding`, `spawn_guided` and `throw`. A seed of world up (`Vec3::Y`) remains only for tests | - |

## The Cannon is not in this table at all

**Recovered 2026-10-07, confidence 82.** `Cannon_UpdateRound` (`0x0886593c`)
has no surface probe, no ride height and no fall: it moves `position +
velocity * dt`, sweeps that segment through `Collision_SweepSegment`
(`0x0883198c`) and branches on the code - `0x7f` keeps flying, `0`/`4` is a
wall (flags `0x14`, `WO_CANNON_SPARKS`), anything else reflects the velocity
about the normal and sets `position = hit + normal * 3.0`. Until then the
Cannon ran the generic follower above, seeded with world up, so on a banked or
looping track its first probe went straight down the *world*, found a floor
that was not under the craft and snapped the round up to 7 units off its
muzzle (and about 1 unit down on flat track). Maintainer report: "on a tilted
track, for example the Moa Therma loop, the cannon fires with an offset to the
side of the craft."

Measured on Moa Therma (`03_Track`), autopilot lap, a burst every 1.5 s, 496
rounds, offset of each round after its first tick from `cannon::launch` plus
one step: worst 7.05 units before, 0.00 after; the 271 rounds from a craft banked
past 18 degrees (`up.y < 0.95`) averaged 0.91 units off the muzzle before and
the 225 on flatter track 1.04 (the drop to ride height). The spawn
itself (`cannon::launch`, built from the craft's right and forward) was never
at fault: where the probe missed, the residual was exactly zero. Pinned by
`oag_weapons::projectile::cannon::tests` (a craft rolled 0 to 135 degrees over
a floor) and the disc-backed
`crates/game/tests/cannon_tilt_ground_truth.rs`.

**Lineage.** HD authors a Cannon (`weaponstats_Race.xml`, drawn by
`hd_weapon_scene_ground_truth`) and runs the same `Race::advance_cannons` and
`Projectiles::advance`, so HD's Cannon now flies straight too: **checked,
applies, wired by sharing the code**, with Pulse's law inherited and no HD
`Cannon_UpdateRound` read (**not checkable** here without an HD capture). 2048
and Omega name their own weapon tables (`weaponstats_Race_2048.xml`,
`WeaponStats_Race.xml`) but whether those author a Cannon block was **not
checked** this lane; if they do, they inherit the same flight. The Missile,
Shuriken and Plasma seed reaches every title through the same shared code.

## The report

A player on HD/Fury in `oag-game`: "rockets don't properly follow the
path/floor - in a looping or wavy section the rockets disappear or go
straight, instead of going with the floor." Treated as a reliable oracle,
and it was one.

## What was measured

`crates/game/tests/rocket_floor_trace.rs` (a scratch trace, `#[ignore]`d and
gated on `OAG_ROCKET_TRACE`) flies the player on the autopilot, fires a volley
every two seconds, and re-runs each rocket's own probe and sweep against the
race's own `CollisionWorld` (`Race::collision`) every tick, so the tick a
rocket stops on names why. 90 seconds a circuit, VENOM, single race with the
field on. The columns that matter: how many rocket-ticks were spent *falling*
(probe miss), and how many rockets **expired** - flew their whole 10 s cap
without meeting anything, which on a closed circuit means they had left it.

| Circuit | Title | falling ticks before -> after | expired before -> after | ends before -> after |
| --- | --- | --- | --- | --- |
| 15 Anulpha Pass | HD | 37,215 -> 111 | 58 of 123 -> 0 of 129 | wall 43, hull 22, expired 58 -> wall 113, probed wall 1, hull 15 |
| 12 Sol 2 | HD | 16,025 -> 9,361 | 27 of 129 -> 16 of 132 | wall 83, hull 19, expired 27 -> wall 96, probed wall 1, hull 19, expired 16 |
| Amphiseum | HD | 26,541 -> 421 | 39 of 121 -> 0 of 134 | wall 57, hull 25, expired 39 -> wall 96, probed wall 4, hull 34 |
| 10 Sebenco Climb | HD | 2,449 -> 86 | 4 of 123 -> 0 of 120 | wall 104, hull 15, expired 4 -> wall 89, probed wall 18, hull 13 |
| 16 Talon's Junction | Pulse | 34,402 -> 107 | 52 of 120 -> 0 of 126 | wall 50, hull 18, expired 52 -> wall 98, probed wall 4, hull 24 |
| 03 | Pulse | 15,541 -> 248 | 26 of 128 -> 0 of 126 | wall 79, hull 23, expired 26 -> wall 98, probed wall 2, hull 26 |
| 07 | Pulse | 9,021 -> 3,543 | 15 of 129 -> 6 of 126 | wall 96, hull 18, expired 15 -> wall 107, probed wall 1, hull 12, expired 6 |
| 10 | Pulse | 4,385 -> 1,590 | 7 of 129 -> 2 of 132 | wall 109, hull 13, expired 7 -> wall 111, probed wall 3, hull 16, expired 2 |

"wall" is a detonation on the travel sweep, "probed wall" one on the surface
probe (the original's code `0` on either query), "hull" a craft. The HD
autopilot beaches on Sebenco Climb at t=1560, so that row's later volleys
fire from a standing craft against the barrier; Sol 2 and Anulpha Pass are the
two HD rows the autopilot laps.

**What is left is jumps, and it is the recovered model doing what it says.**
Every rocket that still expires - 16 on Sol 2, 6 on Pulse's 07, 2 on 10 - was
fired up a ramp that ends in the air: Sol 2's climb at 23 degrees, 07's at 43.
Riding the ramp turns the velocity along it, so the rocket leaves the lip
with 108-190 units/s of climb against a fall of `50.0` units/s^2, reaches its
apex two to four seconds later and comes down a kilometre away. That is the
Pulse-measured constant at the Pulse-measured speed, and this project does not
retune it to suppress a symptom; whether the original's rockets sail off the
same lips is a play-test question, recorded as open.

**Pulse broke the same way as HD**, so the cause was the shared geometry and
HD was incidental. Nothing about HD's collision world was at fault: every
circuit loads its full floor (9,018 floor triangles on Anulpha Pass, 9,420 on
Sebenco Climb, all wound with `normal.y > 0` where the road is upright), the
half-widths (48-110 units) sit beside Pulse's (49-79), and the rocket flies the
same 277.8 units/s (`800 + 200` km/h over 3.6) on both.

## The mechanism, tick by tick

Every rocket that left a circuit did it the same way. From the Anulpha Pass
trace, before the fix:

```text
t=500 slot=2 age=20 pos=(-1113.4,-18.6,-203.8) n=(0.01,1.00,0.01) RIDE probe=hit d=3.00 Floor sweep=-
t=501 slot=2 age=21 pos=(-1117.9,-18.5,-204.8) n=(0.01,1.00,0.01) FALL probe=MISS sweep=hit d=1.68 facing=0.25 Wall
t=502 ... FALL probe=MISS sweep=-          (and so on for 578 more ticks, to y=-865)
```

The rocket rode the floor until its chord met the **barrier at a shallow
angle** - `facing` is `-normal . direction`, and `0.25` is about 75 degrees
off the wall's normal. `nearest_hit` dropped any geometry hit under
`WALL_FACING` (`0.25`) as "the floor being clipped", so the rocket passed
through the wall, found no floor on the far side, and fell at `50.0` units/s^2
until its 10 s cap reaped it. On a wavy or banked stretch the aim line
diverges from the road sooner, so that is where it showed.

The other half is the same constant from the other side: a floor the chord
met at *more* than 75 degrees - the far side of a dip, after a crest had put
the probe out of reach - passed the filter and **detonated**, where the
original lands on it.

Both were one mistake. `Projectiles::advance` carried a doc comment saying
"the original picks detonate or deflect from a collision code its query
returns; this engine's raycaster has no such code, so the test is geometric."
The raycaster has always returned `RaycastHit::surface`, and
`Collision_SweepSegment` (`0x0883198c`) returns exactly that: the struck
collider's `+0x6c` surface type, `0` wall, `1` floor, `3` mag floor, `0x7f`
for nothing, `4` for the craft test. `Rocket_Update`'s switch is on that
number, not on any angle.

## The fix

`nearest_hit` reports every geometry hit and carries its `Surface`;
`Projectiles::advance` branches on it:

- **probe** - nothing: fall; floor/mag floor: ride (adopt the normal, sit
  `3.0` above, turn the velocity parallel); wall: Rocket and Plasma detonate,
  Missile and Shuriken do nothing this tick;
- **travel segment** - craft: detonate; wall: Rocket and Plasma detonate, the
  Missile mirrors up to its budget, the Shuriken mirrors without one;
  floor/mag floor: **land on it** - sit `3.0` above; the Rocket, Plasma and
  Shuriken also adopt the normal and turn their velocity parallel with its
  speed kept, the Missile only moves.

`WALL_FACING` and `RIDEABLE_COS` are gone. The Pulse-measured `6.0`, `3.0` and
`50.0` are untouched, and nothing here is on the title axis: HD needed no
number of its own.

The gameplay determinism reference (`crates/gameplay/tests/determinism.rs`)
did not move: its scenario is a rocket over a flat floor into a face-on wall,
which both models resolve identically.

## Still open

- **Any floor-tagged triangle is now ridden, at any angle.** That is the
  original's own rule - it has no angle test - and the trace has the case it
  admits: `t=1206 slot=15 ... probe=hit d=0.97 cos=0.09 MagFloor` on Pulse's
  03, a near-vertical mag-floor face adopted as the ride surface for a tick.
  Recorded rather than guarded, because a guard would be the same invention
  this page removed.
- **A probe wall hit spends one blast here** where `Rocket_Update` falls
  through to its travel sweep and can spawn a second `WO_ROCKET_EXPLO_TRACK`
  on the same tick. Deliberate.
- **The born normal.** A rocket is spawned riding `Vec3::Y`; the original
  seeds `self+0x100` from the firing craft. Fired down Amphiseum's drop the
  probe along `-Y` missed the near-vertical road for 24 ticks before the
  rocket landed at the bottom. One tick on a flat grid, a second of free
  flight on a drop.
- **The speed after a segment landing** is kept here and recomputed as
  `(next - prev) / dt` in the original. A rocket that lands hard loses speed
  for one tick there and is rescaled by the next probe; here it never slows.
- **Jump lips**, as above: a play-test of the original on Pulse's 07 ramp
  would say whether its rockets sail the same way.
- **HD's own projectile update** has not been read. Everything above is
  Pulse's model applied to HD's geometry, which the measurement supports and a
  read of `ps3-hdfury-eu` would settle.

## Do the other projectiles share the Cannon's bank fault? (2026-10-08)

Maintainer report, played on HD after the Cannon fix: "a similar issue might be
going on with rockets" - asked of the Missile, Plasma and Shuriken too, "across
all titles". **Answer: no. The Rocket, Missile and Shuriken ride a bank
correctly; one real fault was found, the Plasma's fall axis, and fixed.**

The Cannon's fault was a ridden normal seeded with world up, so its first probe
landed on a different deck. The four other floor followers now carry the craft's
up from birth (2026-10-07) and probe along it, so they cannot repeat that.
`crates/game/tests/projectile_tilt_ground_truth.rs` flies each one off the
autopilot's craft, fired through the pad every 30 ticks, on Pulse's Moa Therma
(`03_Track`, the loop) and HD's Vineta K (`01_vineta_k/track.vex`), and on every
tick a projectile is in the air casts the weapon's own probe along `-surface`
(`6.0` Rocket, `12.0` the rest). Floor classes only. Measured on this tree
(roughly 4,000 judged ticks per weapon per title, 20 or more shots from a craft
tilted past 18 degrees each):

| Weapon | Pulse worst angle / height error | HD worst angle / height error |
| --- | --- | --- |
| Rocket | 15.0 deg / 0.47 | 5.0 deg / 0.36 |
| Missile | 0.0 deg / 0.00 | 1.2 deg / 1.02 |
| Plasma | 14.5 deg / 0.00 | 13.2 deg / 0.00 |
| Shuriken | 14.9 deg / 0.87 | 3.1 deg / 1.01 |

**The fault found: the Plasma fell along world `-Y`.** `Plasma_Update`'s `0x7f`
arm is `velocity -= surface * (dt * 50.0)` (`plasma.md`, confidence 85); this
engine gave it the Rocket's `velocity.y -= dt * 50.0`. Wired in
`flight.rs`; pinned by
`projectile::tests::seed::a_plasma_bolt_over_nothing_falls_along_its_carried_normal`
(fails on the old code). Control run: with every seed forced back to world up
*and* the old fall, the Plasma's ride test fails (worst angle 70.9 deg on Pulse,
39.0 on HD); that is the seed's doing (the seed alone breaks it), not the
fall arm's.

**What the ground truth does not prove.** The Plasma's fall fix is pinned by the
unit test alone: a disc-backed check of the velocity change over consecutive
no-floor ticks found 103 fall ticks on HD and none on Pulse, and **none with the
carried normal off world up**, so it cannot tell the two axes apart and was not
kept. Likewise, with the seeds alone forced back to world up the Rocket,
Missile and Shuriken ride tests still pass: the first floor
hit adopts the real normal, and on these two circuits' banks the wrong first
probe is rarely enough to leave the floor. The seeds are pinned by the unit
tests in `projectile/tests/seed.rs` (each fails if its seed is dropped) and the
Missile's by `a_missile_fired_on_a_bank_is_born_riding_the_craft_up`; the ride
test is the guard that nothing re-introduces the Cannon's displacement, not a
seed discriminator.

**The Cannon round was drawn rolled to world up (found and fixed here).**
`Projectiles::spawn` seeds `surface = Vec3::Y`; the Cannon never probes, so the
round carried world up for its whole life, and `projectile_model_matrices` reads
`surface` as the body's up. Measured on the loop: the least dot of round up with
the firing craft's up was 0.04 on Moa Therma (a round drawn nearly 90 degrees
rolled) and 0.78 on Vineta K; after the fix 0.99999 on both
(`cannon_tilt_ground_truth`, 255 and 217 rounds). `advance_cannons` now spawns
the round carrying the craft's up (`spawn_riding`). **The law is a reading, not a
measurement:** `Cannon_Init` (`0x088648ec`) copies the craft's muzzle anchor
into the round's basis (measured live as `(up x f, up, f)`, but only on a near-flat
track where world up and craft up agree), so a round off a banked craft carries
the craft's up; the draw from that basis is the old confidence-70 inference.
`surface` is hashed state for a Cannon round now, see the regeneration note in
the report.

**Trails and the other drawn poses.** The HD rocket ribbon builds its rows from
`projectile.surface` (`rocket_smoke.rs`), so it follows the bank. The Pulse
Missile's two orbiting flares use world up as `up`, **which is the original's**
(`missile.md`: `world_up` is a hardcoded `(0, 1, 0)`, instruction level,
confidence 80), and the Plasma and Missile flare frames were never read (see the
note at `Race::advance_projectile_flares`), so they keep world up; **not
checked on a bank against the original.** This engine draws no Missile body model (`projectile_sprites`' own note), so there is no pose to roll.

**Mine and Bomb.** Laid where the craft is, to the bit, on every tilted drop
(Pulse 482 mines / 225 bombs, HD 204 / 98 on a craft past 18 degrees), and the
Bomb's pose (Pulse) and both charges' pose (HD, the frozen craft pose) have the
craft's up where the craft's is (pose.up above 0.99). Pulse's Mine spins by
design (`Mine_PoseNode`).

**Early deaths on a bank were the craft, not the weapon.** The first HD run
showed 39 of 57 rockets ending within a few ticks from craft at up.y 0.8; every
one sat at one spot (-743, -23, -140) where the autopilot craft was wedged
against a wall, a Wall class one to two units ahead. Not a tilt effect.

**The HD rocket basis (`up = -surface`).** `rocket-trail.md` reads row 1 = `-up`
and row 0 = `forward x row 1` off the *ribbon's* node, a matrix with determinant
`-1`. The six fins sit at 60 degree steps, a set unchanged by flipping up, so the
trail is the same picture either way and does not depend on roll; it is not the
rocket body's matrix and was not touched. Pulse's basis for the body
(`row0 = n x f`) equals HD's row 0, so only the sign of the middle row differs.

**Pictures.** `pulse-rocket-bank.png` (Moa
Therma, at the loop's foot, three rockets climbing the wall) and
`hd-rocket-bank.png`, `hd-missile-bank.png` (Vineta K, at (-282, 86, 106) where the
autopilot's craft read up.y -0.68): the volley follows the track surface and the
trails lie along it. There is no before/after pair, since nothing in the
Rocket/Missile/Shuriken flight changed.

**Lineage.** Pulse and HD: **ported** (the shared code, a ride test each).
Pure shares the code and authors the Rocket, Missile, Plasma and Shuriken:
**checked, applies, not wired** (no circuit with a bank chosen, no Pure test).
2048 and Omega: **checked, applies, not wired** - the shared code reaches them,
no weapon is raced on either (`docs/overview/status.md`).
