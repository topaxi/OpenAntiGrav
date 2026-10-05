//! The per-weapon `<Stats>` blocks: one type per schema the file authors.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`, along the seam the module already had - the
//! types describe *what a weapon is*, and everything left in the parent reads
//! a document into them. A move, with no behaviour change: every type is
//! re-exported from [`super`], so `weapons::RocketStats` and friends still
//! resolve and no call site moved.
//!
//! What is decoded here and what is deliberately not is the parent module's
//! rule, unchanged: an attribute earns a field when something reads it.

/// A weapon whose whole schema is `absorb` and `time`.
///
/// Turbo, Shield and Autopilot. Their three parsers are byte-identical but for
/// the offsets they write.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Simple {
    /// Energy paid back for absorbing this weapon rather than firing it.
    pub absorb: f32,
    /// How long it runs for, in seconds.
    pub time: f32,
}

/// The Rocket's `<Stats>`, less the two attributes nothing consumes.
///
/// # Why this one is decoded and the other nine projectile weapons are not
///
/// The module's rule is that an attribute is decoded when something reads it,
/// and this is the first weapon with a projectile behind it: see
/// `crates/weapons/src/projectile.rs`. The nine still undecoded need a lock, a
/// beam or track deformation, and none of those exists.
///
/// **All eleven are now decoded.** `slowdown_time` was the last one out, and it
/// came in on 2026-09-06 when the slowdown mechanic's law was recovered from the
/// binary - see the field's own docs below.
///
/// # `spread` is the fan angle of three rockets, and that is recovered
///
/// `Weapon_FireRocket` (`0x0886e104`) spawns one rocket through the craft's own
/// matrix, one through it rotated by `+spread` and one by `-spread` - three
/// calls to one spawn helper in a single invocation, with no timer between
/// them. So a Rocket fires **three at once**, and `spread` is the half-angle of
/// the fan, in radians. Confidence 88; see
/// `docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`.
///
/// The attribute's absence from the Missile - which carries the rocket's
/// `<Stats>` less `spread`, plus its lock distances - reads the right way round
/// once this is known: a homing weapon has no use for a launch fan.
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
    /// **Decoded 2026-09-06, and the recovery of its consumer is what earns it
    /// a field.** Until then it was the one attribute of this block left out,
    /// because the module only decodes what something reads. The law, read out
    /// of the PSP executable:
    ///
    /// 1. An impact adds this figure to the victim's pending slot
    ///    (`entity+0x130`).
    /// 2. Once per tick the victim's own update drains that slot into a timer
    ///    (`craft+0x2e0`) and **clamps the total to
    ///    [`WeaponStats::slowdown_limit`]** - so the global is a ceiling on
    ///    *seconds of slowdown outstanding*, not a speed floor.
    /// 3. While that timer is positive the victim gets **no engine thrust, a
    ///    zeroed throttle state and no lateral grip**, and its hover target
    ///    height is lowered. The timer decays by `dt` each tick.
    ///
    /// `Ship_AddSlowdown` (`0x08848690`) is step 2, fifteen instructions with
    /// one caller. See
    /// `docs/ghidra/functions/psp-pulse-usa/engine.md`.
    ///
    /// **Required, not defaulted**, and that is checked against data rather
    /// than assumed: all four tables that reach this parser author it on every
    /// decoded block *that authors it at all* - Pulse's race and Eliminator
    /// files on the USA and EU pressings, and Pure's `weaponstats.xml` on both.
    /// A silent zero here is a weapon that slows nobody, which is the failure
    /// the module's no-defaults rule exists to prevent.
    ///
    /// **[`LeachBeamStats`] is the one block that does not author it**, on any
    /// of those four tables, which is why that struct has no such field and
    /// why this sentence carries a qualification it did not carry before
    /// 2026-09-07. The claim "every weapon authors `slowdown_time`" was true of
    /// the blocks decoded until then and is not true of the file.
    pub slowdown_time: f32,
    /// Added to the class's own speed at launch.
    pub launch_speed: f32,
    /// Half the fan angle of the three-rocket spread, **in radians**.
    ///
    /// `Weapon_FireRocket` (`0x0886e104`) fires one rocket straight ahead, one
    /// rotated by `+spread` and one by `-spread`. The radians reading comes from
    /// the `vcst_s(5)` (`2/pi`) it multiplies by before `vcos_s`/`vsin_s`, which
    /// is exactly the radians-to-quarter-turns conversion Allegrex's
    /// trigonometric instructions need. See
    /// `docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`.
    pub spread: f32,
    /// `venomspeed`, `flashspeed`, `rapierspeed`, `phantomspeed`, in
    /// [`crate::handling::SpeedClass::ALL`]'s order so the class indexes it.
    ///
    /// Not public, because the index *is* the meaning: read it through
    /// [`RocketStats::speed_for`], which cannot get the order wrong.
    pub(super) speeds: [f32; 4],
    /// Whether the file authored **one** `speed` for every class rather than
    /// four named ones.
    ///
    /// Recorded rather than inferred: `class_speeds` knows which of the two
    /// dialects it read, and "all four entries happen to be equal" is a value
    /// coincidence, not the same claim. It is what lets
    /// [`Self::speed_for_named`] answer for a rung outside
    /// [`crate::handling::SpeedClass`] - Pure's `VECTOR` - without borrowing
    /// another rung's number.
    pub(super) class_independent: bool,
}

impl RocketStats {
    /// How fast a rocket flies in one speed class.
    ///
    /// **A per-class projectile speed is the file's own design**, not a scaling
    /// this engine applies: all four are authored separately, which is what a
    /// weapon that has to stay catchable in Phantom and threatening in Venom
    /// needs.
    #[must_use]
    pub fn speed_for(&self, class: crate::handling::SpeedClass) -> f32 {
        self.speeds[class as usize]
    }

    /// The same, for a rung named the way the document spells it.
    ///
    /// `None` when this file cannot answer: a rung outside the four, in a file
    /// that authored a speed *per* class. There is no honest number for that
    /// case - see [`Self::class_independent`].
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
/// # The offsets are read, not guessed
///
/// `WeaponStats_ParseMissile` (`0x0880c31c`) is the same shape as the Rocket's
/// parser - match an attribute name, `swc1` the float at a fixed offset - and its
/// twelve stores land in the per-speed-class stats struct immediately after the
/// Rocket's eleven:
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
/// Confidence 90. See `docs/ghidra/functions/psp-pulse-usa/missile.md`.
///
/// `slowdown_time` at `+0x5c` is **read off the shipped instruction stream, not
/// only off the parser**: the missile's impact bookkeeping (`0x08869054`) does
/// `victim->0x130 += stats->0x5c` on exactly that offset, three instructions
/// after `victim->0x120 += stats->0x30` (`damage`). That is the same accumulator
/// the Mine's blast and the Quake's wave feed, and the whole law is on
/// `docs/ghidra/functions/psp-pulse-usa/engine.md`.
///
/// # A separate struct rather than a widened [`RocketStats`]
///
/// The original parses the two with two different functions into two different
/// sub-blocks, and the two weapons differ in a way that is not a superset
/// relation: the Rocket has `spread` and no lock, the Missile a lock and no
/// `spread`. Folding them together would mean an `Option` per field and a struct
/// that cannot say which weapon it describes.
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
    /// Seconds of slowdown this weapon charges a craft it hits.
    ///
    /// See [`RocketStats::slowdown_time`] for the law it feeds and its
    /// evidence; [`WeaponStats::slowdown_limit`] caps the running total.
    pub slowdown_time: f32,
    /// Added to the *firing craft's own speed* at launch - see
    /// `oag_weapons::projectile::launch_missile`, which is where that
    /// distinction is argued and where the recovered arithmetic lives.
    pub launch_speed: f32,
    /// How far ahead a target must be before it can be locked.
    ///
    /// **Measured along the firer's forward axis, not as a straight-line
    /// range** - `Ship_AcquireLock` (`0x08844784`) compares
    /// `dot(target - origin, forward)` against this pair. A craft alongside is
    /// therefore near zero on this axis however far away it is.
    pub lock_min_dist: f32,
    /// The far end of that same longitudinal window.
    pub lock_max_dist: f32,
    /// `venomspeed`, `flashspeed`, `rapierspeed`, `phantomspeed`, in
    /// [`crate::handling::SpeedClass::ALL`]'s order so the class indexes it.
    ///
    /// Not public for the reason [`RocketStats::speeds`] is: the index *is* the
    /// meaning. Read it through [`MissileStats::speed_for`].
    pub(super) speeds: [f32; 4],
    /// Whether the file authored **one** `speed` for every class rather than
    /// four named ones.
    ///
    /// Recorded rather than inferred: `class_speeds` knows which of the two
    /// dialects it read, and "all four entries happen to be equal" is a value
    /// coincidence, not the same claim. It is what lets
    /// [`Self::speed_for_named`] answer for a rung outside
    /// [`crate::handling::SpeedClass`] - Pure's `VECTOR` - without borrowing
    /// another rung's number.
    pub(super) class_independent: bool,
}

impl MissileStats {
    /// The speed a missile settles at in one speed class, in km/h.
    ///
    /// **The unit matters and it is not units per second.** A missile does not
    /// fly at this from launch either: `Missile_SpeedNow` (`0x0885a038`) ramps
    /// linearly from the launch speed to this over one second. See
    /// `oag_weapons::projectile::missile_speed_kmh`.
    #[must_use]
    pub fn speed_for(&self, class: crate::handling::SpeedClass) -> f32 {
        self.speeds[class as usize]
    }

    /// The same, for a rung named the way the document spells it.
    ///
    /// `None` when this file cannot answer: a rung outside the four, in a file
    /// that authored a speed *per* class. There is no honest number for that
    /// case - see [`Self::class_independent`].
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
/// # The offsets are read, and they close a gap the Mine's page left open
///
/// `WeaponStats_ParsePlasma` (`0x0880cc2c`) is the same shape as the Rocket's
/// and the Mine's - match an attribute name, store the parsed float at a fixed
/// offset. Eleven attributes, eleven offsets, confidence **92**:
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
/// **That block lands exactly where the Mine's page predicted it would.**
/// `docs/ghidra/functions/psp-pulse-usa/mine.md` derived the layout of the
/// `0xa4` unread bytes between the Missile's block and the Mine's from nothing
/// but attribute counts - "Plasma's eleven" at four bytes each - and put the
/// Plasma at `+0x9c`..`+0xc4`. Measuring it there turns that arithmetic from a
/// supported claim into a checked one. See
/// `docs/ghidra/functions/psp-pulse-usa/plasma.md`.
///
/// # One shot, not three, and that is read rather than inferred
///
/// The absence of `spread` says it and so does the handler:
/// `Weapon_FirePlasma` (`0x0886a868`), the bit-`0x4` handler, takes one pool
/// slot, calls `Plasma_Init` once and clears its own request bit in the same
/// breath. There is no fan, no round counter and no reload timer - the three
/// shapes the Rocket and the Mine respectively have.
///
/// # `charge_time` is authored and is **not** a field here
///
/// The file authors `charge_time="3"` and this build spends it nowhere, so by
/// the module's own rule it gets no field - the same treatment
/// [`BombStats`] gives `damageradius`.
///
/// **This is the one open question on the weapon, and it is open in an
/// uncomfortable direction.** `Weapon_FirePlasma` spawns immediately;
/// `Plasma_Update` (`0x0885c6cc`) reads `+0x54` as a plain age and gates
/// nothing on it; `Ship_FireHeldWeapon` (`0x08844ae8`) calls
/// `Weapon_RequestFire` with no timer in front of it. Three functions on the
/// press-to-flight path, and none of them holds a shot back. A maintainer who
/// plays Pulse, asked cold, says the Plasma *does* wind up before it fires -
/// so the consumer exists and has not been found, rather than the attribute
/// being vestigial. Firing instantly is what the read code does and is what
/// this build does; see `docs/ghidra/functions/psp-pulse-usa/plasma.md`.
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
    /// The largest of any weapon on both shipped tables, which is what a
    /// weapon that fires one bolt where the Rocket fires three needs.
    pub damage: f32,
    /// Seconds of slowdown this weapon charges a craft it hits.
    ///
    /// See [`RocketStats::slowdown_time`] for the law it feeds and its
    /// evidence; [`WeaponStats::slowdown_limit`] caps the running total.
    pub slowdown_time: f32,
    /// Added to the class's own speed at launch.
    pub launch_speed: f32,
    /// `venomspeed`, `flashspeed`, `rapierspeed`, `phantomspeed`, in
    /// [`crate::handling::SpeedClass::ALL`]'s order so the class indexes it.
    ///
    /// Not public for the reason [`RocketStats::speeds`] is: the index *is* the
    /// meaning. Read it through [`PlasmaStats::speed_for`].
    pub(super) speeds: [f32; 4],
    /// Whether the file authored **one** `speed` for every class rather than
    /// four named ones.
    ///
    /// Recorded rather than inferred: `class_speeds` knows which of the two
    /// dialects it read, and "all four entries happen to be equal" is a value
    /// coincidence, not the same claim. It is what lets
    /// [`Self::speed_for_named`] answer for a rung outside
    /// [`crate::handling::SpeedClass`] - Pure's `VECTOR` - without borrowing
    /// another rung's number.
    pub(super) class_independent: bool,
}

impl PlasmaStats {
    /// How fast a plasma bolt flies in one speed class, in km/h.
    ///
    /// The unit is the Rocket's, for the Rocket's reason - see
    /// [`RocketStats::speed_for`] and `oag_weapons::projectile::plasma::launch`.
    #[must_use]
    pub fn speed_for(&self, class: crate::handling::SpeedClass) -> f32 {
        self.speeds[class as usize]
    }

    /// The same, for a rung named the way the document spells it.
    ///
    /// `None` when this file cannot answer: a rung outside the four, in a file
    /// that authored a speed *per* class. There is no honest number for that
    /// case - see [`Self::class_independent`].
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
/// # The offsets are read, and they close the run the Mine's page opened
///
/// `WeaponStats_ParseShuriken` (`0x0880d790`), the same shape as every other
/// parser here. **Thirteen** attributes at `+0x144`..`+0x174`, confidence
/// **92**, and that is exactly where
/// `docs/ghidra/functions/psp-pulse-usa/mine.md`'s attribute-count arithmetic
/// puts them - the fourth measured anchor in the struct's unread middle, after
/// Turbo's `+0x84`, Shield's `+0x8c` and the Plasma's `+0x9c`. See
/// `docs/ghidra/functions/psp-pulse-usa/shuriken.md` for the full table.
///
/// # One blade, thrown twenty degrees off the nose
///
/// No `spread`, and `Weapon_FireShuriken` (`0x08870240`) spawns once - the same
/// pair of arguments that says the Plasma fires one. What it does instead of a
/// fan is **pick a side**: `Shuriken_Init` (`0x08877280`) rotates the launch
/// basis by a literal `0.349066` radians - `20.000` degrees - or by its exact
/// negation, chosen by a `rand() > 0.5` the fire handler draws. See
/// [`crate::weapons::ShurikenStats::speed_for`]'s neighbour,
/// `oag_weapons::projectile::shuriken::launch`.
///
/// # Two of the thirteen are authored and decoded nowhere
///
/// `rhicochetdamage` and `rhicochetForce` are the only *second* damage and
/// force any weapon authors, and nothing read says when they are spent - a
/// glancing hit off a craft is the obvious guess and a guess is what it would
/// be.
///
/// A fourth is decoded but only half understood: `fuse` is read
/// **here** but its *meaning* is this engine's reading: `Shuriken_Update`
/// (`0x08877bdc`) counts `+0x48` up every tick - the offset the Mine's fuse
/// lives at - and the pool teardown that would read it was not followed, so
/// whether a blade that times out detonates or is simply reaped is unread.
/// This build reaps it silently, which is what the Rocket's own pool does.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ShurikenStats {
    /// Energy paid back for absorbing it rather than throwing it.
    pub absorb: f32,
    /// The impulse the blast pushes a craft with.
    ///
    /// `blastForce` in the file, capitalised where the Rocket's is not. The
    /// parser's comparison is case-insensitive, so this is the file's spelling
    /// rather than a correction.
    pub blastforce: f32,
    /// How far from the impact the blast reaches.
    pub blastradius: f32,
    /// Energy the blast costs a craft inside that radius.
    ///
    /// `blastdamage`, **not** `rhicochetdamage` - see the type's own docs for
    /// the second pair and why it stays undecoded.
    pub blastdamage: f32,
    /// Seconds of slowdown this weapon charges a craft it hits.
    ///
    /// See [`RocketStats::slowdown_time`] for the law it feeds and its
    /// evidence; [`WeaponStats::slowdown_limit`] caps the running total.
    pub slowdown_time: f32,
    /// Added to the throwing craft's own speed at launch.
    pub launch_speed: f32,
    /// How long a blade lives, in seconds.
    ///
    /// Authored at `2` on both shipped tables. See the type's own docs: the
    /// attribute is read, what running out *does* is this engine's reading.
    pub fuse: f32,
    /// `venomspeed`, `flashspeed`, `rapierspeed`, `phantomspeed`, in
    /// [`crate::handling::SpeedClass::ALL`]'s order so the class indexes it.
    ///
    /// Not public for the reason [`RocketStats::speeds`] is: the index *is* the
    /// meaning. Read it through [`ShurikenStats::speed_for`].
    pub(super) speeds: [f32; 4],
    /// Whether the file authored **one** `speed` for every class rather than
    /// four named ones.
    ///
    /// Recorded rather than inferred: `class_speeds` knows which of the two
    /// dialects it read, and "all four entries happen to be equal" is a value
    /// coincidence, not the same claim. It is what lets
    /// [`Self::speed_for_named`] answer for a rung outside
    /// [`crate::handling::SpeedClass`] - Pure's `VECTOR` - without borrowing
    /// another rung's number.
    pub(super) class_independent: bool,
}

impl ShurikenStats {
    /// How fast a blade flies in one speed class, in km/h.
    ///
    /// **Added to the throwing craft's own speed, which is measured rather than
    /// chosen.** `Weapon_FireShuriken` hands `Shuriken_Init` the craft's speed
    /// multiplied by `3.6` - that is, in km/h - and the constructor computes
    /// `(craft_kmh + authored) / 3.6` for the launch velocity. That settles for
    /// this weapon the open question
    /// `oag_weapons::projectile::rocket::launch` records for the Rocket, and it
    /// is a second, independent statement that the authored speeds are km/h.
    #[must_use]
    pub fn speed_for(&self, class: crate::handling::SpeedClass) -> f32 {
        self.speeds[class as usize]
    }

    /// The same, for a rung named the way the document spells it.
    ///
    /// `None` when this file cannot answer: a rung outside the four, in a file
    /// that authored a speed *per* class. There is no honest number for that
    /// case - see [`Self::class_independent`].
    #[must_use]
    pub fn speed_for_named(&self, name: &str) -> Option<f32> {
        if let Some(class) = crate::handling::SpeedClass::from_name(name) {
            return Some(self.speeds[class as usize]);
        }
        self.class_independent.then(|| self.speeds[0])
    }
}

/// The Mine's `<Stats>`: seven attributes, and none of them a speed.
///
/// # The offsets are read, not guessed
///
/// `WeaponStats_ParseMine` (`0x0880d124`) is the same shape as the Rocket's and
/// the Missile's parsers - match an attribute name, `swc1` the float at a fixed
/// offset - and its seven stores land immediately after the Bomb's eight:
///
/// | Offset | Attribute | | Offset | Attribute |
/// | --- | --- | --- | --- | --- |
/// | `+0xe8` | `damage` | | `+0xf8` | `absorb` |
/// | `+0xec` | `blastradius` | | `+0xfc` | `slowdown_time` |
/// | `+0xf0` | `blastforce` | | `+0x100` | `trigger_radius` |
/// | `+0xf4` | `timetodie` | | | |
///
/// Confidence 92. See `docs/ghidra/functions/psp-pulse-usa/mine.md`, which is
/// also the page that corrects *which weapon* the drop handler belongs to.
///
/// # No speed, and that is the shape of the weapon
///
/// The Rocket, the Missile, the Plasma and the Shuriken each author four
/// per-class speeds; the Mine authors none. It is not launched at a speed - it
/// inherits the firing craft's velocity and a random scatter direction, and
/// then sits there.
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
    /// Seconds of slowdown this weapon charges a craft it hits.
    ///
    /// See [`RocketStats::slowdown_time`] for the law it feeds and its
    /// evidence; [`WeaponStats::slowdown_limit`] caps the running total.
    pub slowdown_time: f32,
    /// How long a dropped mine lives before it expires, in seconds.
    ///
    /// The attribute is spelled `timetodie`, and `Mine_Init` (`0x08859ac8`)
    /// loads it straight into the entity's own countdown - which is what
    /// identifies the offset as this weapon's rather than another's.
    pub timetodie: f32,
    /// How close a craft must come before the mine goes off.
    ///
    /// Distinct from [`Self::blastradius`], and smaller in both shipped tables:
    /// a mine is tripped inside `trigger_radius` and hurts everything inside
    /// `blastradius`.
    pub trigger_radius: f32,
}

/// The Cannon's `<Stats>`: five attributes, and none of them a speed.
///
/// # No speed at all, and that is the schema's own shape
///
/// Every other projectile weapon here authors at least one speed - four per
/// class for the Rocket, the Missile, the Plasma and the Shuriken, none for
/// the Mine and the Bomb because neither is launched. The Cannon has
/// neither shape: `Weapon_FireCannon` (`0x088577ac`) reads
/// `craft->entity->body->speed` - the firing craft's own current speed, not a
/// class figure - and `Cannon_Init` (`0x088648ec`) adds a *base* speed this
/// schema does not carry at all (`func_0x00060af4`, unread). See
/// `oag_weapons::projectile::cannon::BASE_SPEED_KMH`, this engine's own
/// stand-in for that base, and
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md` for the
/// whole reading.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CannonStats {
    /// Energy paid back for absorbing it rather than firing it.
    pub absorb: f32,
    /// How many rounds a picked-up Cannon holds before the slot empties
    /// itself.
    ///
    /// `craft+0x154` starts here and counts down one per round; its low bit
    /// is also what picks which of the two muzzles the next round leaves
    /// from - see `Weapon_FireCannon`'s own reading on the evidence page.
    pub rounds: f32,
    /// Seconds between rounds - **the reciprocal of the `rate` attribute,
    /// because the original's own parser stores it that way.**
    ///
    /// `WeaponStats_ParseCannon` (`0x0880c774`) reads the attribute and
    /// stores `1.0 / value` at `stats+0x78`, and `stats+0x78` is exactly the
    /// field `Cannon_UpdateReload` (`0x0883f424`) adds to `craft+0x158` on
    /// the branch that just fired. So the file authors rounds **per second**
    /// and this field carries seconds per round.
    ///
    /// **An earlier revision read it literally and said so at length**, on
    /// the grounds that the evidence page showed no `1.0 / rate` division.
    /// The page had not read the parser. The consequence shipped and a player
    /// found it: both tables author `rate="20"`, so a literal reading gave
    /// one round every twenty seconds against a magazine of thirty - a
    /// weapon that, held down for a whole race, fires about twenty times and
    /// reads as broken. The reciprocal gives twenty rounds a second and
    /// empties a thirty-round magazine in a second and a half, which is what
    /// a cannon is. See `oag_weapons::projectile::cannon` and
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
    pub rate: f32,
    /// Energy one round costs a craft it hits directly.
    ///
    /// **Direct-hit only, and that is the schema's own shape rather than a
    /// choice.** This is the only projectile weapon whose block authors
    /// neither `blastforce` nor `blastradius`, so a round that hits a wall
    /// costs nobody anything and one that hits a craft costs that craft
    /// alone - there is no splash to sweep for.
    pub damage_per_bullet: f32,
    /// Seconds of slowdown this weapon charges a craft it hits.
    ///
    /// See [`RocketStats::slowdown_time`] for the law it feeds and its
    /// evidence; [`WeaponStats::slowdown_limit`] caps the running
    /// total.
    pub slowdown_time: f32,
}

/// The Quake's `<Stats>`: four attributes, and no speed at all - the wave's
/// own travel rate is not authored, see
/// `oag_weapons::projectile::quake::SPEED_UNITS_PER_SECOND`.
///
/// `WeaponStats_ParseQuake` (`0x0880c60c`), offsets `+0x60`..`+0x6c` measured
/// off the parser itself. See
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md` for the
/// whole reading: `Weapon_FireQuake` copies `damage`/`radius`/`slowdown_time`
/// onto the *firing craft*, not onto the wave instance, the same shape the
/// Repulser's own fields already have - this crate's port keeps them on the
/// wave instead, since only one Quake instance ever exists at once and there
/// is nothing to gain by round-tripping through the craft.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct QuakeStats {
    /// Energy paid back for absorbing it rather than firing it.
    pub absorb: f32,
    /// Energy the wave costs a craft it passes over.
    pub damage: f32,
    /// **The wave's own hit radius, not a blast radius.** Nothing in the
    /// fire handler, `Quake_Init` or `Quake_Update` reads this attribute -
    /// `radius` has exactly one candidate consumer anywhere in the chain,
    /// the proximity test `FUN_088418e0`'s wave branch runs every craft
    /// every frame to decide whether `entity+0x860 & 0x40`'s latch sets. See
    /// that page's "The latch setter, found 2026-09-07" section: the
    /// original's own gate is a smoothed intensity value against a flat
    /// `0.1` threshold, fed by two engine-side helpers
    /// (`Quake_ProximityToCraft`, `Quake_SpanIntensityAt`) whose own
    /// constants (`200.0`, `0.1`) are not authored anywhere this project has
    /// found - `radius` being the *only* authored number in the whole
    /// mechanism is why this crate's port spends it as the proximity gate
    /// rather than inventing a second, unauthored one.
    pub radius: f32,
    /// Seconds of slowdown this weapon charges a craft it passes over.
    ///
    /// See [`RocketStats::slowdown_time`] for the law it feeds and its
    /// evidence; [`WeaponStats::slowdown_limit`] caps the running total.
    pub slowdown_time: f32,
}

/// The Repulser's `<Stats>`, less the one attribute nothing consumes.
///
/// Offsets `+0x128`..`+0x140` on the active stats block, read off
/// `WeaponStats_ParseRepulser` (`0x0880d58c`); see
/// `docs/ghidra/functions/psp-pulse-usa/repulser.md`. `blastradius` is authored
/// (`40` on both Pulse tables) and **spent nowhere** in the original, so it is
/// not decoded - the same treatment the Bomb's `damageradius` gets.
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
/// # The Bomb is the Mine one size up, and that is the shipped data's shape
///
/// Six of its eight attributes are the Mine's six, and every one of them is
/// larger on both shipped tables - a bigger charge, a wider blast, a wider trip
/// and a fuse nearly three times as long. The engine treats it the same way for
/// exactly that reason: see [`MineStats`], whose fields these mirror.
///
/// It differs from the Mine in two places and only two:
///
/// - **A press lays one, not a cluster.** `Weapon_FireBomb` (`0x08863a20`)
///   makes a single spawn where `Weapon_DropMines` (`0x088675cc`) reloads a
///   timer and spawns again.
/// - **`damageradius`**, below.
///
/// # `damageradius` is authored and is deliberately not decoded
///
/// The Bomb is the **only** weapon of the thirteen that authors a second
/// radius, and it is smaller than `blastradius` on both shipped tables. The
/// obvious reading is that the blast pushes over the wider circle and hurts over
/// the narrower one - and it is left as a reading, because the one blast path
/// this project has read at instruction level (`Weapon_PostBlastImpulse`,
/// `0x0886794c`) spends `blastradius` for both, and no consumer of a second
/// radius has been found anywhere.
///
/// **Checked again on Pure, 2026-09-15, with the same answer.** Pure's parser
/// stores it at `WeaponStats_Table+0xcc` (`0x08b1786c` on `psp-pure-usa`) and
/// that slot has no cross-reference in the whole program, where the six slots
/// either side of it each resolve to exactly one consumer. Authored, parsed,
/// stored and never read, on both titles.
///
/// So it stays undecoded, for the reason the module docs give at length: a
/// field decoded with no
/// consumer is a field nobody has checked. It is named on
/// `docs/formats/weapon-stats.md` and here, rather than dropped silently, which
/// is the part that matters - the next person to look for it should find it
/// recorded as *known and unspent* rather than as missed.
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
    /// Seconds of slowdown this weapon charges a craft it hits.
    ///
    /// See [`RocketStats::slowdown_time`] for the law it feeds and its
    /// evidence; [`WeaponStats::slowdown_limit`] caps the running total.
    pub slowdown_time: f32,
    /// How long a dropped bomb lives before it goes off on its own, in seconds,
    /// or `None` for a title whose Bomb has no fuse at all.
    ///
    /// Twenty on both shipped Pulse tables against the Mine's seven, which is
    /// the single biggest difference between the two weapons there: a bomb
    /// left on the track is a hazard for most of a lap.
    ///
    /// **`None` on Pure, and `None` means never, not now.** Pure's
    /// `Data\XML\weaponstats.xml` authors no `timetodie` on its Bomb, and that
    /// is the parser's shape rather than a gap in the file:
    /// `WeaponStats_ParseBomb` (`0x08809230` on `psp-pure-usa`) has no branch
    /// for the attribute where `WeaponStats_ParseMine` beside it does, and
    /// `BombPool_Update` (`0x0884e968`) spends the charge's age on the model's
    /// spin and compares it to nothing. A Pure Bomb sits until a craft enters
    /// [`Self::trigger_radius`], for the whole race if none does. Confidence
    /// 86; see `docs/ghidra/functions/psp-pure-usa/weapons.md`. Until
    /// 2026-09-15 the missing attribute cost Pure its Bomb entirely
    /// (`WeaponStats::skipped`), which read as a title with no Bomb rather
    /// than as a Bomb with no fuse.
    pub timetodie: Option<f32>,
    /// How close a craft must come before the bomb goes off.
    ///
    /// On Pure this is the one trigger there is - `Bomb_UpdateTrigger`
    /// (`0x0884ef78`) box-tests then distance-tests every other craft against
    /// the table slot this fills, and nothing else sets the detonate bit in a
    /// single-player race.
    pub trigger_radius: f32,
}

/// The LeachBeam's `<Stats>`, decoded in full.
///
/// # Why this block is decoded at all
///
/// The module's rule is that an attribute earns a field when something reads
/// it. Two things read these now.
///
/// The first is the **lock-on reticle**. `Ship_AcquireLock` (`0x08844784`) is
/// one function serving two weapons: it selects `stats+0x50`/`+0x54` when the
/// held weapon is the Missile and `stats+0x114`/`+0x118` when it is the
/// LeachBeam, and runs the *same* cone, the same along-track screen and the
/// same nearest-by-longitudinal-distance tie-break either way. So the LeachBeam
/// locks - and `HudSight_Bind` (`0x0881b604`) binds it four sight widgets of its
/// own to show it doing so. See
/// `docs/ghidra/functions/psp-pulse-usa/missile.md` and
/// `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`.
///
/// The second, from 2026-09-08, is the **beam itself**:
/// `oag_weapons::projectile::leach_beam` now holds a link open and drains it.
/// The six attributes that had stayed named and unread until then are read now
/// because that consumer exists, and each one's own doc comment below names the
/// function that spends it.
///
/// # The whole block's offsets, measured
///
/// `WeaponStats_ParseLeachBeam` (`0x0880d328`) was decompiled whole on
/// 2026-09-08, one `Xml_AttributeNameIs` arm per attribute, each storing to a
/// distinct offset:
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
/// See `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
///
/// # It authors no `slowdown_time`, and it is the only decoded block that does
/// not
///
/// Every other block this module decodes carries `slowdown_time`, checked
/// against all four shipped tables. This one does not carry it on any of them -
/// Pulse's race and Eliminator files, on the USA and the EU pressing alike - so
/// there is no field for it here and no default standing in for one. A weapon
/// that fastens onto a craft and drains it is not a weapon that charges it a
/// fixed slowdown on impact, so the absence reads as design rather than as an
/// omission; it is recorded because [`RocketStats::slowdown_time`] claims the
/// attribute is universal and, on the blocks this module had decoded until now,
/// it was.
///
/// # Pure authors no LeachBeam at all
///
/// Measured, not assumed: `Data\XML\weaponstats.xml` on `pure-psp-usa.chd`
/// carries no `<Weapon type="LeachBeam">`, and `Data\XML\Arcade_HUD.xml` on the
/// same disc carries no `leachbeam_sight_*` widget. [`WeaponStats::leach_beam`]
/// returns `None` there, which is the same "the file does not author it" the
/// Cannon already gets on that title.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LeachBeamStats {
    /// Energy paid back for absorbing it rather than firing it.
    pub absorb: f32,
    /// How far ahead a target must be before it can be locked.
    ///
    /// **Measured along the firer's forward axis**, exactly as
    /// [`MissileStats::lock_min_dist`] is - the same comparison in the same
    /// function, off a different pair of offsets.
    pub lock_min_dist: f32,
    /// The far end of that same longitudinal window.
    ///
    /// **Shorter than the Missile's on both shipped tables**, which is the
    /// finding this block was parsed for: the LeachBeam does not reach as far
    /// as the Missile does, so its reticle takes a target later. The near
    /// bounds agree. No value is quoted here for the reason
    /// `crates/formats/tests/weapons_ground_truth.rs` gives - the relation is
    /// the claim, and it is what that test asserts.
    pub lock_max_dist: f32,
    /// Energy the victim loses on every tick the link holds.
    ///
    /// Spent by `LeachBeam_DrainRate` (`0x08872edc`), which returns
    /// `damage * multiplier` and hands it to `LeachBeam_Drain` (`0x08866804`)
    /// to add into the victim's `entity+0x120`. That accumulator is drained
    /// once a tick by `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`) into the
    /// ordinary `Ship_Damage` path, so a fired Shield pickup swallows it the
    /// same way it swallows any other hit.
    ///
    /// **Per tick, not per second** - neither the rate function nor the drain
    /// scales by `dt`. See [`Self::energy_multiplier`] for the one-shot the
    /// first tick gets, and `oag_weapons::projectile::leach_beam` for what
    /// that means at this project's fixed 60 Hz against the original's own
    /// variable timestep.
    pub damage: f32,
    /// Energy the *shooter* gains on every tick the link holds.
    ///
    /// The other half of the transfer, and the reason the weapon is called what
    /// it is. `LeachBeam_RepairRate` (`0x08872f18`) is
    /// [`Self::damage`]'s twin, differing only in which offset it reads and
    /// which of the instance's two one-shot flags it consumes; its result goes
    /// into the shooter's `entity+0x128`, which `Ship_ApplyPendingWeaponRepair`
    /// (`0x0883f228`) drains into `Ship_AddShield` (`0x0883ddc8`) - straight
    /// into the shooter's own shield pool, not through any damage path.
    ///
    /// Authored equal to [`Self::damage`] on both shipped Pulse tables, which
    /// makes the transfer conservative; nothing in the executable requires
    /// that, so the two stay separate fields.
    pub repair: f32,
    /// The handling multiplier a craft under the beam is held at.
    ///
    /// **This is why the block authors no `slowdown_time` and the absence is
    /// design rather than omission.** Every other weapon adds *seconds* to the
    /// shared `entity+0x130` slowdown channel `oag_weapons::slowdown` drains.
    /// The LeachBeam does not touch that channel at all: `LeachBeam_Drain`
    /// copies this attribute into the victim's `entity+0x134`, and
    /// `Ship_ApplyPendingWeaponDamage` writes it on into the victim's *handling*
    /// record at `+0x31c` - but only on the branch where the pending weapon kind
    /// is `7`, the LeachBeam's own tag. A factor held for as long as the link
    /// holds, not a duration charged once on impact.
    pub slow_ship_factor: f32,
    /// How far apart the two craft may drift before the link breaks.
    ///
    /// `LeachBeam_UpdatePool` (`0x08866b08`) measures the floor-clamped
    /// distance between the beam's own resolved position and the target's, and
    /// marks the link disconnected past this figure. Distinct from
    /// [`Self::lock_max_dist`], which is the *longitudinal* window the lock is
    /// taken in: a beam reaches further than it can be aimed, so a target that
    /// pulls ahead stays connected for a while after it would no longer be
    /// lockable.
    ///
    /// **The trap this attribute used to carry is now resolved rather than
    /// avoided.** It is authored at the same figure as
    /// `oag_race::sight::DRAW_RANGE`, a *code* literal in
    /// `HudSight_Update` with nothing to do with this weapon, and this block
    /// left `range` undecoded rather than manufacture a finding out of that
    /// coincidence. What settles it is not the value but the parser: the
    /// `range` arm of `WeaponStats_ParseLeachBeam` stores to `stats+0x11c`, and
    /// `stats+0x11c` is the offset `LeachBeam_UpdatePool` compares its distance
    /// against. The coincidence stands and is still a coincidence.
    pub range: f32,
    /// Seconds a fired beam holds before it lets go, whether or not it ever
    /// connected.
    ///
    /// `LeachBeam_Advance` (`0x08873fa0`) counts the instance's age up by `dt`
    /// and returns `age < active_time`; `LeachBeam_UpdatePool` treats a `false`
    /// there as a disconnect, after which a linger window runs before the pool
    /// retires the instance.
    pub active_time: f32,
    /// What the **first** draining tick's transfer is multiplied by, once on
    /// each half.
    ///
    /// Both rate functions carry their own one-shot flag on the instance
    /// (`+0x58` for the drain, `+0x59` for the repair), set by
    /// `LeachBeam_InitLocked` and cleared on first read. So a connection lands
    /// one large transfer and then settles to [`Self::damage`] and
    /// [`Self::repair`] per tick - which is what makes a glancing, immediately
    /// broken link still worth something, and what stops a long hold being the
    /// only way the weapon does anything.
    pub energy_multiplier: f32,
}
