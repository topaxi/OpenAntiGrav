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
/// plays none or the circuit authors none. The title may fly another file for the race's mode.
/// Says which in `report`.
pub(super) fn read(
    archives: &mut oag_assets::source::Archives,
    track: &str,
    options: &crate::Options,
    rule: Option<&'static oag_title::pre_race::PreRace>,
    report: &mut Vec<String>,
) -> Option<(GridCamera, &'static oag_title::pre_race::PreRace)> {
    let rule = rule?;
    let (directory, file) = track.rsplit_once('\\')?;
    let reversed = file.to_ascii_lowercase().contains("reversed");
    let mut found = None;
    for name in rule.grid_files(options.mode.name(), reversed) {
        let entry = format!(r"{directory}\{name}");
        match archives.read_name(&entry) {
            Ok(blob) => {
                found = Some((entry, blob));
                break;
            }
            Err(e) => report.push(format!("pre-race flyby: {entry} did not read ({e})")),
        }
    }
    let (entry, blob) = found?;
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

/// The entry of the circuit `track`'s flyby voice-over, or `None` where the title plays none or
/// the disc holds no such entry. Says which in `report`.
pub(super) fn read_voice(
    archives: &mut oag_assets::source::Archives,
    title: &'static oag_title::Title,
    track: &str,
    rule: Option<&oag_title::pre_race::PreRace>,
    report: &mut Vec<String>,
) -> Option<String> {
    let rule = rule.filter(|rule| rule.voice_over.is_some())?;
    let Some(id) = track_id(archives, title, track) else {
        report.push(format!(
            "pre-race voice-over: no PI_Track in {} matches {track}; silent",
            title.plugin_definition
        ));
        return None;
    };
    let entry = rule.voice_over_entry(&id)?;
    if archives.locate(&entry).is_none() {
        report.push(format!(
            "pre-race voice-over: {entry} is not on the disc; silent"
        ));
        return None;
    }
    report.push(format!("pre-race voice-over: {entry}"));
    Some(entry)
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
    let (languages, strings) = language(archives, title, preferred_language, report);
    let chosen = oag_ui::language::load::chosen_language(&languages, preferred_language);
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

/// The title's languages and the chosen one's string table, for a screen the race draws.
pub(super) fn language(
    archives: &mut oag_assets::source::Archives,
    title: &'static oag_title::Title,
    preferred_language: Option<&str>,
    report: &mut Vec<String>,
) -> (
    Vec<oag_ui::language::Language>,
    oag_ui::language::StringTable,
) {
    let plugins = title
        .front_end
        .map_or::<&[&str], _>(&[], |f| f.language_plugins);
    let languages = oag_ui::language::load::load_languages(
        archives,
        plugins,
        title.front_end.and_then(|f| f.disc_strings),
        report,
    );
    let strings =
        oag_ui::language::load::load_strings(archives, &languages, preferred_language, report);
    (languages, strings)
}

/// The screen an attract demo draws over its race, when this is a demo and the title authors
/// one. Says which part will not read in `report`.
pub(super) fn read_demo_screen(
    archives: &mut oag_assets::source::Archives,
    title: &'static oag_title::Title,
    options: &crate::Options,
    hud: &oag_hud::Assets,
    report: &mut Vec<String>,
) -> Option<crate::demo_overlay::Assets> {
    if !options.demo {
        return None;
    }
    let screen = title.demo.screen?.value;
    let (languages, strings) = language(archives, title, options.language.as_deref(), report);
    let chosen = oag_ui::language::load::chosen_language(&languages, options.language.as_deref());
    // `font="scroll"`, `Demo_Definition.xml`'s text face, is the language's `Stats` slot.
    let font = crate::hud::hud_font(
        archives,
        &languages,
        chosen,
        oag_ui::language::roles::STATS,
        report,
    );
    crate::demo_overlay::read(archives, &screen, strings, font, hud.space, report)
}
