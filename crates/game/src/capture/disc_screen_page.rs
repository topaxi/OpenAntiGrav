//! `--menu-page pulse-main|pulse-racebox|pulse-single|pulse-options`: the disc's own list
//! screens as a page of the menu model, so a comparison against the original's frame draws
//! the same rows, title and help lines the original does.
//!
//! The live pages stay `menu.toml`'s (ADR-0066: this build's rows are the vanilla ones plus
//! its own). These four pages are compare-only: they are read off the screen's own XML
//! (`MainMenu_Definition.xml`, `RaceBox_Definition.xml`, `Additional_Definition.xml`), every row
//! points at its own page because a still never activates one, and they draw through the
//! same `oag_ui::menu::draw_list` the live pages do. `docs/tools/compare-page.md`, "Pulse
//! front-end list screens".

use anyhow::{Context, Result};
use oag_ui::language::StringTable;
use oag_ui::menu::{Choice, Condition, Entry, Page, Value};
use oag_ui::screen::{Screen, Screens};

/// One of the disc's list screens.
pub(super) struct Spec {
    /// The `--menu-page` name.
    pub page: &'static str,
    /// The archive entry that authors it.
    definition: &'static str,
    /// The screen's own `name` attribute.
    screen: &'static str,
    /// The header's string id where the screen authors no title `Text`: the Racebox screens
    /// are headed by their `AreaController`'s area, whose header text the executable draws.
    /// The frames read `RACEBOX`, the string of the Main Menu's `FE_RACEBOX` row; the area
    /// table itself is not read, so this id is **chosen, not measured**.
    title_id: Option<&'static str>,
    /// Whether the screen has a way back, so the footer legend draws `Back` as it does on a
    /// page opened from another. `Main Menu` has none (`backward="none"`); `Racebox` names
    /// `DefaultPrevious`, and `Additional` and `Single Player` redirect the backward button.
    pub back: bool,
}

const SCREENS: [Spec; 4] = [
    Spec {
        page: "pulse-main",
        definition: oag_pulse::names::MAIN_MENU_DEFINITION,
        screen: "Main Menu",
        title_id: None,
        back: false,
    },
    Spec {
        page: "pulse-racebox",
        definition: oag_pulse::names::RACE_SETUP_DEFINITION,
        screen: "Racebox",
        title_id: Some("FE_RACEBOX"),
        back: true,
    },
    Spec {
        page: "pulse-single",
        definition: oag_pulse::names::RACE_SETUP_DEFINITION,
        screen: "Single Player",
        title_id: Some("FE_RACEBOX"),
        back: true,
    },
    Spec {
        page: "pulse-options",
        definition: oag_pulse::names::ADDITIONAL_DEFINITION,
        screen: "Additional",
        title_id: None,
        back: true,
    },
];

/// Which disc screen a `--menu-page` name asks for, if any.
pub(super) fn spec(page: &str) -> Option<&'static Spec> {
    SCREENS.iter().find(|spec| spec.page == page)
}

/// The screen as a page whose index in the definition will be `own_index`.
///
/// `source` is the disc image the page is read off.
pub(super) fn page(
    spec: &Spec,
    source: &str,
    strings: &StringTable,
    own_index: usize,
) -> Result<Page> {
    let mut archives = oag_pulse::open(source).context("this source is not Pulse's own")?;
    let skin_raw = archives
        .read_name(oag_pulse::names::FRONTEND_ROOT)
        .context("no front-end root on this disc")?;
    let skin = oag_tables::fexml::text(&skin_raw).context("front-end root text")?;
    let globals = Screens::from_xml(&skin).globals;
    let fallback: Vec<(&str, &str)> = globals
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let raw = archives
        .read_name(spec.definition)
        .with_context(|| format!("no {} on this disc", spec.definition))?;
    let xml = oag_tables::fexml::text(&raw).with_context(|| spec.definition.to_string())?;
    let screens = Screens::from_xml_with_fallback_globals(&xml, &fallback);
    let screen = screens
        .by_name(spec.screen)
        .with_context(|| format!("{} authors no {:?} screen", spec.definition, spec.screen))?;
    let text = |id: &str| strings.get_or_id(id).to_string();
    let title = screen
        .texts
        .iter()
        .find(|text| text.font.eq_ignore_ascii_case("title"))
        .and_then(|widget| widget.idstring.as_deref())
        .map(&text)
        .or_else(|| spec.title_id.map(text))
        .unwrap_or_else(|| spec.screen.to_string());
    let (entries, subtitles) = if screen.lists.is_empty() {
        menu_rows(screen, own_index, &text)
    } else {
        list_rows(screen, &text)
    };
    Ok(Page::new(spec.page, title, entries, subtitles))
}

/// A `<Menu>` screen: one row per `Entry`, and the screen's `helptext{n}` widget under row `n`
/// as that row's second line.
fn menu_rows(
    screen: &Screen,
    own_index: usize,
    text: &dyn Fn(&str) -> String,
) -> (Vec<Entry>, Vec<Option<String>>) {
    let menu = screen.menu.as_ref();
    let ids = menu.map_or(&[][..], |menu| menu.entries.as_slice());
    let entries = ids
        .iter()
        .map(|id| Entry::Submenu {
            label: text(id),
            target: own_index,
        })
        .collect();
    let subtitles = (0..ids.len())
        .map(|row| {
            screen
                .texts
                .iter()
                .find(|widget| widget.name.as_deref() == Some(&format!("helptext{row}")))
                .and_then(|widget| widget.idstring.as_deref())
                .map(text)
        })
        .collect();
    (entries, subtitles)
}

/// The race box's settings screen: each `<List>` is a row, labelled by its `<Text name="{name}Text">`
/// and holding the list's own `default`, or its first entry where the list names none.
///
/// A row is greyed while `Mode` is anything but `Elimination`, which is what the original's
/// frame shows for `Eliminations` (KILLS) on a Single Race.
fn list_rows(screen: &Screen, text: &dyn Fn(&str) -> String) -> (Vec<Entry>, Vec<Option<String>>) {
    let modes: Vec<&str> = screen
        .lists
        .iter()
        .find(|list| list.name.as_deref() == Some("Mode"))
        .map_or_else(Vec::new, |list| {
            list.entries.iter().map(String::as_str).collect()
        });
    let mut entries = Vec::new();
    for list in &screen.lists {
        let name = list.name.as_deref().unwrap_or_default();
        let label = screen
            .texts
            .iter()
            .find(|widget| widget.name.as_deref() == Some(&format!("{name}Text")))
            .and_then(|widget| widget.idstring.as_deref())
            .map(text)
            .unwrap_or_else(|| name.to_string());
        let ids: Vec<String> = if list.entries.is_empty() {
            list.literal_entries.clone()
        } else {
            list.entries.clone()
        };
        let values: Vec<Choice> = ids
            .iter()
            .map(|id| Choice::labelled(id.clone(), text(id)))
            .collect();
        let current = list
            .default
            .as_ref()
            .and_then(|default| ids.iter().position(|id| id == default))
            .unwrap_or(0);
        let disabled_by = (name == "Eliminations").then(|| Condition {
            setting: "pulse.mode".to_string(),
            values: modes
                .iter()
                .filter(|mode| **mode != "Elimination")
                .map(|mode| Value::Text((*mode).to_string()))
                .collect(),
        });
        entries.push(Entry::Choice {
            label,
            setting: format!("pulse.{}", name.to_ascii_lowercase()),
            values,
            source: None,
            query: None,
            default: None,
            current,
            disabled_by,
            pins: Vec::new(),
            warnings: Vec::new(),
            restart: None,
        });
    }
    let subtitles = vec![None; entries.len()];
    (entries, subtitles)
}

#[cfg(test)]
mod tests;
