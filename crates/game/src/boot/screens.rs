//! The front-end XML loaders: the skin, one of its includes, and the
//! selection screens' layouts read off that include.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use anyhow::{Context, Result};
use oag_tables::fexml;
use oag_ui::screen::Screens;

/// One of the skin's `LoadXML` includes, parsed on its own with the skin's
/// globals as fallbacks - which is how an include resolves `FEGlobals->`
/// names it never declares.
pub(super) fn load_included_screens(
    archives: &mut oag_assets::Archives,
    name: &str,
    skin: &Screens,
) -> Result<Screens> {
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
    Ok(Screens::from_xml_with_fallback_globals(&xml, &globals))
}

pub(super) fn load_screens(
    archives: &mut oag_assets::Archives,
    root: &str,
    fallback_globals: &[(&str, &str)],
    fallback_images: &[(&str, &str)],
    report: &mut Vec<String>,
) -> Result<Screens> {
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
                movie.entry_name()
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
/// is not. See [`oag_ui::picker::FaceScales`].
pub(super) fn selection_layouts(
    race_box: Option<&Screens>,
    strings: &oag_ui::language::StringTable,
    font: &oag_ui::font::Atlas,
    menu_font: Option<&oag_ui::font::Atlas>,
    space: oag_display::space::Space,
    report: &mut Vec<String>,
) -> (
    Option<oag_ui::picker::Layout>,
    Option<oag_ui::picker::Layout>,
) {
    let faces = oag_ui::picker::FaceScales {
        default: menu_font.map_or(oag_ui::picker::FaceScales::default().default, |menu| {
            font.line_height / menu.line_height
        }),
        ..oag_ui::picker::FaceScales::default()
    };
    let track_select = race_box.and_then(|included| {
        oag_ui::picker::Layout::read(
            included,
            oag_ui::picker::Kind::Track,
            strings,
            faces,
            [space.size.0, space.size.1],
        )
    });
    let ship_select = race_box.and_then(|included| {
        oag_ui::picker::Layout::read(
            included,
            oag_ui::picker::Kind::Ship,
            strings,
            faces,
            [space.size.0, space.size.1],
        )
    });
    report.push(format!(
        "selection screens: track {}, ship {}",
        track_select.as_ref().map_or("unread", |_| "read"),
        ship_select.as_ref().map_or("unread", |_| "read")
    ));
    (track_select, ship_select)
}
