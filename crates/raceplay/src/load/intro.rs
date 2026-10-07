//! The circuit's pre-race flyby: `start_grid.vex`, beside `track.vex`.
//!
//! Which file, and whether the title plays one on this disc at all, is [`oag_title::pre_race`]:
//! Pulse on a PSP disc and Wipeout HD on a PS3 one. Pulse's directory holds one file, so
//! `track_reversed.vex` and `zone_track.vex` share it; HD ships a `_reversed` file beside it.
//! See [`crate::intro_camera`].

use oag_vex::grid_camera::GridCamera;

/// The title's flyby rule when it plays one on this source, else `None`.
pub(super) fn rule(
    title: &oag_title::Title,
    archives: &oag_assets::Archives,
) -> Option<&'static oag_title::pre_race::PreRace> {
    title
        .pre_race
        .filter(|rule| rule.on.applies(archives.layout.platform))
}

/// The flyby for the circuit `track` (a `.vex` entry name) lives in, or `None` where the title
/// plays none or the circuit authors none. Says which in `report`.
pub(super) fn read(
    archives: &mut oag_assets::source::Archives,
    track: &str,
    rule: Option<&'static oag_title::pre_race::PreRace>,
    report: &mut Vec<String>,
) -> Option<(GridCamera, &'static oag_title::pre_race::PreRace)> {
    let rule = rule?;
    let (directory, file) = track.rsplit_once('\\')?;
    let name = match rule.grid_reversed {
        Some(reversed) if file.to_ascii_lowercase().contains("reversed") => reversed,
        _ => rule.grid,
    };
    let entry = format!(r"{directory}\{name}");
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
    grid.map(|grid| (grid, rule))
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
    rule: Option<&oag_title::pre_race::PreRace>,
    preferred_language: Option<&str>,
    report: &mut Vec<String>,
) -> Option<crate::track_panel::Assets> {
    if !rule.is_some_and(|rule| rule.panel) {
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
