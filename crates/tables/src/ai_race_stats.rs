//! `Data\XML\AIRaceStats_<class>.xml`: the AI's position-balancing thrust
//! table, and the one law of the original's that this project reads it for.
//!
//! Pulse ships one file a speed class (`AIRaceStats_venom.xml`, `_flash`,
//! `_rapier`, `_phantom`) in `Data.wad`, each holding one `<<Class>Stats>`
//! with a `<RaceBalancing>` block; `AiStats_LoadAll` (`0x08835830`) loads all
//! four into a per-class record. The layout and the parser are
//! `docs/ghidra/functions/psp-pulse-usa/ai-stats.md` (confidence 88).
//!
//! **Two views of one file.** [`AiRaceStats`] / [`parse`] read `PosBalancing`'s
//! eight `AIThrust` figures and `SkillScale`'s three `ThrustOffset` /
//! `ThrustMultiplier` pairs: the inputs of the thrust the original gives the
//! **player's own craft after the finish line**
//! ([`AiRaceStats::finished_player_thrust`], measured 2026-10-04,
//! `docs/ghidra/functions/psp-pulse-usa/race-finish.md`). [`ClassRace`] /
//! [`parse_class_race`] read the whole `<RaceBalancing>` (`SpreadDist`, the six
//! `RubberBanding` figures, `StartStats`, the skill curve's `SpreadMultiplier`,
//! `AIPackSwapping`, HD's `SplitScreenMultiplier`) for the original's opponent
//! (`docs/gameplay/ai.md`, "The original driver, recovered"), and refuses by name
//! a file or row it does not know. The census and the cross-title verdicts are
//! `docs/formats/ai-tables.md`.

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
        /// The element that was looked for.
        element: String,
    },
    /// An element the full reader does not know sits where it reads.
    UnknownElement {
        /// The element's name.
        element: String,
    },
    /// An attribute the full reader does not know sits on a row it reads.
    UnknownAttribute {
        /// The row's element.
        element: String,
        /// The attribute's name.
        attribute: String,
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
            Self::UnknownElement { element } => write!(f, "unknown <{element}>"),
            Self::UnknownAttribute { element, attribute } => {
                write!(f, "<{element}> has unknown attribute {attribute}")
            }
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
    parse_node(child(class_node, "RaceBalancing")?)
}

fn parse_node(balancing: &Node) -> Result<AiRaceStats> {
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

pub(crate) fn class_title(class: SpeedClass) -> &'static str {
    match class {
        SpeedClass::Venom => "Venom",
        SpeedClass::Flash => "Flash",
        SpeedClass::Rapier => "Rapier",
        SpeedClass::Phantom => "Phantom",
    }
}

pub(crate) fn child<'a>(node: &'a Node, name: &str) -> Result<&'a Node> {
    node.children
        .iter()
        .find(|c| c.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| Error::MissingElement {
            element: name.to_string(),
        })
}

pub(crate) fn number(node: &Node, element: &str, attribute: &'static str) -> Result<f32> {
    node.attr(attribute)
        .and_then(|raw| raw.trim().parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .ok_or_else(|| Error::BadAttribute {
            element: element.to_string(),
            attribute,
        })
}

/// One `<GridPlace<N>>` of `<StartStats>`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridPlace {
    /// `StartBoost`: the launch-window figure `Ship_UpdateStartBoost` reads.
    pub start_boost: f32,
    /// `BaseStartThrust`, as the file spells it. **The original's parser looks
    /// for `BaseThrust` and so never stores this** (`ai-stats.md`); it is read
    /// here because the file authors it, not because anything consumes it.
    pub base_start_thrust: f32,
}

/// One row of `<RubberBanding>`: `WhenLeading` or `WhenBehind`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RubberRow {
    /// `SpeedMul`: thrust per unit of gap past the dead band.
    pub speed_mul: f32,
    /// `MinSpeedChangeDist`: the dead band.
    pub min_speed_change_dist: f32,
    /// `MaxSpeedChange`: the clamp on the adjustment.
    pub max_speed_change: f32,
}

/// One `<AIPackSwapping<N>>` row, the pack shuffle's counts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PackSwapping {
    /// `numAIPositionSwaps`; the original casts it to an integer.
    pub num_ai_position_swaps: f32,
    /// `numIntermediatePositions`; the original casts it to an integer.
    pub num_intermediate_positions: f32,
    /// `intermediateOffsetSize`.
    pub intermediate_offset_size: f32,
}

/// The `<SplitScreenMultiplier>` element HD, 2048 and Omega add: one figure a
/// difficulty. Pulse's files do not author it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitScreenMultiplier {
    /// `Easy`.
    pub easy: f32,
    /// `Medium`.
    pub medium: f32,
    /// `Hard`.
    pub hard: f32,
}

/// Everything one class's `<RaceBalancing>` authors.
///
/// [`AiRaceStats`] stays the thrust view; this holds the whole file, which the
/// original's opponent reads (`docs/ghidra/functions/psp-pulse-usa/ai-controls.md`).
/// Names, structure and what each field governs are on `ai-stats.md`'s layout
/// table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClassRace {
    /// `PosBalancing` thrust and the skill curve, as [`parse`] reads them.
    pub thrust: AiRaceStats,
    /// `<PlayerInPos1..8 SpreadDist>`, indexed by place - 1.
    pub spread_dist: [f32; 8],
    /// `<StartStats><GridPlace1..8>`, indexed by grid slot.
    pub grid: [GridPlace; 8],
    /// `<RubberBanding><WhenLeading>`.
    pub when_leading: RubberRow,
    /// `<RubberBanding><WhenBehind>`.
    pub when_behind: RubberRow,
    /// `<SkillScalePoint1..3 SpreadMultiplier>`.
    pub spread_multiplier: [f32; 3],
    /// `<AIPackSwapping1..3>`.
    pub pack_swapping: [PackSwapping; 3],
    /// `<SplitScreenMultiplier>`, absent in Pulse's files.
    pub split_screen: Option<SplitScreenMultiplier>,
}

/// The children of `<RaceBalancing>` [`parse_class_race`] knows.
const BALANCING_CHILDREN: &[&str] = &[
    "PosBalancing",
    "RubberBanding",
    "StartStats",
    "SkillScale",
    "SplitScreenMultiplier",
];

/// [`parse_class_race`] for `class`'s own element.
pub fn parse_full(expanded: &str, class: SpeedClass) -> Result<ClassRace> {
    parse_class_race(expanded, &format!("{}Stats", class_title(class)))
}

/// [`parse_full`], off bytes straight out of an archive.
pub fn full_from_blob(data: &[u8], class: SpeedClass) -> Result<ClassRace> {
    parse_full(&fexml::text(data)?, class)
}

/// Reads the whole `<RaceBalancing>` of the `<element>` class in an expanded
/// document (`element` is e.g. `VenomStats`, or 2048's `SuperPhantomStats`).
///
/// A reader that refuses by name: an element under `<RaceBalancing>` outside
/// [`BALANCING_CHILDREN`] (HD's combined file authors `ShipSpacingVariation`), an
/// attribute on a row it reads that it does not know, or a missing one, is an
/// [`Error`] and not a skipped field.
pub fn parse_class_race(expanded: &str, element: &str) -> Result<ClassRace> {
    let root = fexml::parse(expanded);
    let stats = child(&root, "AIStats")?;
    let class_node = child(stats, element)?;
    let balancing = child(class_node, "RaceBalancing")?;
    if let Some(other) = balancing.children.iter().find(|c| {
        !BALANCING_CHILDREN
            .iter()
            .any(|n| c.name.eq_ignore_ascii_case(n))
    }) {
        return Err(Error::UnknownElement {
            element: other.name.clone(),
        });
    }
    let thrust = parse_node(balancing)?;

    let positions = child(balancing, "PosBalancing")?;
    let mut spread_dist = [0.0; 8];
    for (index, value) in spread_dist.iter_mut().enumerate() {
        let name = format!("PlayerInPos{}", index + 1);
        let node = child(positions, &name)?;
        only_attributes(node, &name, &["AIThrust", "SpreadDist"])?;
        *value = number(node, &name, "SpreadDist")?;
    }

    let rubber = child(balancing, "RubberBanding")?;
    let row = |name: &str| -> Result<RubberRow> {
        let node = child(rubber, name)?;
        only_attributes(
            node,
            name,
            &["SpeedMul", "MinSpeedChangeDist", "MaxSpeedChange"],
        )?;
        Ok(RubberRow {
            speed_mul: number(node, name, "SpeedMul")?,
            min_speed_change_dist: number(node, name, "MinSpeedChangeDist")?,
            max_speed_change: number(node, name, "MaxSpeedChange")?,
        })
    };
    let (when_leading, when_behind) = (row("WhenLeading")?, row("WhenBehind")?);

    let starts = child(balancing, "StartStats")?;
    let mut grid = [GridPlace {
        start_boost: 0.0,
        base_start_thrust: 0.0,
    }; 8];
    for (index, place) in grid.iter_mut().enumerate() {
        let name = format!("GridPlace{}", index + 1);
        let node = child(starts, &name)?;
        only_attributes(node, &name, &["StartBoost", "BaseStartThrust"])?;
        *place = GridPlace {
            start_boost: number(node, &name, "StartBoost")?,
            base_start_thrust: number(node, &name, "BaseStartThrust")?,
        };
    }

    let skill = child(balancing, "SkillScale")?;
    let mut spread_multiplier = [0.0; 3];
    let mut pack_swapping = [PackSwapping {
        num_ai_position_swaps: 0.0,
        num_intermediate_positions: 0.0,
        intermediate_offset_size: 0.0,
    }; 3];
    for index in 0..3 {
        let name = format!("SkillScalePoint{}", index + 1);
        let node = child(skill, &name)?;
        only_attributes(
            node,
            &name,
            &["ThrustOffset", "ThrustMultiplier", "SpreadMultiplier"],
        )?;
        spread_multiplier[index] = number(node, &name, "SpreadMultiplier")?;
        let name = format!("AIPackSwapping{}", index + 1);
        let node = child(skill, &name)?;
        only_attributes(
            node,
            &name,
            &[
                "numAIPositionSwaps",
                "numIntermediatePositions",
                "intermediateOffsetSize",
            ],
        )?;
        pack_swapping[index] = PackSwapping {
            num_ai_position_swaps: number(node, &name, "numAIPositionSwaps")?,
            num_intermediate_positions: number(node, &name, "numIntermediatePositions")?,
            intermediate_offset_size: number(node, &name, "intermediateOffsetSize")?,
        };
    }

    let split_screen = match child(balancing, "SplitScreenMultiplier") {
        Ok(node) => {
            only_attributes(node, "SplitScreenMultiplier", &["Easy", "Medium", "Hard"])?;
            Some(SplitScreenMultiplier {
                easy: number(node, "SplitScreenMultiplier", "Easy")?,
                medium: number(node, "SplitScreenMultiplier", "Medium")?,
                hard: number(node, "SplitScreenMultiplier", "Hard")?,
            })
        }
        Err(_) => None,
    };
    Ok(ClassRace {
        thrust,
        spread_dist,
        grid,
        when_leading,
        when_behind,
        spread_multiplier,
        pack_swapping,
        split_screen,
    })
}

/// Refuses an attribute of `node` outside `known`, naming it.
pub(crate) fn only_attributes(node: &Node, element: &str, known: &[&str]) -> Result<()> {
    for (key, _) in &node.attrs {
        if !known.iter().any(|k| k.eq_ignore_ascii_case(key)) {
            return Err(Error::UnknownAttribute {
                element: element.to_string(),
                attribute: key.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
