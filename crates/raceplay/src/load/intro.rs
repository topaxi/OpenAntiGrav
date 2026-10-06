//! The circuit's pre-race flyby: `start_grid.vex`, beside `track.vex`.
//!
//! One file per circuit directory - `track_reversed.vex` and `zone_track.vex` share it, since
//! the directory holds no other spelling. Pulse on a PSP disc only: the camera was read and
//! measured there and nowhere else. See [`crate::intro_camera`].

use oag_vex::grid_camera::GridCamera;

/// The flyby for the circuit `track` (a `.vex` entry name) lives in, or `None` off Pulse's PSP
/// disc or when the circuit authors none. Says which in `report`.
pub(super) fn read(
    archives: &mut oag_assets::source::Archives,
    track: &str,
    pulse_psp: bool,
    report: &mut Vec<String>,
) -> Option<GridCamera> {
    if !pulse_psp {
        return None;
    }
    let (directory, _) = track.rsplit_once('\\')?;
    let entry = format!(r"{directory}\start_grid.vex");
    let blob = match archives.read_name(&entry) {
        Ok(blob) => blob,
        Err(e) => {
            report.push(format!("pre-race flyby: {entry} did not read ({e})"));
            return None;
        }
    };
    let grid = GridCamera::read(&blob);
    report.push(match &grid {
        Some(grid) => format!(
            "pre-race flyby: {entry}, {:.2} s, {} cut(s) at key frames {:?}",
            grid.length(),
            grid.cut_frames().len(),
            grid.cut_frames()
        ),
        None => format!("pre-race flyby: {entry} has no playable gridCamera"),
    });
    grid
}

/// The `PI_Track` id of the circuit `track` (a `.vex` entry name) is the race on: the entry in
/// the plugin definition with the same environment directory and the same `Reversed`.
///
/// The race loader is handed a file, not an id, and the panel names the id: `16_Track` and
/// `32_Track` fly one directory's `track.vex` and `track_reversed.vex`. Zone's files are the same
/// pair with a prefix, so they resolve to the same entries. `None` when no entry matches.
fn track_id(
    archives: &mut oag_assets::source::Archives,
    title: &'static oag_title::Title,
    track: &str,
) -> Option<String> {
    let (directory, file) = track.rsplit_once('\\')?;
    let reversed = file.contains("reversed");
    let xml = oag_tables::fexml::text(&archives.read_name(title.plugin_definition).ok()?).ok()?;
    let mut ids = crate::catalogue::tracks(&xml)
        .into_iter()
        .filter(|t| t.location == directory && t.reversed == reversed);
    ids.next().map(|t| t.id)
}

/// The track-description panel for the circuit `track`, or `None` off Pulse's PSP disc and when
/// anything it needs will not read. Says which in `report`.
pub(super) fn read_panel(
    archives: &mut oag_assets::source::Archives,
    title: &'static oag_title::Title,
    track: &str,
    pulse_psp: bool,
    preferred_language: Option<&str>,
    report: &mut Vec<String>,
) -> Option<crate::track_panel::Assets> {
    if !pulse_psp {
        return None;
    }
    let Some(id) = track_id(archives, title, track) else {
        report.push(format!(
            "track panel: no PI_Track in {} matches {track}; no panel",
            title.plugin_definition
        ));
        return None;
    };
    let plugins = title
        .front_end
        .map_or::<&[&str], _>(&[], |f| f.language_plugins);
    let languages = oag_ui::language::load::load_languages(
        archives,
        plugins,
        title.front_end.and_then(|f| f.disc_strings),
        report,
    );
    let chosen = oag_ui::language::load::chosen_language(&languages, preferred_language);
    let strings =
        oag_ui::language::load::load_strings(archives, &languages, preferred_language, report);
    let menu_role = title.front_end?.menu?.menu_font?;
    let menu = crate::hud::hud_font(archives, &languages, chosen, menu_role, report);
    let default = crate::hud::hud_font(
        archives,
        &languages,
        chosen,
        oag_ui::language::roles::DEFAULT,
        report,
    );
    crate::track_panel::read(archives, &id, &strings, (menu, default), report)
}
