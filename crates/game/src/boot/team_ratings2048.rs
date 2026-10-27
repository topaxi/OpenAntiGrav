//! The Select Team panel's numbers: `<FE>` of every native craft's
//! `Data\HandlingStats\<team>\<n>\handlingstats.xml`.
//!
//! Read off the disc, never transcribed: the panel's bars and digits are these
//! files (`docs/ghidra/functions/vita-2048-eu-v104/team-grid.md`). A file that
//! is missing, unreadable or carries no `<FE>`/`firepower` gives that craft no
//! rows and a report line; the panel never draws a zero bar for it.

use oag_2048::race::{HANDLING_DIR, NATIVE_TEAMS, TEAM_VARIANTS};
use oag_ui::frontend::TeamRating;

/// Every native craft's rating, in `NATIVE_TEAMS` then variant order. Empty off
/// 2048 (`touch` is false there), which has no such directory.
pub fn load(
    touch: bool,
    archives: &mut oag_assets::Archives,
    report: &mut Vec<String>,
) -> Vec<TeamRating> {
    if !touch {
        return Vec::new();
    }
    let mut ratings = Vec::new();
    for team in NATIVE_TEAMS {
        for (craft, variant) in TEAM_VARIANTS.variants.iter().enumerate() {
            let name = format!(
                r"{HANDLING_DIR}\{team}\{}\handlingstats.xml",
                variant.suffix
            );
            match read(archives, &name) {
                Ok(rows) => ratings.push(TeamRating { team, craft, rows }),
                Err(why) => report.push(format!("team panel: {name}: {why}")),
            }
        }
    }
    report.push(format!(
        "team panel: {} of {} craft ratings read",
        ratings.len(),
        NATIVE_TEAMS.len() * TEAM_VARIANTS.variants.len()
    ));
    ratings
}

/// `speed`, `handling`, `shield`, `firepower` of one file's `<FE>`.
fn read(archives: &mut oag_assets::Archives, name: &str) -> Result<[f32; 4], String> {
    let blob = archives.read_name(name).map_err(|e| e.to_string())?;
    let stats = oag_tables::handling::from_blob(&blob).map_err(|e| e.to_string())?;
    let fe = stats.fe.ok_or("no <FE>")?;
    let firepower = fe.firepower.ok_or("no firepower in <FE>")?;
    Ok([fe.speed, fe.handling, fe.shield, firepower])
}
