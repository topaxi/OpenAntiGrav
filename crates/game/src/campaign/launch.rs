//! What a confirmed `Cell Selection` cell asks a race for, before any race
//! option is touched.
//!
//! The mapping is pure - a cell and a way to resolve a circuit id in, a plan
//! out - so it lives here in the library rather than inside the session that
//! applies it, which is what lets a disc-backed test hold it to every cell a
//! title ships (`tests/omega_campaign_launch_ground_truth.rs`). Every title
//! whose campaign launches goes through this one function; nothing in it names
//! a title.

use oag_tables::race_campaign::Cell;

/// What a cell asks the race for. The session copies these into
/// [`oag_raceplay::Options`]; a value here is the cell's own, resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct CellPlan {
    /// The mode the cell's own [`oag_tables::race_campaign::Mode`] launches as.
    pub mode: oag_race::Mode,
    /// The archive entry of every circuit the cell races, in order: one for a
    /// plain cell, one per leg for a `Tournament`. Never empty.
    pub leg_entries: Vec<String>,
    /// The speed class the race runs at - the cell's own, or `fallback` for a
    /// `Zone` cell whose `class` is not a speed class.
    pub class: String,
    /// Whether `class` is the caller's fallback rather than the cell's own.
    /// **Chosen, not measured**: nothing here resolves a Zone handling block.
    pub class_is_fallback: bool,
    /// The lap count to force, only for the modes whose own
    /// `Mode::laps_target` already returns `Some` - see
    /// [`oag_raceplay::Options::laps_override`].
    pub laps_override: Option<u32>,
    /// `Elimination`'s kill target, which is the cell's gold target.
    pub eliminator_kill_target: Option<u32>,
    /// The opponents the cell authors (`AICount`), for the report and the
    /// tests: the race's field is the mode's own, and the ground truth holds
    /// the two to agree on every shipped cell rather than this being applied.
    pub ai_count: Option<u32>,
}

/// Why a cell cannot launch. `Display` is the line the session logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The cell's mode is one this engine cannot run.
    Mode(String),
    /// The cell names no circuit at all.
    NoTrack,
    /// The cell names a circuit the source does not offer.
    MissingTrack(String),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mode(mode) => write!(
                f,
                "{mode} is not one of the modes this engine can run yet, so this cell cannot launch"
            ),
            Self::NoTrack => f.write_str("the cell names no track - cannot launch"),
            Self::MissingTrack(id) => {
                write!(
                    f,
                    "this source does not offer {id:?} - the cell cannot launch"
                )
            }
        }
    }
}

/// Resolves `cell` against a source: `track_entry` turns a circuit id into its
/// archive entry for a mode (or `None` when the source does not offer it), and
/// `fallback_class` is what a `Zone` cell races at.
///
/// A `Tournament` cell names its legs through
/// [`Cell::tournament_tracks`]; every leg is resolved before anything is
/// returned, so a tournament is never silently shortened by a missing leg.
///
/// # Errors
///
/// A [`Refusal`] when the mode cannot run, no circuit is named, or a circuit
/// does not resolve.
pub fn plan_cell(
    cell: &Cell,
    fallback_class: &str,
    track_entry: impl Fn(oag_race::Mode, &str) -> Option<String>,
) -> Result<CellPlan, Refusal> {
    let mode = super::race_mode_for_cell(cell.mode.clone())
        .ok_or_else(|| Refusal::Mode(cell.mode.to_string()))?;
    let track_ids: Vec<&String> = if mode == oag_race::Mode::Tournament {
        cell.tournament_tracks.iter().collect()
    } else {
        cell.track.iter().collect()
    };
    if track_ids.is_empty() {
        return Err(Refusal::NoTrack);
    }
    let leg_entries = track_ids
        .iter()
        .map(|id| track_entry(mode, id).ok_or_else(|| Refusal::MissingTrack((*id).clone())))
        .collect::<Result<Vec<_>, _>>()?;
    let (class, class_is_fallback) = match cell.speed_class() {
        Some(_) => (cell.class.clone(), false),
        None => (fallback_class.to_string(), true),
    };
    let laps_override = matches!(
        mode,
        oag_race::Mode::TimeTrial
            | oag_race::Mode::SingleRace
            | oag_race::Mode::Tournament
            | oag_race::Mode::Head2Head
    )
    .then_some(cell.laps)
    .flatten();
    // An HD cell's `gold` is the dummy `1` where its real target is the
    // unmeasured `NitroElim*` number, so the race keeps its own default kill
    // target (Pulse's law, unmeasured on HD) rather than ending on the first kill.
    let eliminator_kill_target = (mode == oag_race::Mode::Eliminator
        && !cell.medal_law_is_unmeasured())
    .then(|| u32::try_from(cell.gold).ok())
    .flatten();
    Ok(CellPlan {
        mode,
        leg_entries,
        class,
        class_is_fallback,
        laps_override,
        eliminator_kill_target,
        ai_count: cell.ai_count,
    })
}
