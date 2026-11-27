//! The scale each menu page's heading is authored at.
//!
//! A heading is the first `Text` in the title font on the screen a page stands where
//! ([`oag_title::PageHeading`], [`oag_title::ListScreen`]). Its `scale` is what the disc says:
//! `FEGlobals->TitleScale` on most screens, none (`1.0`) on Pulse's `Additional` and `extras`.

use std::collections::HashMap;

use oag_tables::fexml;
use oag_ui::screen::Screens;

/// `(page id, scale)` for every page whose screen reads; a file or screen that does not is
/// reported and its page keeps the skin's own scale.
pub(super) fn read(
    archives: &mut oag_assets::Archives,
    front_end: &oag_title::FrontEnd,
    globals: &HashMap<String, String>,
    report: &mut Vec<String>,
) -> Vec<(&'static str, f32)> {
    let wanted = front_end
        .page_headings
        .iter()
        .map(|heading| (heading.page, heading.file, heading.screen))
        .chain(
            front_end
                .list_screens
                .iter()
                .map(|screen| (screen.page, screen.file, screen.screen)),
        );
    let fallback: Vec<(&str, &str)> = globals
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let mut files: HashMap<&str, Option<Screens>> = HashMap::new();
    let mut scales = Vec::new();
    for (page, file, name) in wanted {
        let screens = files.entry(file).or_insert_with(|| {
            let read = archives
                .read_name(file)
                .map_err(anyhow::Error::from)
                .and_then(|blob| fexml::text(&blob).map_err(|e| anyhow::anyhow!("{e}")));
            match read {
                Ok(xml) => Some(Screens::from_xml_with_fallback_globals(&xml, &fallback)),
                Err(error) => {
                    report.push(format!(
                        "{file}: {error:#}; its headings keep the skin's scale"
                    ));
                    None
                }
            }
        });
        let Some(screens) = screens else {
            continue;
        };
        let Some(screen) = screens.by_name(name) else {
            report.push(format!(
                "{file}: no {name:?} screen; {page:?} keeps the skin's heading scale"
            ));
            continue;
        };
        // A screen with no heading widget of its own (Pulse's two Racebox screens, whose area
        // header the executable draws) keeps the skin's scale without a report.
        if let Some(heading) = screen
            .texts
            .iter()
            .find(|text| text.font.eq_ignore_ascii_case("title"))
        {
            scales.push((page, heading.scale));
        }
    }
    scales
}
