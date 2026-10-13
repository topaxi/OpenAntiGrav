//! What the original opponent (`crate::original_ai`) reads off the
//! disc for one race, in whichever shape the title ships its AI tables
//! ([`oag_title::original_driver::AiTables`]):
//!
//! - one `AIRaceStats_<class>.xml` a class and `AIControlStats.xml` (Pulse,
//!   HD, 2048, Omega): the place-indexed throttle and the formation;
//! - one `AIRaceStats.xml` whose legacy block Pure's executable reads, its
//!   `AIControlStats.xml` and the circuit's own `stats.xml` speed tweak
//!   (`docs/ghidra/functions/psp-pure-usa/ai-controls.md`): the cruise
//!   throttle.
//!
//! Read only for a title that flies the original opponent
//! (`oag_title::RaceDefaults::original_opponent`); `None`, said so in the
//! loader report, when anything does not read, and the race keeps this
//! project's driver. Read for every race of such a title, whatever its mode:
//! `oag_game::ai_choice` decides whether the opponents fly it.

use oag_tables::ai_control_stats;
use oag_tables::ai_race_stats::{self, ClassRace};
use oag_tables::handling::SpeedClass;
use oag_tables::pure_ai_stats::{self, PureClass, PureClassRace};
use oag_title::original_driver::AiTables;

/// One race's original-opponent tables.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OriginalTables {
    /// The throttle's tables.
    pub throttle: Throttle,
    /// `AIControlStats.xml`'s row over the executable's defaults.
    pub controller: oag_ai::original::Controller,
    /// The title's variant of the driver for this race.
    pub laws: oag_ai::original::Laws,
}

/// The tables the throttle reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Throttle {
    /// `AIRaceStats_<class>.xml`, whole.
    Formation(ClassRace),
    /// Pure's legacy block and the circuit's tweak range, `(Min, Max)`.
    Cruise {
        /// The class's legacy block.
        table: PureClassRace,
        /// `<AIStats><<Class>Tweak Min Max>` from the circuit's `stats.xml`.
        tweak: (f32, f32),
    },
}

impl OriginalTables {
    /// The field the race flies, seated on `grid` and seeded from `seed`.
    #[must_use]
    pub fn field(
        &self,
        skill: f32,
        level: usize,
        grid: [u8; oag_ai::original::SLOTS],
        seed: u64,
    ) -> oag_ai::original::Field {
        match self.throttle {
            Throttle::Formation(table) => oag_ai::original::Field::new(
                table,
                self.controller,
                self.laws,
                skill,
                level,
                grid,
                seed,
            ),
            Throttle::Cruise { table, tweak } => oag_ai::original::Field::cruising(
                table,
                tweak,
                self.controller,
                self.laws,
                grid,
                seed,
            ),
        }
    }
}

/// This race's tables, or `None` with the reason in `report`. `track` is
/// the race's `.vex` entry, whose directory holds Pure's `stats.xml`.
pub(super) fn read(
    archives: &mut oag_assets::Archives,
    title: &oag_title::Title,
    class: &str,
    track: &str,
    laws: oag_ai::original::Laws,
    report: &mut Vec<String>,
) -> Option<OriginalTables> {
    if !title.race.original_opponent {
        report.push(
            "original AI: not wired for this title; its races keep this project's driver"
                .to_string(),
        );
        return None;
    }
    let shape = title
        .race
        .original_driver
        .map_or(AiTables::PerClassFiles, |driver| driver.tables);
    let read = match shape {
        AiTables::PerClassFiles => per_class(archives, class, laws),
        AiTables::LegacyBlock => legacy_block(archives, class, track, laws),
    };
    match read {
        Ok(tables) => {
            let c = tables.controller;
            report.push(format!(
                "original AI: SteerMul {} SteerDamp {} LookAheadSecs {} xtrackMul {}{}",
                c.steer_mul,
                c.steer_damp,
                c.look_ahead_secs,
                c.xtrack_mul,
                match tables.throttle {
                    Throttle::Formation(_) => String::new(),
                    Throttle::Cruise { table, tweak } => format!(
                        ", the legacy block's cruise speeds {:?}..{:?}, tweak {tweak:?}, \
                         GridSpread {}",
                        table.cruise_min, table.cruise_max, table.grid_spread
                    ),
                }
            ));
            Some(tables)
        }
        Err(e) => {
            report.push(format!(
                "original AI: {e}; the race keeps this project's driver"
            ));
            None
        }
    }
}

/// Pulse's shape: the class's own `AIRaceStats` file and the controller row.
fn per_class(
    archives: &mut oag_assets::Archives,
    class: &str,
    laws: oag_ai::original::Laws,
) -> Result<OriginalTables, String> {
    let class = SpeedClass::named(class)
        .ok_or_else(|| format!("class {class:?} has no AIRaceStats file"))?;
    let entry = ai_race_stats::entry_name(class);
    let race = archives
        .read_name(&entry)
        .map_err(|e| format!("{entry} did not read ({e})"))
        .and_then(|blob| {
            ai_race_stats::full_from_blob(&blob, class)
                .map_err(|e| format!("{entry} did not read whole ({e})"))
        })?;
    let entry = ai_control_stats::entry_name();
    let row = archives
        .read_name(entry)
        .map_err(|e| e.to_string())
        .and_then(|blob| ai_control_stats::from_blob(&blob, class).map_err(|e| e.to_string()))
        .map_err(|e| format!("{entry} did not read ({e})"))?;
    Ok(OriginalTables {
        throttle: Throttle::Formation(race),
        controller: oag_ai::original::Controller::from_disc(&row),
        laws,
    })
}

/// Pure's shape: the legacy block, its controller, and the circuit's tweak.
fn legacy_block(
    archives: &mut oag_assets::Archives,
    class: &str,
    track: &str,
    laws: oag_ai::original::Laws,
) -> Result<OriginalTables, String> {
    let class = PureClass::ALL
        .into_iter()
        .find(|c| c.title().eq_ignore_ascii_case(class))
        .ok_or_else(|| format!("class {class:?} has no legacy block"))?;
    let mut blob = |entry: &str| {
        archives
            .read_name(entry)
            .map_err(|e| format!("{entry} did not read ({e})"))
    };
    let race_entry = pure_ai_stats::race_entry_name();
    let table = pure_ai_stats::race_from_blob(&blob(race_entry)?, class)
        .map_err(|e| format!("{race_entry} did not parse ({e})"))?;
    let control_entry = pure_ai_stats::control_entry_name();
    let row = pure_ai_stats::controller_from_blob(&blob(control_entry)?, class)
        .map_err(|e| format!("{control_entry} did not parse ({e})"))?;
    let directory = track
        .rsplit_once('\\')
        .map(|(directory, _)| directory)
        .ok_or_else(|| format!("{track} names no circuit directory"))?;
    let stats = format!(r"{directory}\stats.xml");
    let tweak = pure_ai_stats::tweak_from_blob(&blob(&stats)?, class)
        .map_err(|e| format!("{stats} has no tweak ({e})"))?;
    Ok(OriginalTables {
        throttle: Throttle::Cruise { table, tweak },
        controller: oag_ai::original::Controller::from_disc(&row),
        laws,
    })
}
