# Weapons and pickups specification

What a weapon pad hands out, and what each weapon does once fired. Pulse (PSP)
is the reference. A reader who never opens the rest of this repository should be
able to rebuild the behaviour from this page plus the disc's own tables.

**What is on the disc and what is not.** Damage, blast radius, blast force,
`slowdown_time`, durations, per-class speeds and pad odds are authored in the
disc's `WeaponStats_*.xml`. They are not reproduced here
([ADR-0006](../architecture/adr/0006-no-copyrighted-content.md)); this page names
the attribute and gives the law that spends it. Numbers below are code literals
out of the executable, each with a 0-100 confidence
([rubric](../reverse-engineering/confidence-rubric.md)). A row with no score is
**chosen, not measured** by this project.

Speeds are authored in **km/h** and divided by `3.6` to units/s. Pure authors one
`speed` per weapon; Pulse and HD author `venomspeed`, `flashspeed`,
`rapierspeed`, `phantomspeed`. The simulation runs at a fixed 60 Hz.

## Pickups

### Granting

A craft crossing a weapon pad grants a pickup (85). One slot holds one weapon id
(88). The draw (92):

```text
r = rand() % total_weight                 cumulative walk over the weapon list
weight = ai          for an AI craft (flat)
       = human blended with front/back by race position, for a human
no weapon twice running (redraw)
```

Weights come from `<Pickupodds class="...">`, one block per speed class, each
weapon carrying `ai`, `human`, `front`, `back`. A weapon the title's table does
not author weighs zero. The Repulser has zero odds outside Eliminator; Shuriken
and Repulser are gated by mode. The original's PRNG sequence is not recovered, so
only the distribution is specified.

Pads: `<WeaponPad refresh_time>` is a **debounce**, not a respawn. The stamp is
written on any hit, including one that grants nothing, and `elimination_refresh_time`
replaces it in Eliminator. Pads exist and trigger only in modes with weapons on.

### Absorb

A press of the absorb button spends the held weapon and pays its `absorb`
attribute into the shield pool, clamped at the pool maximum, never floored
(`Ship_SetShield` takes `min(amount, max)`; unscored in the evidence).
Works for every weapon. In Eliminator absorb pays nothing and fires a one-second
Shield instead (80).

### Hits, damage, slowdown

- A weapon hit posts damage and `slowdown_time` onto the victim; the slowdown
  law is in [physics](physics.md#slowdown-from-weapons) (88). A running Shield
  pickup blocks the slowdown add (85).
- Damage goes through the shield pool (see [physics](physics.md#wall-contact)).
- A posted collision impulse arms the stun timer by `0.5` s, added not
  assigned (85), unless a Shield pickup runs. Which weapons post one is only
  partly identified: do not assume every blast stuns.

## Projectiles that follow the floor

Rocket, Missile, Plasma, Shuriken and Disruptor bolts ride the track; Cannon
rounds do in this engine through the same follower (the original's own
raycast is not reproduced instruction for instruction). Each tick a
projectile probes toward the surface, rides above it, and falls when it finds
nothing (82):

| Weapon | Probe length | Ride height | Fall acceleration |
| --- | --- | --- | --- |
| Rocket | 6.0 | 3.0 | 50 units/s^2 |
| Plasma | 12.0 | 4.0 (measured, 80) | 50 |
| Missile, Shuriken | 12.0 | 3.0 | 50 |
| Disruptor (Pure) | 12.0 | 6.0 | 50 |

The surface class decides wall versus floor, never the hit angle: query result
`0` wall, `1` floor, `3` magstrip, `4` craft. A wall detonates the projectile
except where noted. Velocity is redirected parallel to the surface at constant
speed. Per update the original makes a first, slower flight: a Rocket's or
Plasma's first four updates (about 0.05 s) run at `0.75` of class speed because
the launch direction carries the craft's display scale (88 Rocket, 85 Plasma),
then pin to the class speed.

## Weapons

| Weapon | Law | Confidence |
| --- | --- | --- |
| **Turbo** | For `time` seconds: `T += Engine.turbo` after the thrust cap and before the doubling, so uncapped; with d-pad Up held, boost lift `turbo_jump * T`. Keeps the engine through a stun or slowdown, holds the speed pad at zero and fades it back over one second | 95 (running bit); the rest unscored in the evidence |
| **Shield** | The duration `time` is authored. A running Shield suppresses a posted collision impulse and its stun (85), blocks the slowdown add (85) and suppresses the LeachBeam drain (the first line of the drain, unscored). **Chosen:** that it also refuses all hull damage outright; the original's consumer of `time` was not found. A rocket's blast force still moves a shielded craft | 85 recovered parts |
| **Autopilot** | The player's craft is flown by the original opponent driver for `time` seconds: lane 0, throttle 100, engine multiplier `x1.2` after the cap. Absorb cancels it, fire does nothing, no pad grant while it runs. The `1.2` is the same literal on every title | 90 |
| **Rocket** | Three at once: one straight, one rotated `+spread`, one `-spread` (radians, authored). Lifetime 5 s, then vanishes with no blast. Detonates on its first wall. Speed is `class speed / 3.6` | 88 (fan), 90 (lifetime) |
| **Missile** | One in the air. Lock: target within `acos(0.9)` of forward (about 26 degrees), within `lock_max_dist`, and `\|along-track gap\| / range < 1.4`. A press without a lock still fires. Speed ramps linearly from launch speed (floor 14.4 km/h) to class speed over 1 s. Guidance: `dir = v_unit + clamp(normalize(d - v_unit) * 4 dt, error)`, **not renormalised**, so a turning missile is up to 7 % slower. Survives five wall bounces (pushed 0.1 off), detonates on the sixth hit, or at age 3 s | 95 steer, 90 speed, 88 lock, 80 gap rule |
| **Plasma** | One bolt. Charges 1.0 s before release (a code literal; the authored `charge_time` is read by nothing). Leaves 2.23 units ahead of the craft (75, one craft). Floor follower as the Rocket; ramp to class speed over 1 s | 85 launch ramp, 80 ride height |
| **Cannon** | Fired by holding the button, not by a press: a countdown built from `rate` arms a round each time it reaches zero, alternating two muzzles; `rounds` rounds per pickup. Muzzle speed `craft speed + 500 km/h` (90). Round life 1 s (90). A floor glance reflects the velocity and pushes 3.0 along the normal (82). A wall hit throws sparks; a craft hit within 6.0 does `damage_per_bullet` and no spark (85) | 82 to 90 |
| **Mine** | One press lays five, one every 0.1 s, from the craft's own position (92; measured on the original, position 90). Fuse `timetodie`. The layer is exempt for 0.5 s (recovered for Pulse, HD filmed). **Chosen, not measured:** that a mine does not move, and that a craft inside `trigger_radius` sets one off (the attribute is authored, the code that spends it was not found) | 92 count and interval |
| **Bomb** | A single large mine from the same code path. Pure's Bomb has no fuse. `damageradius` is read by nothing | 90 |
| **Quake** | A travelling bump on the track's spline, not a projectile. Speed 270 units/s along the track, life 5 s, born 15 units ahead of the firer. Height rises linearly to peak 12 over 0.3 s, then falls to zero at 5 s. Half width `25 + 10 * age`. Each craft carries a smoothed height `h += (raw - h) * 8 * dt`, where `raw` is the wave height while the craft is within half the half-width of the centre. A hit lands on the rising edge of `h > 0.1`. The cockpit eye rises by `h`; there is no body kick. A laid mine trips on raw height above 0.1 | 85 to 90 |
| **LeachBeam** | A link between two craft, not a projectile. With no lock the pickup is spent and the beam fizzles after 0.75 s (78). Locked: each tick the victim loses `damage`, the shooter gains `repair`, each times `energy_multiplier` on the first tick. Lives `active_time`; disconnects past `range` or on an invulnerable or shielded target, lingering 0.5 s (80). A shielded shooter suppresses both halves | 80 to 88 |
| **Repulser** | A pool entity living `blast_time + wave_time`: a blast around the firer, then two waves that walk the track control points, **five forward and two back per update**, starting at the firer's place. Each craft a wave sweeps is hit once: full `damage`, full `slowdown_time`, a shove of `blastforce` toward the wave. Laid mines and bombs a wave sweeps go off | 80 to 84 |
| **Shuriken** | One blade thrown 20 degrees (0.349066 rad) off the nose, left or right on a coin flip, carrying the thrower's speed. A wall bounces it (pushed 0.1) where a rocket detonates | 88 |
| **Disruptor** (Pure only) | Hurts nobody. Launch 500 km/h, then the authored `speed`. Homes if the craft had a lock at 1.0 chord/s. Hit test: swept cylinder radius 6.0, ends slack 1.0. Life 10 s. A hit sets one of eight control effects on the victim for an authored time | 82 to 86 |

A Disruptor hit sets one of eight control effects. Five rewrite the craft's
inputs (no thrust, no airbrakes, reversed yaw, blended yaw, an autopilot at a
literal thrust scale). The other three (a random yaw wobble, a hover damping
term scaled by `0.2`, a camera effect) are state only in this engine: set,
timed and hashed, but not applied.

## Per-title differences

Source tags: **M** measured on that title, **I:X** inherited from X and checked
at most by reading, **C** chosen by this project, **A** absent.

| Axis | Pulse (PSP, PS2) | Pure | HD / Fury | 2048 | Omega |
| --- | --- | --- | --- | --- | --- |
| Race table | `WeaponStats_Race.xml`, M | `weaponstats.xml`, M | `WeaponStats_Race.xml`, M | `_2048` copy, C | HD-named copy, C |
| Eliminator table | M | none (uses race table) | M | C | C |
| Per-class speed attributes | M | one `speed`, M | M | I:HD plus `superphantomspeed` | I:HD |
| LeachBeam drain | every tick, M | no LeachBeam | once per ball trip, M | per trip, I:HD | per trip, I:HD |
| LeachBeams alive at once | 1, M (this engine runs 8) | n/a | 1, M (this engine runs 8) | 8, M | 8, M |
| Quake hit gate | smoothed wave height, M | same gate, eye level held, C | I:Pulse | I:Pulse | I:Pulse |
| Cannon | M | A | I:Pulse | I:Pulse, unread | I:Pulse, unread |
| Plasma ball body | A | A | M | unread | unread |
| Autopilot absorb cancels | M | none, M | M | I:HD | I:HD |
| Pickup draw | Pulse's, M | Pure's odds, Pulse's draw | HD's odds, Pulse's draw | 2048's odds, Pulse's draw | Pulse's draw; a fifth class is unread |

Where the original disagrees with itself across titles, the HD LeachBeam is the
sharp case: Pulse's per-tick drain applied with HD's damage figure empties a
shield in about 100 ticks, so each title runs its own drain law.

## Where this engine differs from the original

Chosen, not measured:

- Blast damage is full inside `blastradius` with no falloff, and a craft's hull is
  a sphere wider than the hull on two axes.
- A mine does not move, and a craft entering `trigger_radius` sets it off
  (the attribute is authored; the code that spends it was not found).
- The Shield refuses damage outright.
- Eight LeachBeams are allowed on Pulse and HD.
- Launch offsets are the nose for Missile and Cannon, not the model's
  attachment points.
- Fixed 60 Hz stepping, so the Repulser's five and two control points per
  update become per tick.
- The pad draw's random sequence, and its bounded retry for the no-repeat rule.

## Open

- The Missile's lock gap rule is read at 80; its purpose is inferred.
- Plasma's muzzle distance is measured on one craft and class.
- The Shield's consumer, and `damageradius`, have no reader in the original.
- Pure and HD Cannon, Mine and Repulser laws are inherited by reading, not
  measured, per [weapon lineage](../gameplay/weapon-lineage.md).

## Evidence

- Pickups and effects: [pickups](../gameplay/pickups.md),
  [weapon lineage](../gameplay/weapon-lineage.md),
  [projectile floor](../gameplay/projectile-floor.md)
- Tables: [weapon stats](../formats/weapon-stats.md)
- Ghidra: [weapon fire](../ghidra/functions/psp-pulse-usa/weapon-fire.md),
  [rocket visuals](../ghidra/functions/psp-pulse-usa/rocket-visuals.md),
  [missile](../ghidra/functions/psp-pulse-usa/missile.md),
  [plasma](../ghidra/functions/psp-pulse-usa/plasma.md),
  [mine](../ghidra/functions/psp-pulse-usa/mine.md),
  [cannon, quake, leachbeam](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md),
  [repulser](../ghidra/functions/psp-pulse-usa/repulser.md),
  [shuriken](../ghidra/functions/psp-pulse-usa/shuriken.md),
  [Pure weapons](../ghidra/functions/psp-pure-usa/weapons.md),
  [shield pickup](../ghidra/functions/psp-pulse-usa/shield-pickup.md),
  [autopilot](../ghidra/functions/psp-pulse-usa/autopilot.md)
- Code: `crates/weapons/src` (`pickup`, `projectile/*`, `slowdown`, `disruption`),
  `crates/physics/src/slowdown.rs`
