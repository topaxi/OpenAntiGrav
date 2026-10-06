//! The per-weapon `<Stats>` blocks: one type per schema the file authors.
//!
//! Split out of [`super`] under the 1,000-line rule; re-exported from there.
//! The parent's rule holds: an attribute earns a field when something reads it.

/// A weapon whose whole schema is `absorb` and `time`.
///
/// Turbo, Shield and Autopilot.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Simple {
    /// Energy paid back for absorbing this weapon rather than firing it.
    pub absorb: f32,
    /// How long it runs for, in seconds.
    pub time: f32,
}

/// The Rocket's `<Stats>`, less the two attributes nothing consumes.
///
/// `Weapon_FireRocket` (`0x0886e104`) spawns three rockets in one call: one
/// straight, one rotated by `+spread`, one by `-spread`. Confidence 88; see
/// `docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`. The Missile has no
/// `spread` because a homing weapon needs no fan.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RocketStats {
    /// Energy paid back for absorbing it rather than firing it.
    pub absorb: f32,
    /// The impulse the blast pushes a craft with.
    pub blastforce: f32,
    /// How far from the impact the blast reaches.
    pub blastradius: f32,
    /// Energy the blast costs a craft inside that radius.
    pub damage: f32,
    /// Seconds of slowdown this weapon charges a craft it hits.
    ///
    /// The law, from the PSP executable:
    ///
    /// 1. An impact adds this to the victim's pending slot (`entity+0x130`).
    /// 2. Each tick the victim drains that slot into a timer (`craft+0x2e0`)
    ///    and **clamps the total to [`WeaponStats::slowdown_limit`]**, so the
    ///    global caps *seconds outstanding*, not speed.
    /// 3. While the timer is positive the victim gets **no thrust, a zeroed
    ///    throttle state and no lateral grip**, and its hover height drops. The
    ///    timer decays by `dt` per tick.
    ///
    /// `Ship_AddSlowdown` (`0x08848690`) is step 2. See
    /// `docs/ghidra/functions/psp-pulse-usa/engine.md`.
    ///
    /// **Required, not defaulted**: all four tables that reach this parser
    /// (Pulse race and Eliminator, USA and EU; Pure `weaponstats.xml`, both)
    /// author it on every decoded block except [`LeachBeamStats`], which never
    /// does. A silent zero would be a weapon that slows nobody.
    pub slowdown_time: f32,
    /// Added to the class's own speed at launch.
    pub launch_speed: f32,
    /// Half the fan angle of the three-rocket spread, **in radians**.
    ///
    /// `Weapon_FireRocket` multiplies by `vcst_s(5)` (`2/pi`) before
    /// `vcos_s`/`vsin_s`, the radians-to-quarter-turns conversion Allegrex
    /// needs. See `docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`.
    pub spread: f32,
    /// `venomspeed`, `flashspeed`, `rapierspeed`, `phantomspeed`, in
    /// [`crate::handling::SpeedClass::ALL`]'s order.
    ///
    /// Private to the crate: read through [`RocketStats::speed_for`] so the
    /// order cannot be got wrong.
    pub(super) speeds: [f32; 4],
    /// Whether the file authored **one** `speed` for every class rather than
    /// four named ones.
    ///
    /// Recorded, not inferred from four equal values. It lets
    /// [`Self::speed_for_named`] answer for a rung outside
    /// [`crate::handling::SpeedClass`] (Pure's `VECTOR`) without borrowing
    /// another rung's number.
    pub(super) class_independent: bool,
}

impl RocketStats {
    /// How fast a rocket flies in one speed class.
    ///
    /// Four per-class speeds are the file's own design, not engine scaling.
    #[must_use]
    pub fn speed_for(&self, class: crate::handling::SpeedClass) -> f32 {
        self.speeds[class as usize]
    }

    /// The same, for a rung named the way the document spells it.
    ///
    /// `None` for a rung outside the four in a file that authored a speed *per*
    /// class; see [`Self::class_independent`].
    #[must_use]
    pub fn speed_for_named(&self, name: &str) -> Option<f32> {
        if let Some(class) = crate::handling::SpeedClass::from_name(name) {
            return Some(self.speeds[class as usize]);
        }
        self.class_independent.then(|| self.speeds[0])
    }
}

/// The Missile's `<Stats>`: the Rocket's, less `spread`, plus its lock distances.
///
/// `WeaponStats_ParseMissile` (`0x0880c31c`) stores twelve floats right after the
/// Rocket's eleven. Confidence 90; see
/// `docs/ghidra/functions/psp-pulse-usa/missile.md`.
///
/// | Offset | Attribute | | Offset | Attribute |
/// | --- | --- | --- | --- | --- |
/// | `+0x30` | `damage` | | `+0x48` | `blastradius` |
/// | `+0x34` | `venomspeed` | | `+0x4c` | `blastforce` |
/// | `+0x38` | `flashspeed` | | `+0x50` | `lock_min_dist` |
/// | `+0x3c` | `rapierspeed` | | `+0x54` | `lock_max_dist` |
/// | `+0x40` | `phantomspeed` | | `+0x58` | `absorb` |
/// | `+0x44` | `launchspeed` | | `+0x5c` | `slowdown_time` |
///
/// `slowdown_time` is also confirmed off the instruction stream: the impact code
/// at `0x08869054` does `victim->0x130 += stats->0x5c`, three instructions after
/// `victim->0x120 += stats->0x30` (`damage`). See
/// `docs/ghidra/functions/psp-pulse-usa/engine.md`.
///
/// A separate struct, not a widened [`RocketStats`]: the original parses them
/// with two functions, and neither is a superset of the other.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MissileStats {
    /// Energy paid back for absorbing it rather than firing it.
    pub absorb: f32,
    /// The impulse the blast pushes a craft with.
    pub blastforce: f32,
    /// How far from the impact the blast reaches.
    pub blastradius: f32,
    /// Energy the blast costs a craft inside that radius.
    pub damage: f32,
    /// Seconds of slowdown charged to a craft it hits; see
    /// [`RocketStats::slowdown_time`].
    pub slowdown_time: f32,
    /// Added to the *firing craft's own speed* at launch; see
    /// `oag_weapons::projectile::launch_missile`.
    pub launch_speed: f32,
    /// How far ahead a target must be before it can be locked.
    ///
    /// **Measured along the firer's forward axis, not as a range**:
    /// `Ship_AcquireLock` (`0x08844784`) compares `dot(target - origin,
    /// forward)` against this pair, so a craft alongside reads near zero.
    pub lock_min_dist: f32,
    /// The far end of that same longitudinal window.
    pub lock_max_dist: f32,
    /// Per-class speeds in [`crate::handling::SpeedClass::ALL`]'s order; read
    /// through [`MissileStats::speed_for`].
    pub(super) speeds: [f32; 4],
    /// One authored `speed` for every class; see
    /// [`RocketStats::class_independent`].
    pub(super) class_independent: bool,
}

impl MissileStats {
    /// The speed a missile settles at in one speed class, in km/h.
    ///
    /// Not units per second. `Missile_SpeedNow` (`0x0885a038`) ramps linearly
    /// from the launch speed to this over one second; see
    /// `oag_weapons::projectile::missile_speed_kmh`.
    #[must_use]
    pub fn speed_for(&self, class: crate::handling::SpeedClass) -> f32 {
        self.speeds[class as usize]
    }

    /// The same, for a rung named the way the document spells it; `None` as in
    /// [`RocketStats::speed_for_named`].
    #[must_use]
    pub fn speed_for_named(&self, name: &str) -> Option<f32> {
        if let Some(class) = crate::handling::SpeedClass::from_name(name) {
            return Some(self.speeds[class as usize]);
        }
        self.class_independent.then(|| self.speeds[0])
    }
}

/// The Plasma's `<Stats>`: the Rocket's, less `spread`, plus `charge_time`.
///
/// `WeaponStats_ParsePlasma` (`0x0880cc2c`): eleven attributes, confidence
/// **92**. See `docs/ghidra/functions/psp-pulse-usa/plasma.md`.
///
/// | Offset | Attribute |
/// | --- | --- |
/// | `+0x9c` | `charge_time` |
/// | `+0xa0` | `damage` |
/// | `+0xa4` | `blastradius` |
/// | `+0xa8` | `blastforce` |
/// | `+0xac`..`+0xb8` | `venomspeed`, `flashspeed`, `rapierspeed`, `phantomspeed` |
/// | `+0xbc` | `launchspeed` |
/// | `+0xc0` | `absorb` |
/// | `+0xc4` | `slowdown_time` |
///
/// The block lands where `docs/ghidra/functions/psp-pulse-usa/mine.md` predicted
/// from attribute counts alone, which makes that arithmetic a checked claim.
///
/// One shot, not three: `Weapon_FirePlasma` (`0x0886a868`) takes one pool slot,
/// calls `Plasma_Init` once and clears its request bit. No fan, round counter or
/// reload timer.
///
/// `charge_time="3"` is authored and **not decoded**, as with [`BombStats`]'s
/// `damageradius`. **Open, and uncomfortable:** `Weapon_FirePlasma` spawns at
/// once, `Plasma_Update` (`0x0885c6cc`) reads `+0x54` as a plain age, and
/// `Ship_FireHeldWeapon` (`0x08844ae8`) has no timer before
/// `Weapon_RequestFire`. A maintainer who plays Pulse says the Plasma *does*
/// wind up, so the consumer exists unfound. This build fires instantly, as the
/// read code does.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlasmaStats {
    /// Energy paid back for absorbing it rather than firing it.
    pub absorb: f32,
    /// The impulse the blast pushes a craft with.
    pub blastforce: f32,
    /// How far from the impact the blast reaches.
    pub blastradius: f32,
    /// Energy the blast costs a craft inside that radius.
    ///
    /// The largest of any weapon on both shipped tables.
    pub damage: f32,
    /// Seconds of slowdown charged to a craft it hits; see
    /// [`RocketStats::slowdown_time`].
    pub slowdown_time: f32,
    /// Added to the class's own speed at launch.
    pub launch_speed: f32,
    /// Per-class speeds in [`crate::handling::SpeedClass::ALL`]'s order; read
    /// through [`PlasmaStats::speed_for`].
    pub(super) speeds: [f32; 4],
    /// One authored `speed` for every class; see
    /// [`RocketStats::class_independent`].
    pub(super) class_independent: bool,
}

impl PlasmaStats {
    /// How fast a plasma bolt flies in one speed class, in km/h.
    ///
    /// The Rocket's unit; see [`RocketStats::speed_for`] and
    /// `oag_weapons::projectile::plasma::launch`.
    #[must_use]
    pub fn speed_for(&self, class: crate::handling::SpeedClass) -> f32 {
        self.speeds[class as usize]
    }

    /// The same, for a rung named the way the document spells it; `None` as in
    /// [`RocketStats::speed_for_named`].
    #[must_use]
    pub fn speed_for_named(&self, name: &str) -> Option<f32> {
        if let Some(class) = crate::handling::SpeedClass::from_name(name) {
            return Some(self.speeds[class as usize]);
        }
        self.class_independent.then(|| self.speeds[0])
    }
}

/// The Shuriken's `<Stats>`, less the four attributes nothing consumes.
///
/// `WeaponStats_ParseShuriken` (`0x0880d790`): **thirteen** attributes at
/// `+0x144`..`+0x174`, confidence **92**, where
/// `docs/ghidra/functions/psp-pulse-usa/mine.md` predicted them. Full table in
/// `docs/ghidra/functions/psp-pulse-usa/shuriken.md`.
///
/// One blade, thrown off the nose: `Shuriken_Init` (`0x08877280`) rotates the
/// launch basis by a literal `0.349066` rad (20 degrees) or its negation, picked
/// by a `rand() > 0.5` in `Weapon_FireShuriken` (`0x08870240`). See
/// `oag_weapons::projectile::shuriken::launch`.
///
/// `rhicochetdamage` and `rhicochetForce` are authored and decoded nowhere:
/// nothing read says when they are spent. `fuse` is decoded, but its meaning is
/// this engine's reading: `Shuriken_Update` (`0x08877bdc`) counts `+0x48` up,
/// and the pool teardown that would read it was not followed. This build reaps
/// a timed-out blade silently, as the Rocket's pool does.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ShurikenStats {
    /// Energy paid back for absorbing it rather than throwing it.
    pub absorb: f32,
    /// The impulse the blast pushes a craft with.
    ///
    /// The file spells it `blastForce`; the parser compares case-insensitively.
    pub blastforce: f32,
    /// How far from the impact the blast reaches.
    pub blastradius: f32,
    /// Energy the blast costs a craft inside that radius.
    ///
    /// `blastdamage`, **not** `rhicochetdamage`.
    pub blastdamage: f32,
    /// Seconds of slowdown charged to a craft it hits; see
    /// [`RocketStats::slowdown_time`].
    pub slowdown_time: f32,
    /// Added to the throwing craft's own speed at launch.
    pub launch_speed: f32,
    /// How long a blade lives, in seconds. Authored at `2` on both tables.
    pub fuse: f32,
    /// Per-class speeds in [`crate::handling::SpeedClass::ALL`]'s order; read
    /// through [`ShurikenStats::speed_for`].
    pub(super) speeds: [f32; 4],
    /// One authored `speed` for every class; see
    /// [`RocketStats::class_independent`].
    pub(super) class_independent: bool,
}

impl ShurikenStats {
    /// How fast a blade flies in one speed class, in km/h.
    ///
    /// **Added to the throwing craft's speed, measured rather than chosen.**
    /// `Weapon_FireShuriken` passes `Shuriken_Init` the craft's speed times
    /// `3.6` (km/h) and the constructor computes `(craft_kmh + authored) / 3.6`.
    /// That settles the open question
    /// `oag_weapons::projectile::rocket::launch` records for the Rocket and
    /// independently shows the authored speeds are km/h.
    #[must_use]
    pub fn speed_for(&self, class: crate::handling::SpeedClass) -> f32 {
        self.speeds[class as usize]
    }

    /// The same, for a rung named the way the document spells it; `None` as in
    /// [`RocketStats::speed_for_named`].
    #[must_use]
    pub fn speed_for_named(&self, name: &str) -> Option<f32> {
        if let Some(class) = crate::handling::SpeedClass::from_name(name) {
            return Some(self.speeds[class as usize]);
        }
        self.class_independent.then(|| self.speeds[0])
    }
}

/// The Mine's `<Stats>`: seven attributes, none of them a speed.
///
/// `WeaponStats_ParseMine` (`0x0880d124`) stores seven floats right after the
/// Bomb's eight. Confidence 92; see
/// `docs/ghidra/functions/psp-pulse-usa/mine.md`.
///
/// | Offset | Attribute | | Offset | Attribute |
/// | --- | --- | --- | --- | --- |
/// | `+0xe8` | `damage` | | `+0xf8` | `absorb` |
/// | `+0xec` | `blastradius` | | `+0xfc` | `slowdown_time` |
/// | `+0xf0` | `blastforce` | | `+0x100` | `trigger_radius` |
/// | `+0xf4` | `timetodie` | | | |
///
/// A mine inherits the firing craft's velocity plus a random scatter direction,
/// then sits.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MineStats {
    /// Energy paid back for absorbing it rather than firing it.
    pub absorb: f32,
    /// The impulse the blast pushes a craft with.
    pub blastforce: f32,
    /// How far from the blast a craft still takes damage.
    pub blastradius: f32,
    /// Energy the blast costs a craft inside that radius.
    pub damage: f32,
    /// Seconds of slowdown charged to a craft it hits; see
    /// [`RocketStats::slowdown_time`].
    pub slowdown_time: f32,
    /// How long a dropped mine lives before it expires, in seconds.
    ///
    /// `Mine_Init` (`0x08859ac8`) loads it straight into the entity's countdown.
    pub timetodie: f32,
    /// How close a craft must come before the mine goes off.
    ///
    /// Smaller than [`Self::blastradius`] on both tables: tripped inside this,
    /// hurts everything inside that.
    pub trigger_radius: f32,
}

/// The Cannon's `<Stats>`: five attributes, none of them a speed.
///
/// `Weapon_FireCannon` (`0x088577ac`) reads the firing craft's own current speed
/// (`craft->entity->body->speed`), and `Cannon_Init` (`0x088648ec`) adds a *base*
/// speed this schema does not carry (`func_0x00060af4`, unread). See
/// `oag_weapons::projectile::cannon::BASE_SPEED_KMH`, this engine's stand-in, and
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CannonStats {
    /// Energy paid back for absorbing it rather than firing it.
    pub absorb: f32,
    /// How many rounds a picked-up Cannon holds before the slot empties.
    ///
    /// `craft+0x154` starts here and counts down per round; its low bit picks
    /// which of the two muzzles fires next.
    pub rounds: f32,
    /// Seconds between rounds - **the reciprocal of the `rate` attribute**.
    ///
    /// `WeaponStats_ParseCannon` (`0x0880c774`) stores `1.0 / value` at
    /// `stats+0x78`, which `Cannon_UpdateReload` (`0x0883f424`) adds to
    /// `craft+0x158` after a shot. The file authors rounds per second, this
    /// field carries seconds per round. Both tables author `rate="20"`: a
    /// literal reading once shipped as one round per twenty seconds, which a
    /// player found. See `oag_weapons::projectile::cannon` and
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
    pub rate: f32,
    /// Energy one round costs a craft it hits directly.
    ///
    /// Direct hit only: the block authors neither `blastforce` nor
    /// `blastradius`, so there is no splash.
    pub damage_per_bullet: f32,
    /// Seconds of slowdown charged to a craft it hits; see
    /// [`RocketStats::slowdown_time`].
    pub slowdown_time: f32,
}

/// The Quake's `<Stats>`: four attributes and no speed; the wave's travel rate
/// is not authored, see `oag_weapons::projectile::quake::SPEED_UNITS_PER_SECOND`.
///
/// `WeaponStats_ParseQuake` (`0x0880c60c`), offsets `+0x60`..`+0x6c`. See
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
/// `Weapon_FireQuake` copies `damage`/`radius`/`slowdown_time` onto the *firing
/// craft*; this port keeps them on the wave, since only one Quake exists at once.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct QuakeStats {
    /// Energy paid back for absorbing it rather than firing it.
    pub absorb: f32,
    /// Energy the wave costs a craft it passes over.
    pub damage: f32,
    /// **The wave's own hit radius, not a blast radius.**
    ///
    /// No fire handler, `Quake_Init` or `Quake_Update` reads it. Its one
    /// candidate consumer is the wave branch of `FUN_088418e0`, the proximity
    /// test behind the `entity+0x860 & 0x40` latch (see that page's "The latch
    /// setter, found 2026-09-07"). The original gates on a smoothed intensity
    /// against a flat `0.1`, with constants (`200.0`, `0.1`) in
    /// `Quake_ProximityToCraft` and `Quake_SpanIntensityAt` that are not
    /// authored anywhere found. As the only authored number in the mechanism,
    /// this port spends `radius` as the proximity gate rather than inventing a
    /// second.
    pub radius: f32,
    /// Seconds of slowdown charged to a craft it passes over; see
    /// [`RocketStats::slowdown_time`].
    pub slowdown_time: f32,
}

/// The Repulser's `<Stats>`, less the one attribute nothing consumes.
///
/// Offsets `+0x128`..`+0x140`, from `WeaponStats_ParseRepulser` (`0x0880d58c`);
/// see `docs/ghidra/functions/psp-pulse-usa/repulser.md`. `blastradius` is
/// authored (`40` on both Pulse tables) and spent nowhere, so it is not decoded.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RepulserStats {
    /// Energy paid back for absorbing it rather than firing it.
    pub absorb: f32,
    /// Energy a wave costs each craft it sweeps, once (`+0x128`).
    pub damage: f32,
    /// The shove a wave gives a craft it sweeps, with **no falloff** (`+0x130`).
    pub blastforce: f32,
    /// Seconds of slowdown a wave charges a craft it sweeps (`+0x134`).
    pub slowdown_time: f32,
    /// Seconds from the fire to the waves starting (`+0x13c`).
    pub blast_time: f32,
    /// Seconds the waves run for once they start (`+0x140`).
    pub wave_time: f32,
}

/// The Bomb's `<Stats>`, less the two attributes nothing consumes.
///
/// The Mine one size up: six of its eight attributes are the Mine's, all larger
/// on both tables; see [`MineStats`]. It differs in that a press lays one bomb
/// (`Weapon_FireBomb`, `0x08863a20`) where `Weapon_DropMines` (`0x088675cc`)
/// reloads a timer and spawns again, and in `damageradius`.
///
/// `damageradius` is authored, smaller than `blastradius`, and **deliberately
/// not decoded**: the one blast path read at instruction level
/// (`Weapon_PostBlastImpulse`, `0x0886794c`) spends `blastradius` for both. Pure
/// stores it at `WeaponStats_Table+0xcc` (`0x08b1786c` on `psp-pure-usa`, checked
/// 2026-09-15) and that slot has no cross-reference, where its neighbours each
/// have one consumer. Parsed, stored and never read on both titles; also named
/// on `docs/formats/weapon-stats.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BombStats {
    /// Energy paid back for absorbing it rather than firing it.
    pub absorb: f32,
    /// The impulse the blast pushes a craft with, at the centre.
    pub blastforce: f32,
    /// How far from the blast a craft is still pushed and hurt.
    pub blastradius: f32,
    /// Energy the blast costs a craft inside that radius.
    pub damage: f32,
    /// Seconds of slowdown charged to a craft it hits; see
    /// [`RocketStats::slowdown_time`].
    pub slowdown_time: f32,
    /// How long a dropped bomb lives before it goes off on its own, in seconds.
    ///
    /// Twenty on both Pulse tables against the Mine's seven.
    ///
    /// **`None` on Pure means never, not now.** Pure's `weaponstats.xml` authors
    /// no `timetodie` on its Bomb: `WeaponStats_ParseBomb` (`0x08809230` on
    /// `psp-pure-usa`) has no branch for it, and `BombPool_Update` (`0x0884e968`)
    /// compares the charge's age to nothing. A Pure Bomb sits until a craft
    /// enters [`Self::trigger_radius`]. Confidence 86; see
    /// `docs/ghidra/functions/psp-pure-usa/weapons.md`.
    pub timetodie: Option<f32>,
    /// How close a craft must come before the bomb goes off.
    ///
    /// On Pure this is the only trigger: `Bomb_UpdateTrigger` (`0x0884ef78`)
    /// box-tests then distance-tests every other craft against it.
    pub trigger_radius: f32,
}

/// The LeachBeam's `<Stats>`, decoded in full.
///
/// Two consumers read these. `Ship_AcquireLock` (`0x08844784`) serves both
/// Missile and LeachBeam, selecting `stats+0x50`/`+0x54` or `+0x114`/`+0x118`;
/// `HudSight_Bind` (`0x0881b604`) gives the LeachBeam four sight widgets. See
/// `docs/ghidra/functions/psp-pulse-usa/missile.md` and `lock-sight.md`. And
/// `oag_weapons::projectile::leach_beam` holds a link open and drains it.
///
/// `WeaponStats_ParseLeachBeam` (`0x0880d328`), decompiled whole 2026-09-08; see
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
///
/// | offset | attribute |
/// | --- | --- |
/// | `+0x104` | [`damage`](Self::damage) |
/// | `+0x108` | [`absorb`](Self::absorb) |
/// | `+0x10c` | [`repair`](Self::repair) |
/// | `+0x110` | [`slow_ship_factor`](Self::slow_ship_factor) |
/// | `+0x114` | [`lock_min_dist`](Self::lock_min_dist) |
/// | `+0x118` | [`lock_max_dist`](Self::lock_max_dist) |
/// | `+0x11c` | [`range`](Self::range) |
/// | `+0x120` | [`active_time`](Self::active_time) |
/// | `+0x124` | [`energy_multiplier`](Self::energy_multiplier) |
///
/// It authors no `slowdown_time` on any of the four shipped tables, so there is
/// no field and no default. See [`Self::slow_ship_factor`] for why that is
/// design.
///
/// Pure authors no LeachBeam: its `weaponstats.xml` has no
/// `<Weapon type="LeachBeam">` and `Arcade_HUD.xml` no `leachbeam_sight_*`
/// widget, so [`WeaponStats::leach_beam`] is `None` there.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LeachBeamStats {
    /// Energy paid back for absorbing it rather than firing it.
    pub absorb: f32,
    /// How far ahead a target must be before it can be locked.
    ///
    /// **Measured along the firer's forward axis**, as
    /// [`MissileStats::lock_min_dist`] is.
    pub lock_min_dist: f32,
    /// The far end of that same longitudinal window.
    ///
    /// **Shorter than the Missile's on both tables**; the near bounds agree. No
    /// value is quoted: the relation is the claim, asserted by
    /// `crates/formats/tests/weapons_ground_truth.rs`.
    pub lock_max_dist: f32,
    /// Energy the victim loses on every tick the link holds.
    ///
    /// `LeachBeam_DrainRate` (`0x08872edc`) returns `damage * multiplier` for
    /// `LeachBeam_Drain` (`0x08866804`) to add into the victim's `entity+0x120`,
    /// which `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`) drains each tick into
    /// `Ship_Damage`, so a fired Shield swallows it.
    ///
    /// **Per tick, not per second**: neither function scales by `dt`. See
    /// [`Self::energy_multiplier`] and `oag_weapons::projectile::leach_beam` for
    /// the 60 Hz fixed step against the original's variable one.
    pub damage: f32,
    /// Energy the *shooter* gains on every tick the link holds.
    ///
    /// `LeachBeam_RepairRate` (`0x08872f18`) is [`Self::damage`]'s twin. Its
    /// result goes into `entity+0x128`, which `Ship_ApplyPendingWeaponRepair`
    /// (`0x0883f228`) drains into `Ship_AddShield` (`0x0883ddc8`), not through
    /// any damage path. Authored equal to `damage` on both Pulse tables; nothing
    /// requires it, so they stay separate.
    pub repair: f32,
    /// The handling multiplier a craft under the beam is held at.
    ///
    /// **This is why the block has no `slowdown_time`.** Every other weapon adds
    /// seconds to the `entity+0x130` channel `oag_weapons::slowdown` drains. The
    /// LeachBeam copies this into the victim's `entity+0x134`, and
    /// `Ship_ApplyPendingWeaponDamage` writes it to the handling record at
    /// `+0x31c` only when the pending weapon kind is `7`. A factor held while the
    /// link holds, not a duration charged once.
    pub slow_ship_factor: f32,
    /// How far apart the two craft may drift before the link breaks.
    ///
    /// `LeachBeam_UpdatePool` (`0x08866b08`) compares the floor-clamped distance
    /// between the beam and the target to it. Distinct from
    /// [`Self::lock_max_dist`]: a beam reaches further than it can be aimed.
    ///
    /// Authored at the same figure as `oag_race::sight::DRAW_RANGE`, a code
    /// literal in `HudSight_Update`; that is a coincidence. What settles it is
    /// the parser: the `range` arm stores to `stats+0x11c`, the offset
    /// `LeachBeam_UpdatePool` compares against.
    pub range: f32,
    /// Seconds a fired beam holds before it lets go, connected or not.
    ///
    /// `LeachBeam_Advance` (`0x08873fa0`) counts age up by `dt` and returns
    /// `age < active_time`; `false` is a disconnect, then a linger window runs
    /// before the pool retires the instance.
    pub active_time: f32,
    /// What the **first** draining tick's transfer is multiplied by, once on each
    /// half.
    ///
    /// Each rate function has a one-shot flag on the instance (`+0x58` drain,
    /// `+0x59` repair), set by `LeachBeam_InitLocked` and cleared on first read.
    /// A connection lands one large transfer, then settles to [`Self::damage`]
    /// and [`Self::repair`] per tick.
    pub energy_multiplier: f32,
}
