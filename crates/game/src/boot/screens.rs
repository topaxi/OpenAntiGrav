//! The front-end XML loaders: the skin, one of its includes, and the
//! selection screens' layouts read off that include.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use anyhow::{Context, Result};
use oag_tables::fexml;
use oag_ui::language::StringTable;
use oag_ui::screen::Screens;

/// The front-end root's own `Confirm`/`Back` legend, off `skin_xml` -
/// already-decoded text, the same blob [`super::fury::load`] reads a few
/// lines up in `super::load_shell`, re-parsed here rather than re-fetched
/// from the archive a second time. `globals` is that same root's own
/// `Screens::from_xml`-resolved table (`screens.globals` in the caller),
/// needed to follow a `FEGlobals->` indirection a prompt's own `x`/`y`/
/// `color` may carry. `None` when `skin_xml` is `None` (the root did not
/// decode at all, already reported by the caller) or the file authors no
/// `NavigationController` with either half
/// `oag_ui_screens::campaign::footer::NavigationLegend::read` reads.
///
/// Split out under the 1,000-line rule alongside every other loader in this
/// file, not because this one function is large - it is not - but because
/// `super::load_shell` had no room left for it inline.
#[must_use]
pub(super) fn read_nav_legend(
    skin_xml: Option<&str>,
    globals: &std::collections::HashMap<String, String>,
    strings: &StringTable,
) -> Option<oag_ui_screens::campaign::footer::NavigationLegend> {
    let xml = skin_xml?;
    oag_ui_screens::campaign::footer::NavigationLegend::read(
        &oag_ui::screen::parse(xml),
        globals,
        strings,
    )
}

/// The front-end root's own scrolling tip ticker layout, off the identical
/// `skin_xml`/`globals` pair [`read_nav_legend`] reads - mirrors that
/// function for [`oag_ui_screens::campaign::footer::TickerLayout`], the ordinary
/// menu pages' own footer read rather than `Cell Selection`'s own copy
/// (`oag_game::campaign::read_footer`, which reads the same root a second
/// time for the campaign screens - see that function's own doc for why it
/// cannot share this one's parse). `None` when `skin_xml` is `None`, or the
/// root authors no `TextInfoIsAlwaysLast` viewport - every title but Pulse
/// today.
#[must_use]
pub(super) fn read_ticker(
    skin_xml: Option<&str>,
    globals: &std::collections::HashMap<String, String>,
) -> Option<oag_ui_screens::campaign::footer::TickerLayout> {
    let xml = skin_xml?;
    oag_ui_screens::campaign::footer::TickerLayout::read(&oag_ui::screen::parse(xml), globals)
}

/// One of the skin's `LoadXML` includes, parsed on its own with the skin's
/// globals as fallbacks - which is how an include resolves `FEGlobals->`
/// names it never declares.
pub(super) fn load_included_screens(
    archives: &mut oag_assets::Archives,
    name: &str,
    skin: &Screens,
) -> Result<Screens> {
    load_included(archives, name, skin).map(|(screens, _)| screens)
}

/// A title's standalone `Team Selection` or `Track Creation` definition
/// ([`oag_title::FrontEnd::team_select`], [`oag_title::FrontEnd::track_select`]), parsed with the skin's globals
/// and kept beside its own text - `oag_ui_screens::picker::hd::read` walks the tree
/// again for the widgets [`Screens`] does not collect - and the strings its
/// `idstring`s resolve to in `entries`' own `DATA06` copy, the archive the
/// file itself is on (see `crate::campaign::hd_data06_strings`).
pub(super) struct TeamBox {
    pub(super) screens: Screens,
    pub(super) xml: String,
    pub(super) strings: std::collections::HashMap<String, String>,
}

/// Reads [`TeamBox`]. `None` on a title naming no file, and on one whose
/// file will not read, which is reported.
pub(super) fn load_team_select(
    archives: &mut oag_assets::Archives,
    name: Option<&str>,
    entries: Option<&str>,
    skin: &Screens,
    report: &mut Vec<String>,
) -> Option<TeamBox> {
    load_hd_selection(archives, name, entries, skin, &[], "ship", report)
}

/// Reads [`TeamBox`] for the track screen. `extra_ids` are strings the
/// screen's code looks up that no `idstring` in the file names - the RECORDS
/// table's row labels.
pub(super) fn load_track_select(
    archives: &mut oag_assets::Archives,
    name: Option<&str>,
    entries: Option<&str>,
    skin: &Screens,
    report: &mut Vec<String>,
) -> Option<TeamBox> {
    load_hd_selection(
        archives,
        name,
        entries,
        skin,
        &oag_ui_screens::picker::hd::track::RECORD_ROW_IDS,
        "track",
        report,
    )
}

fn load_hd_selection(
    archives: &mut oag_assets::Archives,
    name: Option<&str>,
    entries: Option<&str>,
    skin: &Screens,
    extra_ids: &[&str],
    which: &str,
    report: &mut Vec<String>,
) -> Option<TeamBox> {
    let name = name?;
    let (screens, xml) = load_included(archives, name, skin)
        .inspect_err(|error| report.push(format!("{name}: {error:#} - no {which} screen")))
        .ok()?;
    let ids: Vec<&str> = xml
        .split("idstring=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .chain(extra_ids.iter().copied())
        .collect();
    let strings = entries
        .map(|path| crate::campaign::hd_data06_strings(archives, path, &ids))
        .unwrap_or_default();
    Some(TeamBox {
        screens,
        xml,
        strings,
    })
}

/// Every team's own `FE\Logo.gtf`, for a source whose ship screen draws
/// one - asked for by name alongside what the screens name, since the
/// `Logo` widget authors no `src`. See `oag_ui_screens::picker::hd::logo_src`.
pub(super) fn team_logos(team_box: bool, teams: &[oag_raceplay::catalogue::Team]) -> Vec<String> {
    if !team_box {
        return Vec::new();
    }
    teams
        .iter()
        .map(|team| oag_ui_screens::picker::hd::logo_src(&team.id))
        .collect()
}

/// Every circuit's own emblem, for a source whose track screen draws one -
/// asked for by name, since the `Emblem` widget authors no `src`. See
/// `oag_ui_screens::picker::hd::track::emblem_src`.
pub(super) fn track_emblems(
    track_box: bool,
    tracks: &[oag_raceplay::catalogue::Track],
) -> Vec<String> {
    if !track_box {
        return Vec::new();
    }
    let mut out: Vec<String> = tracks
        .iter()
        .map(|track| oag_ui_screens::picker::hd::track::emblem_src(&track.location))
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

fn load_included(
    archives: &mut oag_assets::Archives,
    name: &str,
    skin: &Screens,
) -> Result<(Screens, String)> {
    let blob = archives
        .read_name(name)
        .with_context(|| format!("reading {name} out of {}", archives.layout.describe()))?;
    let xml = if fexml::is_fexml(&blob) {
        fexml::expand(&blob).map_err(|e| anyhow::anyhow!("expanding the front-end XML: {e}"))?
    } else {
        String::from_utf8(blob).context("the front-end XML is not text")?
    };
    let globals: Vec<(&str, &str)> = skin
        .globals
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    Ok((Screens::from_xml_with_fallback_globals(&xml, &globals), xml))
}

/// The globals every *activated* style skin declares, sorted by name.
///
/// **Pure's front end is skinnable and Pulse's is not**, and that is why its
/// own `Data\Plugins\PI001\GUI\Skin.xml` declares 13 globals against Pulse's
/// 56: the other 37 - every colour the selection screens, the stat panels and
/// the title text draw in - live in `Data\Skins\Default\Skin.xml`, a second
/// skin the plugin definition activates alongside the UI one:
///
/// ```xml
/// <PI_Skin name="UI">      <Values location="Data\Plugins\PI001\GUI" activate="true"/>
/// <PI_Skin name="Default"> <Values location="Data\Skins\Default" style="true" activate="true"/>
/// <PI_Skin name="Hacker">  <Values location="Data\Skins\Hacker" style="true"/>
/// ```
///
/// `Hacker` is the same file with a black background and is **not**
/// activated, so it is not read - which is the whole reason to follow
/// `activate` rather than load every `PI_Skin` on the disc.
///
/// Pulse declares exactly one `PI_Skin`, the UI one, whose `Skin.xml` is the
/// front-end root already loaded - so this returns nothing there and changes
/// nothing, measured on both pressings rather than assumed.
///
/// A skin that will not read is reported and skipped: a missing colour falls
/// back the way it always did, which is a duller screen and not a dead one.
fn style_skin_globals(
    archives: &mut oag_assets::Archives,
    plugin_definition: &str,
    front_end_root: &str,
    report: &mut Vec<String>,
) -> Vec<(String, String)> {
    let Ok(blob) = archives.read_name(plugin_definition) else {
        return Vec::new();
    };
    let Ok(xml) = expand_front_end_xml(&blob) else {
        return Vec::new();
    };
    let mut skins = Vec::new();
    collect_skins(&fexml::parse(&xml), &mut skins);
    let mut out: Vec<(String, String)> = Vec::new();
    for location in skins {
        let name = format!(r"{location}\Skin.xml");
        // The UI skin is the front-end root, already loaded with its own
        // globals; reading it again here would only shadow itself.
        if name.eq_ignore_ascii_case(front_end_root) {
            continue;
        }
        match archives
            .read_name(&name)
            .map_err(|e| e.to_string())
            .and_then(|blob| expand_front_end_xml(&blob).map_err(|e| e.to_string()))
        {
            Ok(xml) => {
                let globals = Screens::from_xml(&xml).globals;
                report.push(format!("style skin {name}: {} globals", globals.len()));
                out.extend(globals);
            }
            Err(error) => report.push(format!("{name}: {error} - its globals are unread")),
        }
    }
    out.sort();
    out
}

/// Every activated `PI_Skin`'s `location`, in document order.
fn collect_skins(node: &fexml::Node, out: &mut Vec<String>) {
    for skin in node.children_named("PI_Skin") {
        if skin.flag("activate") == Some(true)
            && let Some(location) = skin.value("location")
        {
            out.push(location.to_string());
        }
    }
    for child in &node.children {
        collect_skins(child, out);
    }
}

/// Front-end XML is stored with its element and attribute names shortened
/// through a per-file dictionary; files that begin `<?xml` are already plain.
fn expand_front_end_xml(blob: &[u8]) -> Result<String> {
    if fexml::is_fexml(blob) {
        fexml::expand(blob).map_err(|e| anyhow::anyhow!("expanding the front-end XML: {e}"))
    } else {
        String::from_utf8(blob.to_vec()).context("the front-end XML is not text")
    }
}

/// Reads the front-end root, with the activated style skins' globals seeded
/// under the title's own measured stand-ins.
///
/// **The style skins first**, because on a skinnable front end most of the
/// colours are the style's - see [`style_skin_globals`]. Both go in as
/// *fallbacks*, so the root's own declarations win over either.
///
/// Which order they are in turns out not to matter on the one title that has
/// a style skin: Pure's root declares 9 globals, its style skin 41, and the
/// union is 50 - **the two files name no global in common**, measured rather
/// than assumed. The precedence is stated anyway so that a title that does
/// overlap resolves predictably rather than by whichever file was read last.
pub(super) fn load_screens(
    archives: &mut oag_assets::Archives,
    root: &str,
    plugin_definition: &str,
    fallback_globals: &[(&str, &str)],
    fallback_images: &[(&str, &str)],
    movie_region: &str,
    report: &mut Vec<String>,
) -> Result<Screens> {
    let style = style_skin_globals(archives, plugin_definition, root, report);
    let fallback_globals: Vec<(&str, &str)> = style
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .chain(fallback_globals.iter().copied())
        .collect();
    let fallback_globals = fallback_globals.as_slice();
    // The one piece with no degraded form: a front end with no screens is not a
    // front end. The message names the archives searched, because on a source
    // whose front-end root has never been located that is the useful half.
    let blob = archives
        .read_name(root)
        .with_context(|| format!("reading {} out of {}", root, archives.layout.describe()))?;

    // Front-end XML is stored with its element and attribute names shortened
    // through a per-file dictionary. Files that begin `<?xml` are already plain.
    let xml = if fexml::is_fexml(&blob) {
        fexml::expand(&blob).map_err(|e| anyhow::anyhow!("expanding the front-end XML: {e}"))?
    } else {
        String::from_utf8(blob).context("the front-end XML is not text")?
    };

    // This title's own measured stand-ins, not every title's. Pure's table used
    // to be handed to every source on the grounds that `or_insert` made it a
    // no-op on Pulse - true, and true only for as long as no two titles measure a
    // *different* value for one name. Pulse's own table is empty.
    let screens = Screens::from_xml_with_fallbacks(&xml, fallback_globals, fallback_images);
    report.push(format!(
        "{}: {} screens, {} globals, {} LoadXML includes",
        root,
        screens.screens.len(),
        screens.globals.len(),
        screens.load_xml.len()
    ));
    for screen in screens.with_movies() {
        // One line per widget. A screen with four of them used to report one,
        // whichever the parser happened to keep last, which made the other three
        // look absent from the disc.
        for movie in &screen.movies {
            report.push(format!(
                "  screen {:?} plays {}",
                screen.name,
                movie.entry_name(movie_region)
            ));
        }
    }
    Ok(screens)
}

/// The race box's two layouts off its parsed definition, or neither when
/// the title names none.
///
/// The face ratios are measured off the faces actually loaded where both
/// are - `default` against `menu` - and Pulse's own `small` where the third
/// is not. See [`oag_ui_screens::picker::FaceScales`].
#[allow(
    clippy::too_many_arguments,
    reason = "each is a separate fact of the two screens: the three files, the strings and faces, the grid"
)]
pub(super) fn selection_layouts(
    race_box: Option<&Screens>,
    team_box: Option<&TeamBox>,
    track_box: Option<&TeamBox>,
    strings: &oag_ui::language::StringTable,
    font: &oag_ui::font::Atlas,
    menu_font: Option<&oag_ui::font::Atlas>,
    space: oag_display::space::Space,
    report: &mut Vec<String>,
) -> (
    Option<oag_ui_screens::picker::Layout>,
    Option<oag_ui_screens::picker::Layout>,
) {
    let faces = oag_ui_screens::picker::FaceScales {
        default: menu_font.map_or(
            oag_ui_screens::picker::FaceScales::default().default,
            |menu| font.line_height / menu.line_height,
        ),
        ..oag_ui_screens::picker::FaceScales::default()
    };
    let track_select = race_box
        .and_then(|included| {
            oag_ui_screens::picker::Layout::read(
                included,
                oag_ui_screens::picker::Kind::Track,
                strings,
                faces,
                [space.size.0, space.size.1],
            )
        })
        .or_else(|| {
            let track_box = track_box?;
            let strings = with_served_gaps(strings, track_box);
            let grid = [space.size.0, space.size.1];
            oag_ui_screens::picker::hd::track::read(
                &track_box.xml,
                &track_box.screens,
                &strings,
                faces,
                grid,
            )
        });
    let ship_select = race_box
        .and_then(|included| {
            oag_ui_screens::picker::Layout::read(
                included,
                oag_ui_screens::picker::Kind::Ship,
                strings,
                faces,
                [space.size.0, space.size.1],
            )
        })
        .or_else(|| {
            let team_box = team_box?;
            let strings = with_served_gaps(strings, team_box);
            let grid = [space.size.0, space.size.1];
            oag_ui_screens::picker::hd::read(
                &team_box.xml,
                &team_box.screens,
                &strings,
                faces,
                grid,
            )
        });
    report.push(format!(
        "selection screens: track {}, ship {}",
        track_select.as_ref().map_or("unread", |_| "read"),
        ship_select.as_ref().map_or("unread", |_| "read")
    ));
    (track_select, ship_select)
}

/// `strings` with what the served table lacks filled in from `file`'s own
/// `DATA06` copy. An id both copies carry keeps the served copy's text, as
/// every other screen does.
fn with_served_gaps(
    strings: &oag_ui::language::StringTable,
    file: &TeamBox,
) -> oag_ui::language::StringTable {
    let mut strings = strings.clone();
    strings.merge(
        file.strings
            .iter()
            .filter(|(id, _)| strings.get(id).is_none())
            .map(|(id, text)| (id.clone(), text.clone()))
            .collect(),
    );
    strings
}

/// What the race box's `Single Player` screen authors for the rows this build
/// reads: the `KILLS` row's targets and the `WEAPONS` row's two states, each in
/// the order the disc's own list has them. Empty when the title names no such
/// file ([`oag_title::FrontEnd::race_setup`]) or it will not read (reported),
/// which leaves the row unusable rather than offering values nothing authored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RaceSetup {
    /// The `Eliminations` list (`5`, `10`, `15`, `20`, `25` on Pulse).
    pub kill_targets: Vec<String>,
    /// The `Weapons` list as `(string id, value)`: `("FE_ON", "On")` then
    /// `("FE_OFF", "Off")` on Pulse. The value is what the original's
    /// `Race_ReadSetupOptions` compares against `"On"`; the string id is the
    /// row's label in the front end's own string table.
    pub weapons: Vec<(String, String)>,
}

impl RaceSetup {
    /// The `WEAPONS` row's options: stored as the disc's `value` (`On`/`Off`),
    /// shown as its own string for the entry (`FE_ON`/`FE_OFF`, in the
    /// language `strings` is for).
    #[must_use]
    pub fn weapon_choices(
        &self,
        strings: &oag_ui::language::StringTable,
    ) -> Vec<oag_ui::menu::Choice> {
        self.weapons
            .iter()
            .map(|(id, value)| oag_ui::menu::Choice::labelled(value, strings.get_or_id(id)))
            .collect()
    }
}

/// The race box's `Single Player` lists off `name` - see [`RaceSetup`].
pub(super) fn read_race_setup(
    archives: &mut oag_assets::Archives,
    name: Option<&str>,
    report: &mut Vec<String>,
) -> RaceSetup {
    let Some(name) = name else {
        return RaceSetup::default();
    };
    let xml = archives
        .read_name(name)
        .map_err(anyhow::Error::from)
        .and_then(|blob| {
            fexml::text(&blob).map_err(|e| anyhow::anyhow!("reading the front-end XML: {e}"))
        });
    match xml {
        Ok(xml) => RaceSetup {
            kill_targets: list_entries(&xml, "Eliminations")
                .into_iter()
                .map(|(string, _)| string)
                .collect(),
            weapons: list_entries(&xml, "Weapons")
                .into_iter()
                .filter_map(|(string, value)| Some((string, value?)))
                .collect(),
        },
        Err(error) => {
            report.push(format!(
                "{name}: {error:#} - no KILLS or WEAPONS row values"
            ));
            RaceSetup::default()
        }
    }
}

/// The original's settings-screen look, read off the file the skin names -
/// Pulse's `RaceBox_Definition.xml`, which holds `Single Player`. `None` on a title
/// whose skin names no settings screen, and on a file that will not read or
/// whose screen authors no `<List>`, which is reported.
pub(super) fn read_settings_layout(
    archives: &mut oag_assets::Archives,
    skin: &oag_title::MenuSkin,
    globals: &std::collections::HashMap<String, String>,
    sprites: &[(String, oag_ui::frontend::Placed)],
    report: &mut Vec<String>,
) -> Option<oag_ui::menu::SettingsLayout> {
    let spec = skin.settings?;
    let name = spec.file;
    let xml = archives
        .read_name(name)
        .map_err(anyhow::Error::from)
        .and_then(|blob| {
            fexml::text(&blob).map_err(|e| anyhow::anyhow!("reading the front-end XML: {e}"))
        });
    let layout = xml.map(|xml| oag_ui::menu::SettingsLayout::read(&xml, globals, spec, sprites));
    match layout {
        Ok(Some(layout)) => {
            report.push(layout.describe());
            Some(layout)
        }
        Ok(None) => {
            report.push(format!(
                "{name}: no {:?} screen with a <List> to place a settings column by; the pages draw as plain rows",
                spec.screen
            ));
            None
        }
        Err(error) => {
            report.push(format!("{name}: {error:#}; the pages draw as plain rows"));
            None
        }
    }
}

/// The `string` (`idstring` on an entry that names a string-table id) and the
/// `value`, when the entry has one, of every tag in the first list named `list`
/// that carries one, unparsed. The tag's own name is
/// not matched: the front end's shortened XML expands it per file.
fn list_entries(xml: &str, list: &str) -> Vec<(String, Option<String>)> {
    let Some(at) = xml.find(&format!("name=\"{list}\"")) else {
        return Vec::new();
    };
    let body = &xml[at..];
    let body = &body[..body.find("</List>").unwrap_or(body.len())];
    body.split('<')
        .filter_map(|tag| {
            let tag = &tag[..tag.find('>').unwrap_or(tag.len())];
            let attribute = |key: &str| {
                let rest = tag.split(&format!(" {key}=\"")).nth(1)?;
                rest.split('"').next().map(str::to_string)
            };
            Some((
                attribute("string").or_else(|| attribute("idstring"))?,
                attribute("value"),
            ))
        })
        .collect()
}

#[cfg(test)]
mod race_setup_tests {
    use super::list_entries;

    #[test]
    fn a_list_is_read_in_authored_order_and_nothing_else() {
        let xml = r#"<List name="Weapons" global="Weapons"><Entry idstring="FE_ON" value="On"></Entry><Entry idstring="FE_OFF" value="Off"></Entry></List>
<List name="Eliminations" focus="true" global="Eliminations"><Anim x="250"/><Data string="5"/><Data string="10"/><Data string="25"/></List>
<Text string="later"/>"#;
        let kills: Vec<String> = list_entries(xml, "Eliminations")
            .into_iter()
            .map(|(string, _)| string)
            .collect();
        assert_eq!(kills, ["5", "10", "25"]);
        assert_eq!(
            list_entries(xml, "Weapons"),
            [
                ("FE_ON".to_string(), Some("On".to_string())),
                ("FE_OFF".to_string(), Some("Off".to_string())),
            ]
        );
        assert!(list_entries("<List name=\"Mode\"/>", "Eliminations").is_empty());
    }
}
