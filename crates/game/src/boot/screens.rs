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
/// `oag_ui::campaign::footer::NavigationLegend::read` reads.
///
/// Split out under the 1,000-line rule alongside every other loader in this
/// file, not because this one function is large - it is not - but because
/// `super::load_shell` had no room left for it inline.
#[must_use]
pub(super) fn read_nav_legend(
    skin_xml: Option<&str>,
    globals: &std::collections::HashMap<String, String>,
    strings: &StringTable,
) -> Option<oag_ui::campaign::footer::NavigationLegend> {
    let xml = skin_xml?;
    oag_ui::campaign::footer::NavigationLegend::read(&oag_ui::screen::parse(xml), globals, strings)
}

/// The front-end root's own scrolling tip ticker layout, off the identical
/// `skin_xml`/`globals` pair [`read_nav_legend`] reads - mirrors that
/// function for [`oag_ui::campaign::footer::TickerLayout`], the ordinary
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
) -> Option<oag_ui::campaign::footer::TickerLayout> {
    let xml = skin_xml?;
    oag_ui::campaign::footer::TickerLayout::read(&oag_ui::screen::parse(xml), globals)
}

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
