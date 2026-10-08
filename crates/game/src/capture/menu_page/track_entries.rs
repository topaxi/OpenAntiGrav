//! The track picker's entries for a `--menu-page track-select` capture: the
//! same list, order and labels the live screen opens with
//! (`session::picker::open_track_picker`).

use oag_ui_screens::picker::{Details, Entry};

/// The entries, each one's preview mesh name, and the grid's column count
/// (`0` off HD's own track screen).
pub(super) fn track_entries(
    title: &'static oag_title::Title,
    settings: &crate::settings::Settings,
    circuit_names: &oag_ui::language::CircuitNames,
    strings: &oag_ui::language::StringTable,
    tracks: &[oag_raceplay::catalogue::Track],
    layout: &oag_ui_screens::picker::Layout,
    distance: Option<f32>,
) -> (Vec<Entry>, Vec<String>, usize) {
    let labelled: Vec<(oag_raceplay::catalogue::Track, String)> = tracks
        .iter()
        .map(|track| {
            (
                track.clone(),
                oag_raceplay::catalogue::label(track, circuit_names, strings, tracks),
            )
        })
        .collect();
    let mode = oag_race::Mode::from_name(&settings.race.mode).unwrap_or_default();
    // HD's grid: the same order and columns the live screen uses.
    let (labelled, columns) = if layout.hd_track.is_some() {
        oag_raceplay::catalogue::direction_rows(title, mode, &labelled)
    } else {
        (labelled, 0)
    };
    let laps = oag_raceplay::catalogue::race_laps(mode, settings.race.class.trim());
    let (entries, previews): (Vec<Entry>, Vec<String>) = labelled
        .iter()
        .map(|(track, label)| {
            // A capture keeps no records store; the distance is the
            // caller's, for the selected circuit only.
            let measured = distance.filter(|_| track.id == settings.race.track);
            let info = match (measured, layout.hd_track.is_some()) {
                (Some(d), true) => {
                    let [length, race] = oag_ui_screens::picker::hd::track::length_rows(d, laps);
                    [length, race, "-".into()]
                }
                (Some(d), false) => [format!("{d:.0}"), "-".into(), "-".into()],
                (None, _) => ["-".into(), "-".into(), "-".into()],
            };
            (
                Entry {
                    id: track.id.clone(),
                    label: label.clone(),
                    details: Details::Track {
                        info,
                        emblem: oag_raceplay::catalogue::track_emblem(title, track),
                        icon: oag_raceplay::catalogue::track_icon(title, track),
                        reversed: track.reversed,
                    },
                },
                format!(
                    r"{}\FE\{}.vex",
                    track.location,
                    if track.reversed { "reverse" } else { "forward" }
                ),
            )
        })
        .unzip();
    (entries, previews, columns)
}
