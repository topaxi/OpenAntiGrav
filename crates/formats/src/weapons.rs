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

mod stats;

pub use stats::{BombStats, MineStats, MissileStats, RocketStats, Simple};

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
    /// Weapons the file authors that this build could not decode, and the first
    /// attribute each was missing.
    ///
    /// **Not an error and not silence.** See [`optional_block`]: a title that
    /// tunes a weapon on different attributes loses that weapon and keeps the
    /// rest of its table. A loader reports this so the difference between "the
    /// disc has no Bomb" and "this build cannot read this disc's Bomb" stays
    /// visible.
    pub skipped: Vec<(Weapon, &'static str)>,
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
    let mut skipped = Vec::new();

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
                launch_speed: optional(block, "Stats", "launchSpeed")?.unwrap_or(0.0),
                spread: number(block, "Stats", "spread")?,
                speeds: class_speeds(block)?,
            });
            continue;
        }
        if weapon_kind == Weapon::Missile {
            missile = Some(MissileStats {
                absorb: number(block, "Stats", "absorb")?,
                blastforce: number(block, "Stats", "blastforce")?,
                blastradius: number(block, "Stats", "blastradius")?,
                damage: number(block, "Stats", "damage")?,
                launch_speed: optional(block, "Stats", "launchSpeed")?.unwrap_or(0.0),
                lock_min_dist: number(block, "Stats", "lock_min_dist")?,
                lock_max_dist: number(block, "Stats", "lock_max_dist")?,
                speeds: class_speeds(block)?,
            });
            continue;
        }
        if weapon_kind == Weapon::Bomb {
            // **A missing attribute leaves the weapon undecoded rather than
            // failing the file**, and Pure's Bomb is why: it authors
            // `damageradius` and **no** `timetodie`, so its charge has no fuse
            // and sits until something triggers it. That is a different weapon
            // wearing the same name, not a field this build forgot to read -
            // and before this, one such block cost Pure its *whole* table, so a
            // Pure race parsed no Missile, no Rocket and no pickup odds either.
            //
            // A bad *value* still fails: a file that does not carry a field is
            // a dialect, and one with rubbish in a field is broken.
            bomb = optional_block(&mut skipped, weapon_kind, || {
                Ok(BombStats {
                    absorb: number(block, "Stats", "absorb")?,
                    blastforce: number(block, "Stats", "blastforce")?,
                    blastradius: number(block, "Stats", "blastradius")?,
                    damage: number(block, "Stats", "damage")?,
                    timetodie: number(block, "Stats", "timetodie")?,
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
                    timetodie: number(block, "Stats", "timetodie")?,
                    trigger_radius: number(block, "Stats", "trigger_radius")?,
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
        mine,
        bomb,
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
/// **The distinction is between a dialect and a broken file.** A block that does
/// not carry an attribute this build reads is a title tuning that weapon
/// differently - Pure's fuseless Bomb - and costs that weapon alone. A block
/// whose attribute is present and unparseable is a broken file and still fails
/// the whole read.
///
/// Recorded rather than dropped: [`WeaponStats::skipped`] is what a loader
/// reports, so "this title has no Bomb" is visible instead of looking like a
/// file that omits one.
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
/// **Two shapes, and the second is Pure's.** Pulse and Wipeout HD author one
/// speed per speed class - `venomspeed`, `flashspeed`, `rapierspeed`,
/// `phantomspeed` - and Pure authors a single `speed` that every class flies at.
/// Measured 2026-08-26 off `Data\XML\weaponstats.xml`, where the Missile reads
/// `speed="950"` and the Rocket `speed="1000"` with no per-class attribute in
/// the file at all.
///
/// The per-class spelling wins where both are present, which no shipped file
/// does; a file with neither is the error, and it names `venomspeed` because
/// that is the dialect a reader of this project is likelier to be holding.
///
/// **Folding one speed into four is not a substitution for missing data.** Pure
/// genuinely flies every class at the same weapon speed - there is nothing per
/// class to lose - so the array is the shape this crate carries rather than a
/// claim about the file.
fn class_speeds(block: &Node) -> Result<[f32; 4]> {
    if let Some(one) = optional(block, "Stats", "speed")?
        && block.value("venomspeed").is_none()
    {
        return Ok([one; 4]);
    }
    Ok([
        number(block, "Stats", "venomspeed")?,
        number(block, "Stats", "flashspeed")?,
        number(block, "Stats", "rapierspeed")?,
        number(block, "Stats", "phantomspeed")?,
    ])
}

/// [`number`], but a missing attribute is `None` rather than an error.
///
/// For the attributes one dialect authors and another does not. A *present*
/// attribute that will not parse is still an error: this is about the file not
/// carrying a field, not about tolerating rubbish in one.
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
