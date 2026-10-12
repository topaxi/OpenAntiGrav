//! Wipeout Pure's opponent tables, as Pure's executable reads them.
//!
//! Three files, `docs/ghidra/functions/psp-pure-usa/ai-controls.md`:
//!
//! - `Data\XML\AIRaceStats.xml`: five `<<Class>Stats>` blocks (Vector first),
//!   each twice, once with the legacy rows and once holding only `<NewStats>`.
//!   `AiStats_ParseClass` (`0x0891c738`) stores the two into disjoint halves of
//!   one record and every reader takes **the legacy half**: the switch that
//!   would select `<NewStats>` reads `0` in a race (measured 2026-10-12). So this
//!   reads `StartStats`, `ZoningStats`, `FatigueStats` and `GridSpread`, and
//!   skips `<NewStats>` and the root's `MasterSpeedTweak` (parsed and dropped by
//!   the original) by name.
//! - `Data\XML\AIControlStats.xml`: each class's `<Controller>`, Pulse's six
//!   attributes. `<ControllerPO>` is never parsed by the original (it matches
//!   element names exactly), so it is skipped by name.
//! - a track's `Data\Environments\<n>_<Name>\stats.xml` in `FEData.wad`:
//!   `<AIStats><<Class>Tweak Min=".." Max=".."/>`, the range the speed tweak is
//!   drawn from (`TrackStats` parser `0x088d9390`, read live off the race's
//!   settings object, 2026-10-12).
//!
//! Refuses by name an element or attribute the original's parser has no branch
//! for and the file does not ship, so a different schema fails loudly.

use crate::ai_control_stats::Controller;
use crate::ai_race_stats::{Error, Result, child, number, only_attributes};
use crate::fexml::{self, Node};

/// Pure's five speed classes, in the order its record array and its class index
/// (`0x08b173e0`) use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PureClass {
    Vector,
    Venom,
    Flash,
    Rapier,
    Phantom,
}

impl PureClass {
    /// All five, slowest first; the position is the original's class index.
    pub const ALL: [Self; 5] = [
        Self::Vector,
        Self::Venom,
        Self::Flash,
        Self::Rapier,
        Self::Phantom,
    ];

    /// The element-name stem the files use (`VectorStats`, `VectorTweak`).
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Vector => "Vector",
            Self::Venom => "Venom",
            Self::Flash => "Flash",
            Self::Rapier => "Rapier",
            Self::Phantom => "Phantom",
        }
    }
}

/// One `ZoningStats` curve: six points of `SpeedPercent` against `Position`,
/// the nearest opponent's distance ahead of the player (negative behind).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneCurve {
    /// `Position`, in the order the file numbers them.
    pub position: [f32; 6],
    /// `SpeedPercent / 100`, as `AiStats_ParseZoning` stores it.
    pub speed: [f32; 6],
}

/// `FatigueStats`, the anti-camping window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fatigue {
    /// `FatigueStartTime`, seconds inside the window before fatigue starts.
    pub start_time: f32,
    /// `FatigueDecayTime`, seconds over which it reaches its multiplier.
    pub decay_time: f32,
    /// `FatigueMultiplier`, the speed factor at full fatigue.
    pub multiplier: f32,
    /// `RestoreRate`, seconds of fatigue shed per second outside the window.
    pub restore_rate: f32,
    /// `FatigueZoneFront`, the window's far edge ahead of the player.
    pub zone_front: f32,
    /// `FatigueZoneBack`, its edge behind (negative).
    pub zone_back: f32,
}

/// One class's legacy block of `AIRaceStats.xml`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PureClassRace {
    /// `GridSpread distance`: the field's spread that ends the launch phase.
    pub grid_spread: f32,
    /// `NormalRaceSpeedMin`, per grid slot (`GridPlace1` first).
    pub cruise_min: [f32; 8],
    /// `NormalRaceSpeedMax`, per grid slot.
    pub cruise_max: [f32; 8],
    /// `StartSpeed`, per grid slot: the launch throttle.
    pub start_speed: [f32; 8],
    /// `Zone1..6`: the field's speed while the player does not lead.
    pub zone: ZoneCurve,
    /// `LeadZone1..6`: the same while the player leads.
    pub lead_zone: ZoneCurve,
    /// `FatigueStats`.
    pub fatigue: Fatigue,
}

/// The archive entry name of the race file.
#[must_use]
pub fn race_entry_name() -> &'static str {
    r"Data\XML\AIRaceStats.xml"
}

/// The archive entry name of the controller file.
#[must_use]
pub fn control_entry_name() -> &'static str {
    r"Data\XML\AIControlStats.xml"
}

/// Reads `class`'s legacy block from the text of `AIRaceStats.xml`.
pub fn parse_race(text: &str, class: PureClass) -> Result<PureClassRace> {
    let root = fexml::parse(text);
    let stats = child(&root, "AIStats")?;
    let name = format!("{}Stats", class.title());
    let mut start = None;
    let mut zones = None;
    let mut fatigue = None;
    let mut grid_spread = None;
    for block in stats
        .children
        .iter()
        .filter(|c| c.name.eq_ignore_ascii_case(&name))
    {
        for row in &block.children {
            match row.name.as_str() {
                "StartStats" => start = Some(start_stats(row)?),
                "ZoningStats" => zones = Some(zoning(row)?),
                "FatigueStats" => fatigue = Some(fatigue_stats(row)?),
                "GridSpread" => {
                    only_attributes(row, "GridSpread", &["distance"])?;
                    grid_spread = Some(number(row, "GridSpread", "distance")?);
                }
                // The half the original parses and never selects.
                "NewStats" => {}
                other => {
                    return Err(Error::UnknownElement {
                        element: other.to_string(),
                    });
                }
            }
        }
    }
    let missing = |element: &str| Error::MissingElement {
        element: element.to_string(),
    };
    let (cruise_min, cruise_max, start_speed) = start.ok_or_else(|| missing("StartStats"))?;
    let (zone, lead_zone) = zones.ok_or_else(|| missing("ZoningStats"))?;
    Ok(PureClassRace {
        grid_spread: grid_spread.ok_or_else(|| missing("GridSpread"))?,
        cruise_min,
        cruise_max,
        start_speed,
        zone,
        lead_zone,
        fatigue: fatigue.ok_or_else(|| missing("FatigueStats"))?,
    })
}

/// [`parse_race`], off bytes straight out of an archive.
pub fn race_from_blob(data: &[u8], class: PureClass) -> Result<PureClassRace> {
    parse_race(&fexml::text(data)?, class)
}

/// Reads `class`'s `<Controller>` from the text of `AIControlStats.xml`.
pub fn parse_controller(text: &str, class: PureClass) -> Result<Controller> {
    let root = fexml::parse(text);
    let stats = child(&root, "AIStats")?;
    let block = child(stats, &format!("{}Stats", class.title()))?;
    for row in &block.children {
        if !row.name.eq_ignore_ascii_case("Controller") && row.name != "ControllerPO" {
            return Err(Error::UnknownElement {
                element: row.name.clone(),
            });
        }
    }
    crate::ai_control_stats::controller(child(block, "Controller")?)
}

/// [`parse_controller`], off bytes straight out of an archive.
pub fn controller_from_blob(data: &[u8], class: PureClass) -> Result<Controller> {
    parse_controller(&fexml::text(data)?, class)
}

/// The speed tweak's range for `class` out of a track's `stats.xml`, as
/// `(Min, Max)`. The original draws `Min + u * (Max - Min)` once a race
/// (`AiStats_LoadAll`, `0x0891c314`); the shipped files author `Min >= Max`.
pub fn parse_tweak(text: &str, class: PureClass) -> Result<(f32, f32)> {
    let root = fexml::parse(text);
    let stats = child(&root, "Stats")?;
    let ai = child(stats, "AIStats")?;
    let element = format!("{}Tweak", class.title());
    let row = child(ai, &element)?;
    only_attributes(row, &element, &["Min", "Max"])?;
    Ok((number(row, &element, "Min")?, number(row, &element, "Max")?))
}

/// [`parse_tweak`], off bytes straight out of an archive.
pub fn tweak_from_blob(data: &[u8], class: PureClass) -> Result<(f32, f32)> {
    parse_tweak(&fexml::text(data)?, class)
}

/// `StartStats`: `GridPlace1..8`, each `NormalRaceSpeedMin`/`Max` and `StartSpeed`.
fn start_stats(node: &Node) -> Result<([f32; 8], [f32; 8], [f32; 8])> {
    let mut min = [None; 8];
    let mut max = [None; 8];
    let mut start = [None; 8];
    for row in &node.children {
        let slot = numbered(row, "GridPlace", 8)?;
        only_attributes(
            row,
            &row.name,
            &["NormalRaceSpeedMin", "NormalRaceSpeedMax", "StartSpeed"],
        )?;
        min[slot] = Some(number(row, &row.name, "NormalRaceSpeedMin")?);
        max[slot] = Some(number(row, &row.name, "NormalRaceSpeedMax")?);
        start[slot] = Some(number(row, &row.name, "StartSpeed")?);
    }
    Ok((
        filled(min, "GridPlace")?,
        filled(max, "GridPlace")?,
        filled(start, "GridPlace")?,
    ))
}

/// `ZoningStats`: `Zone1..6` and `LeadZone1..6`. The legacy parser has no
/// `TailZone` branch, so a `TailZone` here is refused.
fn zoning(node: &Node) -> Result<(ZoneCurve, ZoneCurve)> {
    let mut zone = ([None; 6], [None; 6]);
    let mut lead = ([None; 6], [None; 6]);
    for row in &node.children {
        let (curve, stem) = if row.name.starts_with("LeadZone") {
            (&mut lead, "LeadZone")
        } else {
            (&mut zone, "Zone")
        };
        let point = numbered(row, stem, 6)?;
        only_attributes(row, &row.name, &["Position", "SpeedPercent"])?;
        curve.0[point] = Some(number(row, &row.name, "Position")?);
        curve.1[point] = Some(number(row, &row.name, "SpeedPercent")? * 0.01);
    }
    Ok((
        ZoneCurve {
            position: filled(zone.0, "Zone")?,
            speed: filled(zone.1, "Zone")?,
        },
        ZoneCurve {
            position: filled(lead.0, "LeadZone")?,
            speed: filled(lead.1, "LeadZone")?,
        },
    ))
}

/// `FatigueStats`: six single-`val` rows.
fn fatigue_stats(node: &Node) -> Result<Fatigue> {
    let read = |name: &'static str| -> Result<f32> {
        let row = child(node, name)?;
        only_attributes(row, name, &["val"])?;
        number(row, name, "val")
    };
    for row in &node.children {
        if ![
            "FatigueStartTime",
            "FatigueDecayTime",
            "FatigueMultiplier",
            "RestoreRate",
            "FatigueZoneFront",
            "FatigueZoneBack",
        ]
        .contains(&row.name.as_str())
        {
            return Err(Error::UnknownElement {
                element: row.name.clone(),
            });
        }
    }
    Ok(Fatigue {
        start_time: read("FatigueStartTime")?,
        decay_time: read("FatigueDecayTime")?,
        multiplier: read("FatigueMultiplier")?,
        restore_rate: read("RestoreRate")?,
        zone_front: read("FatigueZoneFront")?,
        zone_back: read("FatigueZoneBack")?,
    })
}

/// `<stem><N>` with `N` in `1..=count`, as the zero-based index. The original
/// reads the digit after the stem and requires nothing after it.
fn numbered(row: &Node, stem: &str, count: usize) -> Result<usize> {
    row.name
        .strip_prefix(stem)
        .and_then(|digit| digit.parse::<usize>().ok())
        .filter(|n| (1..=count).contains(n))
        .map(|n| n - 1)
        .ok_or_else(|| Error::UnknownElement {
            element: row.name.clone(),
        })
}

fn filled<const N: usize>(values: [Option<f32>; N], stem: &str) -> Result<[f32; N]> {
    let mut out = [0.0; N];
    for (k, value) in values.iter().enumerate() {
        out[k] = value.ok_or_else(|| Error::MissingElement {
            element: format!("{stem}{}", k + 1),
        })?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
