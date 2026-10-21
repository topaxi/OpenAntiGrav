//! The circuit's own `stats.xml`: the authored `<RaceTimes>` and `<LapTimes>`
//! the HUD's `RECORD` readout counts down to.
//!
//! Pulse's PSP disc keeps it in `FEData.wad`, beside the per-track front-end
//! screens, **not** in the `Data.wad` the race reads its geometry from -
//! `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s `TrackStats_Load`
//! opens `"<track directory>\stats.xml"` (`stats_reversed.xml` for a reversed
//! circuit). Every other pressing keeps it among the race's own archives under
//! the same entry name: Pulse PS2, and HD, whose circuits author
//! `/data/environments/<dir>/stats.xml` (`<RaceTimes Venom="599" .../>` on the
//! thirteen Time Trial circuits, measured as the `RECORD` the original counts
//! down from; `docs/ui/hud.md`, "HD's RECORD"). The campaign's AI-skill lookup
//! reads the same file the same way (`Session::resolve_campaign_ai_skill_scale`).

use oag_tables::track_stats::{self, TrackStats};

/// The stats for `track` (a `.vex` entry name), or `None` where the file is
/// absent or unreadable. Says which in `report`. `in_fedata` is Pulse's PSP
/// disc, whose file sits in `FEData.wad`; every other title reads it out of the
/// race's own `archives`.
pub(super) fn read(
    source: &str,
    archives: &mut oag_assets::Archives,
    track: &str,
    in_fedata: bool,
    report: &mut Vec<String>,
) -> Option<TrackStats> {
    let track = track.replace('/', "\\");
    let (directory, file) = track.trim_start_matches('\\').rsplit_once('\\')?;
    let name = if file.to_ascii_lowercase().contains("reversed") {
        "stats_reversed.xml"
    } else {
        "stats.xml"
    };
    let entry = format!(r"{directory}\{name}");
    let blob = if in_fedata {
        let spec = oag_assets::source::archive_spec(source, oag_pulse::archives::FEDATA);
        oag_assets::Archive::open(&spec).and_then(|mut archive| archive.read_name(&entry))
    } else {
        archives.read_name(&entry)
    }
    .map_err(|e| report.push(format!("track stats: {entry} did not read ({e})")))
    .ok()?;
    match track_stats::from_blob(&blob) {
        Ok(stats) => {
            report.push(format!(
                "track stats: {entry}, race times {:?} s, lap times {:?} s",
                stats.race_times, stats.lap_times
            ));
            Some(stats)
        }
        Err(e) => {
            report.push(format!("track stats: {entry} did not parse ({e})"));
            None
        }
    }
}
