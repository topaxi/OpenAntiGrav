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
//!
//! Everything else - blast radii, projectile speeds, lock distances, the seven
//! disturber effects - is named on `docs/formats/weapon-stats.md` and left
//! undecoded, because nothing fires a projectile yet and a field decoded with no
//! consumer is a field nobody has checked.
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
