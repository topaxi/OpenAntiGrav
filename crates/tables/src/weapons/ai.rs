//! `Data\XML\WeaponAIstats.xml`: the fire-or-absorb odds an opponent rolls
//! against, one row per weapon.
//!
//! Found 2026-08-17 (`WeaponAiStats_Load`, `0x08851d88`) and read end to end
//! 2026-09-16 alongside the decision that consumes it - see
//! `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md` and
//! [`ai-stats.md`](../../../../docs/ghidra/functions/psp-pulse-usa/ai-stats.md).
//! `WeaponStats_Race.xml`/`WeaponStats_Elimination.xml` ([`super`]) tune what a
//! weapon *does*; this file tunes how readily an opponent *uses* one, and nothing
//! shares a struct between the two - the schemas do not overlap by one
//! attribute.
//!
//! # The schema is three floats a weapon, one element each
//!
//! ```xml
//! <WeaponAIStats>
//!   <Rockets useAgainstPlayer="1.1" useAgainstAI="1.2" absorb="1.0"/>
//!   <Missiles .../> <Quake .../> <Turbo .../> <Shield .../> <Cannon .../>
//!   <Autopilot .../> <Plasma .../> <Bomb .../> <Mines .../>
//!   <Leachbeam .../> <Repulser .../> <Shuriken .../>
//!   <Template .../>   <!-- matched by the loader, then discarded: not data -->
//! </WeaponAIStats>
//! ```
//!
//! # Keyed by element name, not by the loader's own index
//!
//! `WeaponAiStats_ParseWeapon(record, element, weapon_id)` indexes its C array
//! by a `weapon_id` the dispatch chain assigns from **document order**, and
//! [`ai-stats.md`] found that this is a *fourth* id space: it agrees with
//! `craft+0x1bc` (the id [`super::Weapon`]'s own doc comment already warns is
//! not [`super::Weapon::ALL`]'s pool order) at eleven of thirteen positions and
//! disagrees at Turbo/Shield/Cannon. Reading this file into anything indexed
//! by a different id would silently swap those three rows.
//!
//! This parser sidesteps the question entirely: it matches each `<Element>`
//! **by name** against a fixed name table below, straight to [`super::Weapon`],
//! so no numeric id space - the loader's, the craft's, or the pool's - ever
//! enters the result. `weapons_ground_truth`'s own coverage is the check that
//! this landed on the right rows: every weapon but Plasma and Quake reads
//! `1.2`/`1.1`, and only those two read `1.0`/`1.0` - a misrouted row would move
//! one of those two off the uniform value and could not hide.
//!
//! # The shipped values are nearly uniform, and that is not a parsing bug
//!
//! **This paragraph is Pulse's file** (and Pure's). HD's, 2048's and Omega's
//! `WeaponAIStats2048.xml` carry the same thirteen element names with
//! different values (Bomb and Mines author `absorb="1.1"`) and two more rows,
//! [`WeaponAiStats::all_weapons`] and [`WeaponAiStats::eliminator`].
//!
//! Every Pulse weapon authors `absorb="1.0"`. Every weapon but the Plasma and the
//! Quake authors `useAgainstAI="1.2" useAgainstPlayer="1.1"`; those two author
//! `1.0"/"1.0`. [`ai-stats.md`] spent two readings treating this as evidence the
//! file could not be the fire-or-absorb decision, on the theory that a
//! probability table would mark something as fire-only or absorb-only. It does
//! not need to: `WeaponAi_DecideFireOrAbsorb` multiplies each value into a roll
//! against a five-entry, skill-indexed rate table before comparing it to a
//! random draw, so `1.1` against `1.2` is a small bias, not a flag - see
//! `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`.

use crate::fexml::{self, Node};

use super::Weapon;

/// `Data\XML\WeaponAIstats.xml`.
pub const ENTRY: &str = r"Data\XML\WeaponAIstats.xml";

/// What can go wrong reading [`ENTRY`].
///
/// A sibling of [`super::Error`] rather than a variant of it: that type's
/// `MissingRoot` names `<WeaponStats>` in its own `Display`, and this file's
/// root is a different element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The blob is not text, or the shortened-XML expansion failed.
    Fexml(fexml::Error),
    /// No `<WeaponAIStats>` root.
    MissingRoot,
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

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fexml(e) => write!(f, "{e}"),
            Self::MissingRoot => write!(f, "no <WeaponAIStats> element"),
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

/// One weapon's `useAgainstPlayer`/`useAgainstAI`/`absorb` row.
///
/// All three are the multiplier `WeaponAi_DecideFireOrAbsorb` applies to a
/// skill-indexed rate before comparing it to a random draw - a bias, not a
/// probability on its own. See [`self`]'s own module doc for the shipped
/// values.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WeaponAiOdds {
    /// The multiplier used while [`super::Weapon`] is a viable shot at the
    /// player specifically - `WeaponAi_Update`'s own along-track gap test
    /// against the player, not against the nearest craft. See
    /// `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`.
    pub use_against_player: f32,
    /// The multiplier used otherwise - a viable shot at anything else, or no
    /// viable shot direction is authored for this weapon at all (the Autopilot
    /// has no direction flags and always reads this row).
    pub use_against_ai: f32,
    /// The multiplier applied to absorbing this weapon instead of firing it.
    pub absorb: f32,
}

/// `Data\XML\WeaponAIstats.xml`, one [`WeaponAiOdds`] per weapon the file
/// authors.
///
/// A `Vec` rather than a fixed array indexed by [`Weapon`], for the same
/// reason [`super::WeaponStats::absorb`] is one: a title need not author every
/// weapon, and this project's own no-`HashMap`-in-simulation-state rule
/// (`docs/architecture/determinism.md`) rules out the obvious alternative.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WeaponAiStats {
    odds: Vec<(Weapon, WeaponAiOdds)>,
    all_weapons: Option<WeaponAiOdds>,
    eliminator: Option<EliminatorAiStats>,
}

/// The `<EliminatorAIStats>` row HD, 2048 and Omega author and Pulse's file
/// does not: thirteen floats, by the attribute names the file itself uses.
///
/// **Parsed, not consumed.** Nothing in the simulation reads it. What it
/// tunes is a hypothesis, recorded with its evidence in
/// `docs/gameplay/race-modes.md` ("HD's `EliminatorAIStats` row"); the field
/// names here are the file's own words, not a claim about the original's
/// consumer. The three-step `easy`/`medium`/`hard` scales are kept by tier so
/// a reader never has to guess which is which.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EliminatorAiStats {
    /// `normalFlip`.
    pub normal_flip: f32,
    /// `infrontFlip`.
    pub infront_flip: f32,
    /// `easyFlipScale`, `mediumFlipScale`, `hardFlipScale`.
    pub flip_scale: [f32; 3],
    /// `easyAbsorbScale`, `mediumAbsorbScale`, `hardAbsorbScale`.
    pub absorb_scale: [f32; 3],
    /// `easyUseScale`, `mediumUseScale`, `hardUseScale`.
    pub use_scale: [f32; 3],
    /// `easyScoreScale`, `mediumScoreScale`, `hardScoreScale`.
    pub score_scale: [f32; 3],
}

impl WeaponAiStats {
    /// This weapon's odds, or `None` when the file authors no row for it.
    #[must_use]
    pub fn get(&self, weapon: Weapon) -> Option<WeaponAiOdds> {
        self.odds
            .iter()
            .find(|(w, _)| *w == weapon)
            .map(|(_, o)| *o)
    }

    /// The `<AllWeapons>` row HD, 2048 and Omega author after the thirteen:
    /// same three attributes, no weapon. Pulse's file has none. Parsed and
    /// unconsumed.
    #[must_use]
    pub fn all_weapons(&self) -> Option<WeaponAiOdds> {
        self.all_weapons
    }

    /// The `<EliminatorAIStats>` row, or `None` when the file authors none
    /// (Pulse's and Pure's). Parsed and unconsumed.
    #[must_use]
    pub fn eliminator(&self) -> Option<EliminatorAiStats> {
        self.eliminator
    }
}

/// Reads [`WeaponAiStats`] out of a raw archive entry, expanding shortened XML
/// first if that is what the blob is.
///
/// # Errors
///
/// [`Error`] for a document that is not this one.
pub fn from_blob(data: &[u8]) -> Result<WeaponAiStats, Error> {
    if fexml::is_fexml(data) {
        parse(&fexml::expand(data)?)
    } else {
        parse(std::str::from_utf8(data).map_err(|_| fexml::Error::NotText)?)
    }
}

/// Reads [`WeaponAiStats`] out of expanded XML.
///
/// # Errors
///
/// [`Error`] for a document that is not this one.
pub fn parse(xml: &str) -> Result<WeaponAiStats, Error> {
    let root = fexml::parse(xml);
    let stats = find(&root, "WeaponAIStats").ok_or(Error::MissingRoot)?;

    let mut odds = Vec::new();
    for (element, weapon) in ELEMENT_NAMES {
        let Some(node) = stats.children_named(element).next() else {
            continue;
        };
        odds.push((weapon, odds_of(node, element)?));
    }
    let all_weapons = match stats.children_named("AllWeapons").next() {
        Some(node) => Some(odds_of(node, "AllWeapons")?),
        None => None,
    };
    let eliminator = match stats.children_named("EliminatorAIStats").next() {
        Some(node) => Some(eliminator_of(node)?),
        None => None,
    };
    Ok(WeaponAiStats {
        odds,
        all_weapons,
        eliminator,
    })
}

fn odds_of(node: &Node, element: &'static str) -> Result<WeaponAiOdds, Error> {
    Ok(WeaponAiOdds {
        use_against_player: number(node, element, "useAgainstPlayer")?,
        use_against_ai: number(node, element, "useAgainstAI")?,
        absorb: number(node, element, "absorb")?,
    })
}

fn eliminator_of(node: &Node) -> Result<EliminatorAiStats, Error> {
    const E: &str = "EliminatorAIStats";
    Ok(EliminatorAiStats {
        normal_flip: number(node, E, "normalFlip")?,
        infront_flip: number(node, E, "infrontFlip")?,
        flip_scale: [
            number(node, E, "easyFlipScale")?,
            number(node, E, "mediumFlipScale")?,
            number(node, E, "hardFlipScale")?,
        ],
        absorb_scale: [
            number(node, E, "easyAbsorbScale")?,
            number(node, E, "mediumAbsorbScale")?,
            number(node, E, "hardAbsorbScale")?,
        ],
        use_scale: [
            number(node, E, "easyUseScale")?,
            number(node, E, "mediumUseScale")?,
            number(node, E, "hardUseScale")?,
        ],
        score_scale: [
            number(node, E, "easyScoreScale")?,
            number(node, E, "mediumScoreScale")?,
            number(node, E, "hardScoreScale")?,
        ],
    })
}

/// The file's own element names, which is a fourth id space from
/// [`super::Weapon`]'s pool order and from `craft+0x1bc` alike - see [`self`]'s
/// module doc. Not [`Weapon::as_type`]: three of these spellings
/// (`Rockets`/`Missiles`/`Mines`) are plural and one (`Leachbeam`) is cased
/// differently from that method's own.
const ELEMENT_NAMES: [(&str, Weapon); 13] = [
    ("Rockets", Weapon::Rocket),
    ("Missiles", Weapon::Missile),
    ("Quake", Weapon::Quake),
    ("Turbo", Weapon::Turbo),
    ("Shield", Weapon::Shield),
    ("Cannon", Weapon::Cannon),
    ("Autopilot", Weapon::Autopilot),
    ("Plasma", Weapon::Plasma),
    ("Bomb", Weapon::Bomb),
    ("Mines", Weapon::Mine),
    ("Leachbeam", Weapon::LeachBeam),
    ("Repulser", Weapon::Repulser),
    ("Shuriken", Weapon::Shuriken),
];

/// The first node with this name, at any depth - [`super::find`]'s twin, kept
/// separate because that one is private to the sibling module.
fn find<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if node.name.eq_ignore_ascii_case(name) {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, name))
}

fn number(node: &Node, element: &'static str, attribute: &'static str) -> Result<f32, Error> {
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
