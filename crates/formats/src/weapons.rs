//! `WeaponStats_*.xml`: the weapon table, the disturber odds and the pickup
//! distribution.
//!
//! Weapons in Pulse are **data, not code**, the same way ship handling is. Two
//! files carry the whole system - [`RACE_ENTRY`] for the ordinary modes and
//! [`ELIMINATION_ENTRY`] for Eliminator - and `WeaponStats_Parse` (`0x0880db7c`)
//! reads both through one parser, so a mode swaps the table wholesale rather
//! than patching it.
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
//! The schema and the per-weapon parser addresses are in
//! `docs/formats/weapon-stats.md`.
//!
//! # What this decodes, and what it names without decoding
//!
//! The same principle [`crate::handling`] states: only what a consumer exists
//! for. Concretely:
//!
//! - **The three weapons that damage nobody** - Turbo, Shield and Autopilot -
//!   in full. They share one two-attribute schema and are the three that need
//!   no projectile, no target and no AI to be worth building.
//! - **`absorb` for every weapon**, because it is the one attribute all fourteen
//!   carry and it is what `Ship_Damage`'s absorb branch spends
//!   (`docs/ghidra/functions/psp-pulse-usa/shield.md`).
//! - **`<Pickupodds>` in full**, because it *is* the pickup system's design:
//!   what a pad hands out, weighted per speed class and separately for AI,
//!   human, front and back of the grid.
//! - **`<Global>`'s `slowdown_limit`**.
//! - **The Rocket's block**, since [`crate::weapons`]'s first consumer -
//!   `oag_gameplay::projectile` - flies one.
//! - **The Missile's block, lock distances included**, since a missile now locks
//!   and homes. That is what moved them out of the undecoded list below: the
//!   rule is "only what a consumer exists for", and the consumer is
//!   `oag_gameplay::projectile::lock`.
//! - **The Mine's block**, since `oag_gameplay::projectile::mine` drops one.
//!   Seven attributes and no speed, which is the shape of a weapon that is not
//!   launched so much as left behind.
//! - **The Bomb's block**, less `damageradius`, since the same module drops one
//!   of those too. That weapon is the Mine one size up and its block says so.
//!
//! Everything else - the other eight weapons' blocks and the seven disturber
//! effects - is named on `docs/formats/weapon-stats.md` and left undecoded,
//! because a field decoded with no consumer is a field nobody has checked.
//! `slowdown_time` stays out on all four decoded weapons for exactly that
//! reason, and so does the Bomb's second radius - see [`BombStats`], which is
//! explicit about it because that one is easy to mistake for an oversight.
//!
//! # Nothing here defaults
//!
//! An attribute the schema lists is required, and a missing one is an [`Error`]
//! rather than a zero, for the reason [`crate::handling`] gives at length: a
//! quietly-zero `time` is a shield that expires immediately, which reads as a
//! gameplay bug rather than as a parse failure.
//!
//! A weapon the file does not author at all is a different thing and is not an
//! error - [`WeaponStats::simple`] returns `None`. The two Eliminator and Race
//! files need not carry the same set.

use std::fmt;

use crate::fexml::{self, Node};

/// `Data\XML\WeaponStats_Race.xml`, the ordinary race modes' table.
pub const RACE_ENTRY: &str = r"Data\XML\WeaponStats_Race.xml";

/// `Data\XML\WeaponStats_Elimination.xml`, Eliminator's.
pub const ELIMINATION_ENTRY: &str = r"Data\XML\WeaponStats_Elimination.xml";

/// The fourteen pickups, in the order the class-name pool at `0x08a78c00` holds
/// them.
///
/// The order is the executable's own rather than the file's, because the file's
/// order differs between the two shipped tables while the pool does not - and
/// `Ship_Damage`'s `weapon_kind` telemetry buckets are indexed by *something*,
/// with this pool the obvious candidate. That mapping is **not confirmed**; see
/// `docs/ghidra/functions/psp-pulse-usa/shield.md`.
///
/// # This order is the `<Stats>` struct's layout, and is *not* the weapon id
///
/// Recovered 2026-08-26 and worth stating on the enum itself, because the
/// project has now got it wrong twice in the other direction. The pool order
/// **is** the order the original's per-class `<Stats>` struct lays its blocks
/// out in, which is what makes the offsets on [`RocketStats`], [`MissileStats`]
/// and [`MineStats`] check against each other. It is **not** the weapon id at
/// `craft+0x1bc`: that agrees at eleven of thirteen positions and reaches the
/// Mine at 8 and the Bomb at 9. Nor is it the index
/// `WeaponAiStats_Load` and the incoming-weapon announcement table use, which
/// is a third thing - the shipped file's element order, with `Cannon` after
/// `Shield`. See `docs/ghidra/functions/psp-pulse-usa/mine.md`, which reads all
/// three.
///
/// Nothing here reads this enum's index as a weapon id, and nothing should
/// start.
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
}

impl Weapon {
    /// Every weapon, in the pool's order.
    ///
    /// Fixed-size so that adding one is a compile error at every caller that
    /// enumerates them, the same reason `oag_race::Mode::ALL` is an array.
    pub const ALL: [Self; 13] = [
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
    ];

    /// The `type` attribute's spelling, exactly as the file has it.
    ///
    /// `LeachBeam` and `Repulser` are the game's spellings and are kept rather
    /// than corrected: this is the string a document is matched against.
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
    /// The three that cannot are the three with no projectile and no blast, and
    /// they are exactly the three whose `<Stats>` is `absorb` and `time` alone -
    /// which is a property of the shipped schema, not a judgement.
    #[must_use]
    pub fn damages(self) -> bool {
        !matches!(self, Self::Turbo | Self::Shield | Self::Autopilot)
    }
}

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
/// `crates/gameplay/src/projectile.rs`. The nine still undecoded need a lock, a
/// beam, track deformation or the slowdown mechanic, and none of those exists.
///
/// **One of the Rocket's eleven is left out for the same reason**:
/// `slowdown_time`, the slowdown mechanic's half of the pair with
/// [`WeaponStats::slowdown_limit`], which has no consumer.
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
    /// Private, because the index *is* the meaning: read it through
    /// [`RocketStats::speed_for`], which cannot get the order wrong.
    speeds: [f32; 4],
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
/// `slowdown_time` is left out for the reason [`RocketStats`] leaves it out: the
/// slowdown mechanic it feeds has no consumer here yet. **It does have one in the
/// original** - the missile's own impact accumulates it into the victim's
/// `weapon_record+0x130` - which is recorded on that page so whoever builds the
/// mechanic starts from a known consumer rather than from nothing.
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
    /// Added to the *firing craft's own speed* at launch - see
    /// `oag_gameplay::projectile::launch_missile`, which is where that
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
    /// Private for the reason [`RocketStats::speeds`] is: the index *is* the
    /// meaning. Read it through [`MissileStats::speed_for`].
    speeds: [f32; 4],
}

impl MissileStats {
    /// The speed a missile settles at in one speed class, in km/h.
    ///
    /// **The unit matters and it is not units per second.** A missile does not
    /// fly at this from launch either: `Missile_SpeedNow` (`0x0885a038`) ramps
    /// linearly from the launch speed to this over one second. See
    /// `oag_gameplay::projectile::missile_speed_kmh`.
    #[must_use]
    pub fn speed_for(&self, class: crate::handling::SpeedClass) -> f32 {
        self.speeds[class as usize]
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
/// then sits there. `slowdown_time` is left out for the reason
/// [`RocketStats`] leaves it out: the mechanic behind `<Global slowdown_limit>`
/// still has no consumer.
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
/// So it stays undecoded, for the reason [`RocketStats`] leaves `slowdown_time`
/// undecoded and the module docs give at length: a field decoded with no
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
    /// How long a dropped bomb lives before it goes off on its own, in seconds.
    ///
    /// Twenty on both shipped tables against the Mine's seven, which is the
    /// single biggest difference between the two weapons: a bomb left on the
    /// track is a hazard for most of a lap.
    pub timetodie: f32,
    /// How close a craft must come before the bomb goes off.
    pub trigger_radius: f32,
}

/// How likely a pad is to hand out one weapon, from one `<Pickupodds>` block.
///
/// Four weights, and their shape is the pickup design: the same weapon is
/// weighted differently for an AI and for a human, and differently again by
/// where the craft is on the grid. Raw weights, not probabilities - nothing here
/// normalises them, because how the original draws from them is unread.
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
    /// A string rather than [`crate::handling::SpeedClass`], because the
    /// *parser* knows a fifth name the *data* does not use: `WeaponStats_Parse`
    /// tests the attribute against `Vector` and discards the result before
    /// testing the four that store an index. The shipped race table authors four
    /// blocks and no `Vector`, which
    /// `crates/formats/tests/weapons_ground_truth.rs` pins both halves of.
    ///
    /// Narrowing this to the four would throw that evidence away, and
    /// `docs/formats/handling-stats.md`'s fifth-class question is still open.
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
    pub slowdown_limit: f32,
    /// Turbo, Shield and Autopilot, in that order, as the file authors them.
    ///
    /// `None` for one the file omits. Read it through [`Self::simple`].
    simple: [Option<Simple>; 3],
    /// The Rocket's own block, or `None` for a file that omits it.
    ///
    /// Read it through [`Self::rocket`].
    rocket: Option<RocketStats>,
    /// The Missile's own block, or `None` for a file that omits it.
    ///
    /// Read it through [`Self::missile`].
    missile: Option<MissileStats>,
    /// The Mine's own block, or `None` for a file that omits it.
    ///
    /// Read it through [`Self::mine`].
    mine: Option<MineStats>,
    /// The Bomb's own block, or `None` for a file that omits it.
    ///
    /// Read it through [`Self::bomb`].
    bomb: Option<BombStats>,
    /// `absorb` for every weapon the file authors, in document order.
    pub absorb: Vec<(Weapon, f32)>,
    /// One entry per `<Pickupodds>` block, in document order.
    pub pickups: Vec<PickupTable>,
}

impl WeaponStats {
    /// The `absorb`/`time` pair for one of the three simple weapons.
    ///
    /// `None` both for a weapon the file omits and for one that is not simple -
    /// asking for a rocket's simple stats is a caller error, and answering with
    /// a defaulted zero would hide it.
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
    /// `None` is a real state rather than a failure, the same way
    /// [`Self::simple`]'s is: the Race and Eliminator tables need not carry the
    /// same set, and a caller with no rocket stats hands out no rocket.
    #[must_use]
    pub fn rocket(&self) -> Option<RocketStats> {
        self.rocket
    }

    /// The Missile's `<Stats>`, or `None` when the file authors no Missile.
    ///
    /// `None` is a real state rather than a failure, exactly as [`Self::rocket`]'s
    /// is. Both shipped tables do author one, and they differ: the Eliminator's
    /// missile hits twice as hard for half the absorb.
    #[must_use]
    pub fn missile(&self) -> Option<MissileStats> {
        self.missile
    }

    /// The Mine's `<Stats>`, or `None` when the file authors no Mine.
    ///
    /// `None` is a real state rather than a failure, exactly as [`Self::rocket`]'s
    /// is. Both shipped tables do author one.
    #[must_use]
    pub fn mine(&self) -> Option<MineStats> {
        self.mine
    }

    /// The Bomb's `<Stats>`, or `None` when the file authors no Bomb.
    ///
    /// `None` is a real state rather than a failure, exactly as [`Self::rocket`]'s
    /// is. Both shipped tables do author one.
    #[must_use]
    pub fn bomb(&self) -> Option<BombStats> {
        self.bomb
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

/// What can go wrong reading a weapon table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The blob is not text, or the shortened-XML expansion failed.
    Fexml(fexml::Error),
    /// No `<WeaponStats>` root.
    MissingRoot,
    /// A required element is absent.
    MissingElement {
        /// The element that should have held it.
        parent: &'static str,
        /// What was missing.
        element: &'static str,
    },
    /// A required attribute is absent.
    MissingAttribute {
        /// The element it should have been on.
        element: &'static str,
        /// The attribute.
        attribute: &'static str,
    },
    /// An attribute is present and is not a finite number.
    NotANumber {
        /// The element it was on.
        element: &'static str,
        /// The attribute.
        attribute: &'static str,
        /// What the document actually said.
        value: String,
    },
}

impl From<fexml::Error> for Error {
    fn from(e: fexml::Error) -> Self {
        Self::Fexml(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fexml(e) => write!(f, "{e}"),
            Self::MissingRoot => write!(f, "no <WeaponStats> element"),
            Self::MissingElement { parent, element } => {
                write!(f, "<{parent}> has no <{element}>")
            }
            Self::MissingAttribute { element, attribute } => {
                write!(f, "<{element}> has no {attribute} attribute")
            }
            Self::NotANumber {
                element,
                attribute,
                value,
            } => write!(f, "<{element} {attribute}=\"{value}\"> is not a number"),
        }
    }
}

impl std::error::Error for Error {}

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
    let mut mine = None;
    let mut bomb = None;
    let mut absorb = Vec::new();

    // One pass over `<Weapon>` children, so an unknown `type` is skipped and a
    // duplicate overwrites rather than being silently the first or the last by
    // accident of a lookup. `Global` is a `type` like any other in the document
    // and is only special here.
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
            rocket = Some(RocketStats {
                absorb: number(block, "Stats", "absorb")?,
                blastforce: number(block, "Stats", "blastforce")?,
                blastradius: number(block, "Stats", "blastradius")?,
                damage: number(block, "Stats", "damage")?,
                // Camel-cased in the document while every other attribute here
                // is lower-cased. The file's spelling, kept verbatim, because
                // this is the string a lookup is matched against.
                launch_speed: number(block, "Stats", "launchSpeed")?,
                spread: number(block, "Stats", "spread")?,
                speeds: [
                    number(block, "Stats", "venomspeed")?,
                    number(block, "Stats", "flashspeed")?,
                    number(block, "Stats", "rapierspeed")?,
                    number(block, "Stats", "phantomspeed")?,
                ],
            });
            continue;
        }
        if weapon_kind == Weapon::Missile {
            missile = Some(MissileStats {
                absorb: number(block, "Stats", "absorb")?,
                blastforce: number(block, "Stats", "blastforce")?,
                blastradius: number(block, "Stats", "blastradius")?,
                damage: number(block, "Stats", "damage")?,
                launch_speed: number(block, "Stats", "launchSpeed")?,
                lock_min_dist: number(block, "Stats", "lock_min_dist")?,
                lock_max_dist: number(block, "Stats", "lock_max_dist")?,
                speeds: [
                    number(block, "Stats", "venomspeed")?,
                    number(block, "Stats", "flashspeed")?,
                    number(block, "Stats", "rapierspeed")?,
                    number(block, "Stats", "phantomspeed")?,
                ],
            });
            continue;
        }
        if weapon_kind == Weapon::Bomb {
            bomb = Some(BombStats {
                absorb: number(block, "Stats", "absorb")?,
                blastforce: number(block, "Stats", "blastforce")?,
                blastradius: number(block, "Stats", "blastradius")?,
                damage: number(block, "Stats", "damage")?,
                timetodie: number(block, "Stats", "timetodie")?,
                trigger_radius: number(block, "Stats", "trigger_radius")?,
            });
            continue;
        }
        if weapon_kind == Weapon::Mine {
            mine = Some(MineStats {
                absorb: number(block, "Stats", "absorb")?,
                blastforce: number(block, "Stats", "blastforce")?,
                blastradius: number(block, "Stats", "blastradius")?,
                damage: number(block, "Stats", "damage")?,
                timetodie: number(block, "Stats", "timetodie")?,
                trigger_radius: number(block, "Stats", "trigger_radius")?,
            });
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
        mine,
        bomb,
        absorb,
        pickups,
    })
}

/// The first node with this name, at any depth.
fn find<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if node.name == name {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, name))
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
