//! `WeaponStats_*.xml`: the weapon table, the disturber odds and the pickup
//! distribution.
//!
//! Weapons are **data, not code**. [`RACE_ENTRY`] and [`ELIMINATION_ENTRY`]
//! carry the whole system, and `WeaponStats_Parse` (`0x0880db7c`) reads both
//! through one parser, so a mode swaps the table wholesale.
//!
//! ```text
//! <WeaponStats>
//!   <Weapon type="Global">    <Stats slowdown_limit/> </Weapon>
//!   <Weapon type="Turbo">     <Stats absorb time/>    </Weapon>
//!   ...                                                     x14
//!   <DisturberOdds>
//!     <Weapon type="Rocket"> <Odds .../> <Times .../> </Weapon>
//!     ...
//!   </DisturberOdds>
//!   <Pickupodds class="Venom">
//!     <Weapon type="Autopilot"> <Stats ai back front human/> </Weapon>
//!     ...
//!   </Pickupodds>                                            x4
//! </WeaponStats>
//! ```
//!
//! Schema and per-weapon parser addresses: `docs/formats/weapon-stats.md`.
//!
//! # What this decodes
//!
//! Only what a consumer exists for; a field decoded with no consumer is a field
//! nobody has checked.
//!
//! - Turbo, Shield and Autopilot in full (one two-attribute schema).
//! - `absorb` for every weapon: it is what `Ship_Damage`'s absorb branch spends
//!   (`docs/ghidra/functions/psp-pulse-usa/shield.md`).
//! - `<Pickupodds>` in full: it *is* the pickup design, weighted per speed class
//!   and separately for AI, human, front and back of the grid.
//! - `<Global>`'s `slowdown_limit`.
//! - The blocks of Rocket, Missile (lock distances included), Mine, Bomb (less
//!   `damageradius`), Cannon, Plasma, Shuriken, Quake, Repulser and LeachBeam,
//!   each read by a consumer in `oag_weapons::projectile`.
//! - The Disruptor (Pure only, 2026-09-15): a tree of `<Effect>` children, so it
//!   has its own module, [`disruptor`].
//! - The Bomb's `timetodie` is an `Option`: Pure's Bomb authors none and its
//!   pool reads none, so `None` is a charge that sits until tripped. See
//!   [`BombStats::timetodie`].
//!
//! Left undecoded and named on `docs/formats/weapon-stats.md`: the seven
//! disturber effects and the Bomb's second radius (see [`BombStats`]).
//!
//! `slowdown_time` (2026-09-06) is on every decoded block that authors it. The
//! LeachBeam never does, so [`LeachBeamStats`] has no field rather than a zero.
//! See [`RocketStats::slowdown_time`].
//!
//! # Nothing here defaults
//!
//! A required attribute that is missing is an [`Error`], not a zero: a
//! quietly-zero `time` is a shield that expires immediately, which reads as a
//! gameplay bug. A weapon the file does not author at all is not an error;
//! [`WeaponStats::simple`] returns `None`.

use crate::fexml::{self, Node};

/// `Data\XML\WeaponStats_Race.xml`, the ordinary race modes' table.
pub const RACE_ENTRY: &str = r"Data\XML\WeaponStats_Race.xml";

/// `Data\XML\WeaponStats_Elimination.xml`, Eliminator's.
pub const ELIMINATION_ENTRY: &str = r"Data\XML\WeaponStats_Elimination.xml";

/// The thirteen Pulse pickups in the order of the class-name pool at
/// `0x08a78c00`, plus Pure's Disruptor.
///
/// The order is the executable's, since the file's order differs between the two
/// tables. `Ship_Damage`'s `weapon_kind` telemetry buckets may be indexed by it;
/// **not confirmed**, see `docs/ghidra/functions/psp-pulse-usa/shield.md`.
///
/// This is the order of the original's per-class `<Stats>` struct (recovered
/// 2026-08-26), which is why the offsets on [`RocketStats`], [`MissileStats`]
/// and [`MineStats`] check against each other. It is **not** the weapon id at
/// `craft+0x1bc` (agrees at eleven of thirteen; the Mine is 8, the Bomb 9), nor
/// the index `WeaponAiStats_Load` and the incoming-weapon announcement table use
/// (the file's element order, `Cannon` after `Shield`). See
/// `docs/ghidra/functions/psp-pulse-usa/mine.md`. Nothing here reads this index
/// as a weapon id, and nothing should.
///
/// `Disruptor` is last on purpose. Pure's class-name run (`0x08a445d0..0x08a44628`
/// on `psp-pure-usa`) holds it fourth and has no `Cannon`, `LeachBeam`,
/// `Repulser` or `Shuriken`, so no one order fits both discs. Appending keeps
/// every consumer indexed by position (`oag_gameplay::hash`'s discriminant, the
/// per-title HUD tables) unchanged. See `docs/formats/weapon-stats.md`'s Pure
/// dialect section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Weapon {
    Rocket,
    Missile,
    Quake,
    Cannon,
    Turbo,
    Shield,
    Autopilot,
    Plasma,
    Bomb,
    Mine,
    LeachBeam,
    Repulser,
    Shuriken,
    /// Pure's disruption bolt: hurts nobody, hands its victim one of eight
    /// control effects. Authored on no Pulse or HD table.
    Disruptor,
}

impl Weapon {
    /// Every weapon, in the pool's order.
    ///
    /// Fixed-size so a new weapon is a compile error at every enumerating caller.
    pub const ALL: [Self; 14] = [
        Self::Rocket,
        Self::Missile,
        Self::Quake,
        Self::Cannon,
        Self::Turbo,
        Self::Shield,
        Self::Autopilot,
        Self::Plasma,
        Self::Bomb,
        Self::Mine,
        Self::LeachBeam,
        Self::Repulser,
        Self::Shuriken,
        Self::Disruptor,
    ];

    /// The `type` attribute's spelling, exactly as the file has it.
    ///
    /// `LeachBeam` and `Repulser` are the game's spellings, kept as matched.
    #[must_use]
    pub fn as_type(self) -> &'static str {
        match self {
            Self::Rocket => "Rocket",
            Self::Missile => "Missile",
            Self::Quake => "Quake",
            Self::Cannon => "Cannon",
            Self::Turbo => "Turbo",
            Self::Shield => "Shield",
            Self::Autopilot => "Autopilot",
            Self::Plasma => "Plasma",
            Self::Bomb => "Bomb",
            Self::Mine => "Mine",
            Self::LeachBeam => "LeachBeam",
            Self::Repulser => "Repulser",
            Self::Shuriken => "Shuriken",
            Self::Disruptor => "Disruptor",
        }
    }

    /// The weapon a `type` attribute names, or `None` for `Global` and for
    /// anything the pool does not hold.
    #[must_use]
    pub fn from_type(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|w| w.as_type() == value)
    }

    /// Whether this weapon can damage another craft.
    ///
    /// Turbo, Shield and Autopilot have no projectile and no blast. The
    /// Disruptor is a projectile but authors no `damage` or `slowdown_time`: a
    /// hit applies a control effect and takes no energy (`Disruptor_ApplyEffect`,
    /// `0x08850ec8` on `psp-pure-usa`).
    #[must_use]
    pub fn damages(self) -> bool {
        !matches!(
            self,
            Self::Turbo | Self::Shield | Self::Autopilot | Self::Disruptor
        )
    }
}

pub mod ai;
mod disruptor;
mod error;
mod stats;

pub use disruptor::{
    DisruptorStats, Effect as DisruptorEffect, EffectKind as DisruptorEffectKind,
    PER_CLASS_KMH as DISRUPTOR_PER_CLASS_KMH,
};
pub use error::Error;
pub use stats::{
    BombStats, CannonStats, LeachBeamStats, MineStats, MissileStats, PlasmaStats, QuakeStats,
    RepulserStats, RocketStats, ShurikenStats, Simple,
};

/// How likely a pad is to hand out one weapon, from one `<Pickupodds>` block.
///
/// Raw weights, not probabilities: how the original draws from them is unread.
/// The same weapon is weighted differently for AI and human, and by grid
/// position.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PickupOdds {
    /// Weight for an AI-driven craft.
    pub ai: f32,
    /// Weight at the back of the field.
    pub back: f32,
    /// Weight at the front of the field.
    pub front: f32,
    /// Weight for a human-driven craft.
    pub human: f32,
}

/// One speed class's pickup table.
#[derive(Debug, Clone, PartialEq)]
pub struct PickupTable {
    /// The `class` attribute, verbatim.
    ///
    /// A string, not [`crate::handling::SpeedClass`]: `WeaponStats_Parse` tests
    /// the attribute against a fifth name, `Vector`, and discards the result,
    /// while the shipped race table authors four blocks and no `Vector`
    /// (`crates/formats/tests/weapons_ground_truth.rs` pins both). Narrowing
    /// would lose that; `docs/formats/handling-stats.md`'s fifth-class question
    /// is still open.
    pub class: String,
    /// The weapons this class authors odds for, in document order.
    pub odds: Vec<(Weapon, PickupOdds)>,
}

impl PickupTable {
    /// This class's odds for one weapon, or `None` when it authors none.
    #[must_use]
    pub fn get(&self, weapon: Weapon) -> Option<PickupOdds> {
        self.odds
            .iter()
            .find(|(w, _)| *w == weapon)
            .map(|(_, o)| *o)
    }
}

/// One `WeaponStats_*.xml`.
#[derive(Debug, Clone, PartialEq)]
pub struct WeaponStats {
    /// `<Weapon type="Global"><Stats slowdown_limit/>`.
    ///
    /// **A ceiling on seconds of slowdown outstanding, not a speed floor.**
    /// `Ship_AddSlowdown` (`0x08848690`) clamps the victim's timer to this after
    /// adding a hit's [`RocketStats::slowdown_time`]; it is `+0x00` of the block
    /// `WeaponStats_ParseGlobal` (`0x0880dab0`) writes. See
    /// `docs/ghidra/functions/psp-pulse-usa/engine.md`.
    pub slowdown_limit: f32,
    /// Turbo, Shield and Autopilot, in that order; `None` for one the file
    /// omits. Read through [`Self::simple`].
    simple: [Option<Simple>; 3],
    /// The Rocket's block, or `None`; read through [`Self::rocket`].
    rocket: Option<RocketStats>,
    /// The Missile's block, or `None`; read through [`Self::missile`].
    missile: Option<MissileStats>,
    /// The Plasma's block, or `None`; read through [`Self::plasma`].
    plasma: Option<PlasmaStats>,
    /// The Mine's block, or `None`; read through [`Self::mine`].
    mine: Option<MineStats>,
    /// The Shuriken's block, or `None`; read through [`Self::shuriken`].
    shuriken: Option<ShurikenStats>,
    /// The Bomb's block, or `None`; read through [`Self::bomb`].
    bomb: Option<BombStats>,
    /// The Cannon's block, or `None`; read through [`Self::cannon`].
    cannon: Option<CannonStats>,
    /// The Quake's block, or `None`; read through [`Self::quake`].
    quake: Option<QuakeStats>,
    /// The Repulser's block, or `None`; read through [`Self::repulser`].
    repulser: Option<RepulserStats>,
    /// The LeachBeam's block, or `None`; read through [`Self::leach_beam`].
    leach_beam: Option<LeachBeamStats>,
    /// The Disruptor's block, or `None`; read through [`Self::disruptor`].
    disruptor: Option<DisruptorStats>,
    /// `absorb` for every weapon the file authors, in document order.
    pub absorb: Vec<(Weapon, f32)>,
    /// One entry per `<Pickupodds>` block, in document order.
    pub pickups: Vec<PickupTable>,
    /// Weapons the file authors that this build could not decode, and the first
    /// attribute each was missing.
    ///
    /// Not an error and not silence: see [`optional_block`]. A loader reports
    /// it so "the disc has no Bomb" stays distinct from "this build cannot read
    /// this disc's Bomb".
    pub skipped: Vec<(Weapon, &'static str)>,
}

impl WeaponStats {
    /// The `absorb`/`time` pair for one of the three simple weapons.
    ///
    /// `None` for a weapon the file omits and for one that is not simple;
    /// answering a rocket's with a zero would hide a caller error.
    #[must_use]
    pub fn simple(&self, weapon: Weapon) -> Option<Simple> {
        let index = match weapon {
            Weapon::Turbo => 0,
            Weapon::Shield => 1,
            Weapon::Autopilot => 2,
            _ => return None,
        };
        self.simple[index]
    }

    /// The Rocket's `<Stats>`, or `None` when the file authors no Rocket.
    ///
    /// `None` is a real state: the Race and Eliminator tables need not carry the
    /// same set.
    #[must_use]
    pub fn rocket(&self) -> Option<RocketStats> {
        self.rocket
    }

    /// The Missile's `<Stats>`, or `None` when the file authors no Missile.
    ///
    /// The Eliminator's missile hits twice as hard for half the absorb.
    #[must_use]
    pub fn missile(&self) -> Option<MissileStats> {
        self.missile
    }

    /// The Plasma's `<Stats>`, or `None`. Both Pulse tables author one, Pure's
    /// does not; see `docs/formats/pure-status.md`.
    #[must_use]
    pub fn plasma(&self) -> Option<PlasmaStats> {
        self.plasma
    }

    /// The Mine's `<Stats>`, or `None` when the file authors no Mine.
    #[must_use]
    pub fn mine(&self) -> Option<MineStats> {
        self.mine
    }

    /// The Bomb's `<Stats>`, or `None` when the file authors no Bomb.
    #[must_use]
    pub fn bomb(&self) -> Option<BombStats> {
        self.bomb
    }

    /// The Cannon's `<Stats>`, or `None` when the file authors no Cannon.
    #[must_use]
    pub fn cannon(&self) -> Option<CannonStats> {
        self.cannon
    }

    /// The Quake's `<Stats>`, or `None` when the file authors no Quake.
    #[must_use]
    pub fn quake(&self) -> Option<QuakeStats> {
        self.quake
    }

    /// The Repulser's `<Stats>`, or `None` when the file authors no Repulser.
    ///
    /// `None` on Pure, which authors no Repulser block.
    #[must_use]
    pub fn repulser(&self) -> Option<RepulserStats> {
        self.repulser
    }

    /// The LeachBeam's `<Stats>`, or `None` when the file authors no LeachBeam.
    ///
    /// Pure authors none (no block, no sight widgets; measured on
    /// `pure-psp-usa.chd`), so `None` is the usual answer there.
    #[must_use]
    pub fn leach_beam(&self) -> Option<LeachBeamStats> {
        self.leach_beam
    }

    /// The Shuriken's `<Stats>`, or `None` when the file authors no Shuriken.
    #[must_use]
    pub fn shuriken(&self) -> Option<ShurikenStats> {
        self.shuriken
    }

    /// The Disruptor's block, or `None` when the file authors no Disruptor.
    ///
    /// Only Pure authors it, so a pad on any other title never hands one out:
    /// `oag_weapons::pickup` draws only weapons the table weights.
    #[must_use]
    pub fn disruptor(&self) -> Option<DisruptorStats> {
        self.disruptor
    }

    /// One weapon's `absorb`, or `None` when the file authors no such weapon.
    #[must_use]
    pub fn absorb(&self, weapon: Weapon) -> Option<f32> {
        self.absorb
            .iter()
            .find(|(w, _)| *w == weapon)
            .map(|(_, a)| *a)
    }

    /// One class's pickup table, matched on the `class` attribute.
    #[must_use]
    pub fn pickups_for(&self, class: &str) -> Option<&PickupTable> {
        self.pickups.iter().find(|t| t.class == class)
    }
}

type Result<T> = std::result::Result<T, Error>;

/// Reads a weapon table out of an archive entry, expanding shortened XML.
///
/// # Errors
///
/// [`Error`] for a blob that is not this document.
pub fn from_blob(data: &[u8]) -> Result<WeaponStats> {
    if fexml::is_fexml(data) {
        parse(&fexml::expand(data)?)
    } else {
        parse(std::str::from_utf8(data).map_err(|_| fexml::Error::NotText)?)
    }
}

/// Reads a weapon table out of expanded XML.
///
/// # Errors
///
/// [`Error`] for a document that is not this one.
pub fn parse(xml: &str) -> Result<WeaponStats> {
    let root = fexml::parse(xml);
    let stats = find(&root, "WeaponStats").ok_or(Error::MissingRoot)?;

    let mut slowdown_limit = None;
    let mut simple: [Option<Simple>; 3] = [None; 3];
    let mut rocket = None;
    let mut missile = None;
    let mut plasma = None;
    let mut shuriken = None;
    let mut mine = None;
    let mut bomb = None;
    let mut cannon = None;
    let mut quake = None;
    let mut repulser = None;
    let mut leach_beam = None;
    let mut disruptor = None;
    let mut absorb = Vec::new();
    let mut skipped = Vec::new();

    // One pass: an unknown `type` is skipped and a duplicate overwrites.
    // `Global` is special only here.
    for weapon in stats.children_named("Weapon") {
        let Some(kind) = weapon.value("type") else {
            continue;
        };
        let Some(block) = weapon.children_named("Stats").next() else {
            continue;
        };
        if kind == "Global" {
            slowdown_limit = Some(number(block, "Stats", "slowdown_limit")?);
            continue;
        }
        let Some(weapon_kind) = Weapon::from_type(kind) else {
            continue;
        };
        absorb.push((weapon_kind, number(block, "Stats", "absorb")?));
        if weapon_kind == Weapon::Rocket {
            let (speeds, class_independent) = class_speeds(block)?;
            rocket = Some(RocketStats {
                absorb: number(block, "Stats", "absorb")?,
                blastforce: number(block, "Stats", "blastforce")?,
                blastradius: number(block, "Stats", "blastradius")?,
                damage: number(block, "Stats", "damage")?,
                slowdown_time: number(block, "Stats", "slowdown_time")?,
                // Camel-cased in the document; the file's spelling, matched verbatim.
                launch_speed: optional(block, "Stats", "launchSpeed")?.unwrap_or(0.0),
                spread: number(block, "Stats", "spread")?,
                speeds,
                class_independent,
            });
            continue;
        }
        if weapon_kind == Weapon::Missile {
            let (speeds, class_independent) = class_speeds(block)?;
            missile = Some(MissileStats {
                absorb: number(block, "Stats", "absorb")?,
                blastforce: number(block, "Stats", "blastforce")?,
                blastradius: number(block, "Stats", "blastradius")?,
                damage: number(block, "Stats", "damage")?,
                slowdown_time: number(block, "Stats", "slowdown_time")?,
                launch_speed: optional(block, "Stats", "launchSpeed")?.unwrap_or(0.0),
                lock_min_dist: number(block, "Stats", "lock_min_dist")?,
                lock_max_dist: number(block, "Stats", "lock_max_dist")?,
                speeds,
                class_independent,
            });
            continue;
        }
        if weapon_kind == Weapon::Plasma {
            // Optional for the Bomb's reason: a title that tunes the Plasma on
            // attributes this build does not read loses it, not the table. Pure
            // authors `speed="900"` and no `launchSpeed` (measured 2026-09-02 on
            // `pure-psp-usa.chd`), which `class_speeds` and `optional` handle.
            // `charge_time` is deliberately not read; see `PlasmaStats`.
            plasma = optional_block(&mut skipped, weapon_kind, || {
                let (speeds, class_independent) = class_speeds(block)?;
                Ok(PlasmaStats {
                    absorb: number(block, "Stats", "absorb")?,
                    blastforce: number(block, "Stats", "blastforce")?,
                    blastradius: number(block, "Stats", "blastradius")?,
                    damage: number(block, "Stats", "damage")?,
                    slowdown_time: number(block, "Stats", "slowdown_time")?,
                    launch_speed: optional(block, "Stats", "launchSpeed")?.unwrap_or(0.0),
                    speeds,
                    class_independent,
                })
            })?;
            continue;
        }
        if weapon_kind == Weapon::Shuriken {
            // Optional for the Bomb's reason. `rhicochetdamage` and
            // `rhicochetForce` are deliberately absent; see `ShurikenStats`.
            shuriken = optional_block(&mut skipped, weapon_kind, || {
                let (speeds, class_independent) = class_speeds(block)?;
                Ok(ShurikenStats {
                    absorb: number(block, "Stats", "absorb")?,
                    // The file capitalises this where the Rocket's is lower-case.
                    blastforce: number(block, "Stats", "blastForce")?,
                    blastradius: number(block, "Stats", "blastradius")?,
                    blastdamage: number(block, "Stats", "blastdamage")?,
                    slowdown_time: number(block, "Stats", "slowdown_time")?,
                    launch_speed: optional(block, "Stats", "launchSpeed")?.unwrap_or(0.0),
                    fuse: number(block, "Stats", "fuse")?,
                    speeds,
                    class_independent,
                })
            })?;
            continue;
        }
        if weapon_kind == Weapon::Bomb {
            // A missing attribute leaves the weapon undecoded rather than failing
            // the file. Pure's Bomb is why: it authors `damageradius` and no
            // `timetodie`, and one such block used to cost Pure its whole table.
            //
            // `timetodie` is optional since 2026-09-15: Pure's
            // `WeaponStats_ParseBomb` has no branch for it, so `None` is a Bomb
            // with no fuse. See `BombStats::timetodie`.
            //
            // A bad *value* still fails: a missing field is a dialect, rubbish in
            // one is a broken file.
            bomb = optional_block(&mut skipped, weapon_kind, || {
                Ok(BombStats {
                    absorb: number(block, "Stats", "absorb")?,
                    blastforce: number(block, "Stats", "blastforce")?,
                    blastradius: number(block, "Stats", "blastradius")?,
                    damage: number(block, "Stats", "damage")?,
                    slowdown_time: number(block, "Stats", "slowdown_time")?,
                    timetodie: optional(block, "Stats", "timetodie")?,
                    trigger_radius: number(block, "Stats", "trigger_radius")?,
                })
            })?;
            continue;
        }
        if weapon_kind == Weapon::Mine {
            mine = optional_block(&mut skipped, weapon_kind, || {
                Ok(MineStats {
                    absorb: number(block, "Stats", "absorb")?,
                    blastforce: number(block, "Stats", "blastforce")?,
                    blastradius: number(block, "Stats", "blastradius")?,
                    damage: number(block, "Stats", "damage")?,
                    slowdown_time: number(block, "Stats", "slowdown_time")?,
                    timetodie: number(block, "Stats", "timetodie")?,
                    trigger_radius: number(block, "Stats", "trigger_radius")?,
                })
            })?;
            continue;
        }
        if weapon_kind == Weapon::Cannon {
            // Optional for the Bomb's reason. Pure ships no `<Weapon
            // type="Cannon">` at all, per `pickups.md`'s reading of its
            // loading-screen icon set.
            cannon = optional_block(&mut skipped, weapon_kind, || {
                Ok(CannonStats {
                    absorb: number(block, "Stats", "absorb")?,
                    // Truncated, not rounded: `WeaponStats_ParseCannon` stores
                    // `(int)Xml_AttributeAsFloat(...)` at `stats+0x70`.
                    rounds: number(block, "Stats", "rounds")?.trunc(),
                    // The reciprocal, as `WeaponStats_ParseCannon` (`0x0880c774`)
                    // stores `1.0 / attribute` at `stats+0x78`: the attribute is
                    // rounds per second, the field seconds per round. See
                    // `CannonStats::rate`.
                    rate: 1.0 / number(block, "Stats", "rate")?,
                    damage_per_bullet: number(block, "Stats", "damage_per_bullet")?,
                    slowdown_time: number(block, "Stats", "slowdown_time")?,
                })
            })?;
            continue;
        }
        if weapon_kind == Weapon::LeachBeam {
            // Optional for the Bomb's reason, and the block that needs it: Pure
            // authors no LeachBeam. No `slowdown_time` is asked for, since no
            // shipped table authors one; see `LeachBeamStats`.
            leach_beam = optional_block(&mut skipped, weapon_kind, || {
                Ok(LeachBeamStats {
                    absorb: number(block, "Stats", "absorb")?,
                    lock_min_dist: number(block, "Stats", "lock_min_dist")?,
                    lock_max_dist: number(block, "Stats", "lock_max_dist")?,
                    damage: number(block, "Stats", "damage")?,
                    repair: number(block, "Stats", "repair")?,
                    slow_ship_factor: number(block, "Stats", "slowShipFactor")?,
                    range: number(block, "Stats", "range")?,
                    active_time: number(block, "Stats", "active_time")?,
                    energy_multiplier: number(block, "Stats", "energy_multiplier")?,
                })
            })?;
            continue;
        }
        if weapon_kind == Weapon::Disruptor {
            // Optional for the Bomb's reason. Pure is the only title authoring
            // the block; the `<Effect>` children are why it has its own module.
            disruptor = optional_block(&mut skipped, weapon_kind, || {
                disruptor::parse(weapon, block)
            })?;
            continue;
        }
        if weapon_kind == Weapon::Repulser {
            // Optional for the Bomb's reason. The original's `strcasecmp` matches
            // `blastForce`/`blastRadius` case-insensitively.
            repulser = optional_block(&mut skipped, weapon_kind, || {
                Ok(RepulserStats {
                    absorb: number(block, "Stats", "absorb")?,
                    damage: number(block, "Stats", "damage")?,
                    blastforce: number(block, "Stats", "blastforce")?,
                    slowdown_time: number(block, "Stats", "slowdown_time")?,
                    blast_time: number(block, "Stats", "blast_time")?,
                    wave_time: number(block, "Stats", "wave_time")?,
                })
            })?;
            continue;
        }
        if weapon_kind == Weapon::Quake {
            // Optional for the Bomb's reason.
            quake = optional_block(&mut skipped, weapon_kind, || {
                Ok(QuakeStats {
                    absorb: number(block, "Stats", "absorb")?,
                    damage: number(block, "Stats", "damage")?,
                    radius: number(block, "Stats", "radius")?,
                    slowdown_time: number(block, "Stats", "slowdown_time")?,
                })
            })?;
            continue;
        }
        let index = match weapon_kind {
            Weapon::Turbo => 0,
            Weapon::Shield => 1,
            Weapon::Autopilot => 2,
            _ => continue,
        };
        simple[index] = Some(Simple {
            absorb: number(block, "Stats", "absorb")?,
            time: number(block, "Stats", "time")?,
        });
    }

    let mut pickups = Vec::new();
    for table in stats.children_named("Pickupodds") {
        let class = table
            .value("class")
            .ok_or(Error::MissingAttribute {
                element: "Pickupodds",
                attribute: "class",
            })?
            .to_owned();
        let mut odds = Vec::new();
        for weapon in table.children_named("Weapon") {
            let Some(kind) = weapon.value("type").and_then(Weapon::from_type) else {
                continue;
            };
            let Some(block) = weapon.children_named("Stats").next() else {
                continue;
            };
            odds.push((
                kind,
                PickupOdds {
                    ai: number(block, "Stats", "ai")?,
                    back: number(block, "Stats", "back")?,
                    front: number(block, "Stats", "front")?,
                    human: number(block, "Stats", "human")?,
                },
            ));
        }
        pickups.push(PickupTable { class, odds });
    }

    Ok(WeaponStats {
        slowdown_limit: slowdown_limit.ok_or(Error::MissingElement {
            parent: "WeaponStats",
            element: "Weapon type=\"Global\"",
        })?,
        simple,
        rocket,
        missile,
        plasma,
        shuriken,
        mine,
        bomb,
        cannon,
        quake,
        repulser,
        leach_beam,
        disruptor,
        absorb,
        pickups,
        skipped,
    })
}

/// The first node with this name, at any depth.
fn find<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if node.name == name {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, name))
}

/// Decodes one weapon's block, or records that this build cannot.
///
/// A block missing an attribute this build reads is a dialect (Pure's fuseless
/// Bomb) and costs that weapon alone. A present but unparseable attribute is a
/// broken file and fails the whole read. Skips land in [`WeaponStats::skipped`].
fn optional_block<T>(
    skipped: &mut Vec<(Weapon, &'static str)>,
    weapon: Weapon,
    decode: impl FnOnce() -> Result<T>,
) -> Result<Option<T>> {
    match decode() {
        Ok(stats) => Ok(Some(stats)),
        Err(Error::MissingAttribute { attribute, .. }) => {
            skipped.push((weapon, attribute));
            Ok(None)
        }
        Err(other) => Err(other),
    }
}

/// The four per-class speeds, from whichever of the two dialects the file uses.
///
/// Pulse and HD author `venomspeed`, `flashspeed`, `rapierspeed` and
/// `phantomspeed`. Pure authors a single `speed` for every class (measured
/// 2026-08-26: Missile `speed="950"`, Rocket `speed="1000"`). Folding one into
/// four is the shape this crate carries, not a substitution. The per-class
/// spelling wins if both appear; a file with neither errors naming `venomspeed`.
///
/// The second return is `true` for the single-`speed` dialect, which
/// [`stats::RocketStats::speed_for_named`] needs for a fifth rung.
fn class_speeds(block: &Node) -> Result<([f32; 4], bool)> {
    if let Some(one) = optional(block, "Stats", "speed")?
        && block.value("venomspeed").is_none()
    {
        return Ok(([one; 4], true));
    }
    Ok((
        [
            number(block, "Stats", "venomspeed")?,
            number(block, "Stats", "flashspeed")?,
            number(block, "Stats", "rapierspeed")?,
            number(block, "Stats", "phantomspeed")?,
        ],
        false,
    ))
}

/// [`number`], but a missing attribute is `None`. A present attribute that will
/// not parse is still an error.
fn optional(node: &Node, element: &'static str, attribute: &'static str) -> Result<Option<f32>> {
    if node.value(attribute).is_none() {
        return Ok(None);
    }
    number(node, element, attribute).map(Some)
}

fn number(node: &Node, element: &'static str, attribute: &'static str) -> Result<f32> {
    let raw = node
        .value(attribute)
        .ok_or(Error::MissingAttribute { element, attribute })?;
    let bad = || Error::NotANumber {
        element,
        attribute,
        value: raw.to_string(),
    };
    let parsed: f32 = raw.trim().parse().map_err(|_| bad())?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(bad())
    }
}

#[cfg(test)]
mod tests;
