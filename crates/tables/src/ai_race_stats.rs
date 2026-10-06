//! `Data\XML\AIRaceStats_<class>.xml`: the AI's position-balancing thrust
//! table, and the one law of the original's that this project reads it for.
//!
//! Pulse ships one file a speed class (`AIRaceStats_venom.xml`, `_flash`,
//! `_rapier`, `_phantom`) in `Data.wad`, each holding one `<<Class>Stats>`
//! with a `<RaceBalancing>` block; `AiStats_LoadAll` (`0x08835830`) loads all
//! four into a per-class record. The layout and the parser are
//! `docs/ghidra/functions/psp-pulse-usa/ai-stats.md` (confidence 88).
//!
//! **What this module reads, and why only this**: `PosBalancing`'s eight
//! `AIThrust` figures and `SkillScale`'s three `ThrustOffset` /
//! `ThrustMultiplier` pairs - the inputs of the thrust the original gives the
//! **player's own craft after the finish line**
//! ([`AiRaceStats::finished_player_thrust`], measured 2026-10-04,
//! `docs/ghidra/functions/psp-pulse-usa/race-finish.md`). The opponents'
//! use of the same table is player-coupled rubber-banding this project does
//! not port (`docs/gameplay/ai.md`), so the rest of the file (`SpreadDist`,
//! `RubberBanding`, `StartStats`, `AIPackSwapping`) is left unread here.

use crate::fexml::{self, Node};
use crate::handling::SpeedClass;

/// The thrust inputs of one class's `AIRaceStats_<class>.xml`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AiRaceStats {
    /// `<PosBalancing><PlayerInPos1..8 AIThrust>`, indexed by place - 1.
    /// Percent of full thrust.
    pub ai_thrust: [f32; 8],
    /// `<SkillScale><SkillScalePoint1..3>`, the skill curve's three points.
    pub skill_points: [SkillPoint; 3],
}

/// One `<SkillScalePoint<N>>`: what the AI's thrust is shifted and scaled by
/// at skill `N`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillPoint {
    /// `ThrustOffset`, percent added.
    pub thrust_offset: f32,
    /// `ThrustMultiplier`, applied to the thrust's distance from place 1's.
    pub thrust_multiplier: f32,
}

impl AiRaceStats {
    /// `(ThrustOffset, ThrustMultiplier)` at skill scale `skill`.
    ///
    /// `AI_ComputeOpponentThrust` (`0x08855904`): below `2.0` it lerps
    /// Point1 to Point2 at `skill - 1`, otherwise Point2 to Point3 at
    /// `skill - 2`, **unclamped** (`AI_InterpolateThrustPoint`,
    /// `0x08852d14`, `a * (1 - t) + t * b`). An Easy skill under `1.0` therefore
    /// extrapolates past Point1: Talon's Junction's Venom Easy `0.9` gives an
    /// offset of `-7.7` from Point1's `-7.0`, read live.
    #[must_use]
    pub fn skill_terms(&self, skill: f32) -> (f32, f32) {
        let (lo, hi, t) = if skill < 2.0 {
            (self.skill_points[0], self.skill_points[1], skill - 1.0)
        } else {
            (self.skill_points[1], self.skill_points[2], skill - 2.0)
        };
        (
            lerp(lo.thrust_offset, hi.thrust_offset, t),
            lerp(lo.thrust_multiplier, hi.thrust_multiplier, t),
        )
    }

    /// The thrust, in percent of full, the original flies the **player's**
    /// craft at once it has finished in `place` (1-based) at skill `skill`.
    ///
    /// `AI_ComputeOpponentThrust` run on the player's own driver once its
    /// racer record's `finished` copy is set (confidence 90, measured
    /// 2026-10-04: first place `56.7`, fourth `56.0` on Venom Easy). The law
    /// aims the player about 250 units behind the craft one place ahead, so
    /// its position step sits on the lower stop, `-0.3 * AIThrust[place]`:
    ///
    /// ```text
    /// thrust = off + (0.7 * AIThrust[place] - AIThrust[1]) * mul + AIThrust[1]
    /// ```
    ///
    /// clamped to `1..=100` (the law's own floor, and the cap it applies to a
    /// craft near the player after 20 s of race, which a finished player
    /// always is). The step comes off that stop only when the player has
    /// fallen about 195 units behind its target, which needs the opponents'
    /// own `spread` wander (`FUN_08852ef4`) this project does not model: that
    /// regime is not reproduced. `place` outside `1..=8` reads as the nearest.
    #[must_use]
    pub fn finished_player_thrust(&self, place: u8, skill: f32) -> f32 {
        let index = usize::from(place.clamp(1, 8)) - 1;
        let first = self.ai_thrust[0];
        let own = self.ai_thrust[index];
        let (offset, multiplier) = self.skill_terms(skill);
        let lower_stop = own - own * 0.3;
        let thrust = offset + (lower_stop - first) * multiplier + first;
        thrust.clamp(1.0, 100.0)
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + t * b
}

/// Something wrong with an `AIRaceStats_<class>.xml` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The blob could not be expanded out of its shortened form.
    Expand(fexml::Error),
    /// A required element is absent.
    MissingElement {
        element: String,
    },
    /// A required attribute is absent or not a finite number.
    BadAttribute {
        element: String,
        attribute: &'static str,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expand(e) => write!(f, "could not expand: {e}"),
            Self::MissingElement { element } => write!(f, "no <{element}>"),
            Self::BadAttribute { element, attribute } => {
                write!(f, "<{element}> has no numeric {attribute}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<fexml::Error> for Error {
    fn from(e: fexml::Error) -> Self {
        Self::Expand(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// The archive entry name of `class`'s file.
#[must_use]
pub fn entry_name(class: SpeedClass) -> String {
    format!(
        r"Data\XML\AIRaceStats_{}.xml",
        class.as_str().to_ascii_lowercase()
    )
}

/// Reads an **already expanded** document for `class`. Use [`from_blob`] for
/// bytes straight out of an archive.
pub fn parse(expanded: &str, class: SpeedClass) -> Result<AiRaceStats> {
    let root = fexml::parse(expanded);
    let stats = child(&root, "AIStats")?;
    let class_element = format!("{}Stats", class_title(class));
    let class_node = child(stats, &class_element)?;
    let balancing = child(class_node, "RaceBalancing")?;

    let positions = child(balancing, "PosBalancing")?;
    let mut ai_thrust = [0.0; 8];
    for (index, value) in ai_thrust.iter_mut().enumerate() {
        let name = format!("PlayerInPos{}", index + 1);
        *value = number(child(positions, &name)?, &name, "AIThrust")?;
    }

    let skill = child(balancing, "SkillScale")?;
    let mut skill_points = [SkillPoint {
        thrust_offset: 0.0,
        thrust_multiplier: 1.0,
    }; 3];
    for (index, point) in skill_points.iter_mut().enumerate() {
        let name = format!("SkillScalePoint{}", index + 1);
        let node = child(skill, &name)?;
        *point = SkillPoint {
            thrust_offset: number(node, &name, "ThrustOffset")?,
            thrust_multiplier: number(node, &name, "ThrustMultiplier")?,
        };
    }
    Ok(AiRaceStats {
        ai_thrust,
        skill_points,
    })
}

/// [`parse`], off bytes straight out of an archive.
pub fn from_blob(data: &[u8], class: SpeedClass) -> Result<AiRaceStats> {
    parse(&fexml::text(data)?, class)
}

fn class_title(class: SpeedClass) -> &'static str {
    match class {
        SpeedClass::Venom => "Venom",
        SpeedClass::Flash => "Flash",
        SpeedClass::Rapier => "Rapier",
        SpeedClass::Phantom => "Phantom",
    }
}

fn child<'a>(node: &'a Node, name: &str) -> Result<&'a Node> {
    node.children
        .iter()
        .find(|c| c.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| Error::MissingElement {
            element: name.to_string(),
        })
}

fn number(node: &Node, element: &str, attribute: &'static str) -> Result<f32> {
    node.attr(attribute)
        .and_then(|raw| raw.trim().parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .ok_or_else(|| Error::BadAttribute {
            element: element.to_string(),
            attribute,
        })
}

#[cfg(test)]
mod tests;
