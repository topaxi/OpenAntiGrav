//! `Data\XML\AIControlStats.xml`: the steering controller's per-class constants.
//!
//! One `<<Class>Stats>` element a speed class, each holding a `<Controller>`
//! row. `AiStats_ParseController` (`0x08836548`) stores six 32-bit floats at the
//! head of the class record: `SteerMul`, `SteerDamp`, `LookAheadSecs`,
//! `xtrackMul`, `xtrackMax`, `xtrackDamp`. `Ai_ComputeControls` reads the first
//! four; `xtrackMax` and `xtrackDamp` have no found reader. The schema, the
//! order and the consumer are `docs/ghidra/functions/psp-pulse-usa/ai-controls.md`
//! and `ai-stats.md`.
//!
//! **A field the file does not author is `None`, not a number**: the original's
//! parser pre-fills each class with executable constants before it reads
//! (`AiStats_SetDefaults`), and those are the consumer's to apply, not this
//! reader's to invent. Shipped Pulse files omit `xtrackMax`.
//!
//! Refuses by name: an attribute outside the six, an element beside
//! `Controller` that is not the other family's `ControllerPO` (HD's `_kart`
//! file, a different controller no Pulse code reads), a missing class.

use crate::ai_race_stats::{Error, Result, child, only_attributes};
use crate::fexml::{self, Node};
use crate::handling::SpeedClass;

/// One class's `<Controller>` row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Controller {
    /// `SteerMul`: scales the steering output.
    pub steer_mul: Option<f32>,
    /// `SteerDamp`: weight of the heading-error derivative.
    pub steer_damp: Option<f32>,
    /// `LookAheadSecs`: how far ahead along the line the target is taken.
    pub look_ahead_secs: Option<f32>,
    /// `xtrackMul`: cross-track error gain.
    pub xtrack_mul: Option<f32>,
    /// `xtrackMax`: no reader found.
    pub xtrack_max: Option<f32>,
    /// `xtrackDamp`: no reader found.
    pub xtrack_damp: Option<f32>,
}

const ATTRIBUTES: &[&str] = &[
    "SteerMul",
    "SteerDamp",
    "LookAheadSecs",
    "xtrackMul",
    "xtrackMax",
    "xtrackDamp",
];

/// The archive entry name of the file.
#[must_use]
pub fn entry_name() -> &'static str {
    r"Data\XML\AIControlStats.xml"
}

/// Reads `class`'s `<Controller>` from an expanded document.
pub fn parse(expanded: &str, class: SpeedClass) -> Result<Controller> {
    parse_element(
        expanded,
        &format!("{}Stats", crate::ai_race_stats::class_title(class)),
    )
}

/// [`parse`], off bytes straight out of an archive.
pub fn from_blob(data: &[u8], class: SpeedClass) -> Result<Controller> {
    parse(&fexml::text(data)?, class)
}

/// Reads the `<Controller>` of the named class element (`VenomStats`, or 2048's
/// `SuperPhantomStats`) from an expanded document.
pub fn parse_element(expanded: &str, element: &str) -> Result<Controller> {
    let root = fexml::parse(expanded);
    let stats = child(&root, "AIStats")?;
    let class = child(stats, element)?;
    if let Some(other) = class
        .children
        .iter()
        .find(|c| !c.name.eq_ignore_ascii_case("Controller"))
    {
        return Err(Error::UnknownElement {
            element: other.name.clone(),
        });
    }
    controller(child(class, "Controller")?)
}

fn controller(node: &Node) -> Result<Controller> {
    only_attributes(node, "Controller", ATTRIBUTES)?;
    let read = |attribute: &'static str| -> Result<Option<f32>> {
        match node.attr(attribute) {
            None => Ok(None),
            Some(raw) => raw
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|v| v.is_finite())
                .map(Some)
                .ok_or_else(|| Error::BadAttribute {
                    element: "Controller".to_string(),
                    attribute,
                }),
        }
    };
    Ok(Controller {
        steer_mul: read("SteerMul")?,
        steer_damp: read("SteerDamp")?,
        look_ahead_secs: read("LookAheadSecs")?,
        xtrack_mul: read("xtrackMul")?,
        xtrack_max: read("xtrackMax")?,
        xtrack_damp: read("xtrackDamp")?,
    })
}

#[cfg(test)]
mod tests;
