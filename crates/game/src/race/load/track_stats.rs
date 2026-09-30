//! The circuit's own `stats.xml`: the authored `<RaceTimes>` and `<LapTimes>`
//! the HUD's `RECORD` readout counts down to.
//!
//! Pulse keeps it in `FEData.wad`, beside the per-track front-end screens,
//! **not** in the `Data.wad` the race reads its geometry from -
//! `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s `TrackStats_Load`
//! opens `"<track directory>\stats.xml"` (`stats_reversed.xml` for a reversed
//! circuit). The campaign's AI-skill lookup reads the same file the same way
//! (`Session::resolve_campaign_ai_skill_scale`).

use oag_tables::track_stats::{self, TrackStats};

/// The stats for `track` (a `.vex` entry name), or `None` off Pulse's PSP disc
/// or when the file is absent or unreadable. Says which in `report`.
pub(super) fn read(
    source: &str,
    track: &str,
    pulse_psp: bool,
    report: &mut Vec<String>,
) -> Option<TrackStats> {
    if !pulse_psp {
        report.push(
            "track stats: not read (only Pulse on a PSP disc keeps them in FEData.wad); the \
             HUD's RECORD readout has no authored time"
                .into(),
        );
        return None;
    }
    let (directory, file) = track.rsplit_once('\\')?;
    let name = if file.to_ascii_lowercase().contains("reversed") {
        "stats_reversed.xml"
    } else {
        "stats.xml"
    };
    let entry = format!(r"{directory}\{name}");
    let spec = format!("{source}:{}", oag_pulse::archives::FEDATA);
    let blob = oag_assets::Archive::open(&spec)
        .and_then(|mut archive| archive.read_name(&entry))
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
