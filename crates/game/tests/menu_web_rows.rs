//! The rows the web build drops from the options pages, the one it adds, and
//! the word it shows for fullscreen. Its own file: the menu definition tests
//! sit at the file-length ceiling.

use oag_ui::menu::*;

fn built_in() -> Definition {
    Definition::parse(BUILT_IN, &oag_ui::language::StringTable::default())
        .expect("the built-in menu must parse")
}

/// The web build drops the rows a page cannot have, and a desktop drops the
/// one only a page has; each side still keeps the row that stands in for it.
#[test]
fn the_web_and_the_desktop_each_drop_the_rows_they_cannot_have() {
    use oag_game::settings::web::{HIDDEN_ROWS, WEB_ONLY_ROWS};
    let has = |definition: &Definition, setting: &str| {
        definition.pages.iter().any(|page| {
            page.entries
                .iter()
                .any(|entry| entry.setting() == Some(setting))
        })
    };
    let full = built_in();
    for row in HIDDEN_ROWS.iter().chain(&WEB_ONLY_ROWS) {
        assert!(has(&full, row), "the built-in menu offers {row}");
    }
    let mut web = built_in();
    web.drop_settings(&HIDDEN_ROWS);
    web.drop_settings(&["display.window_size"]);
    assert!(HIDDEN_ROWS.iter().all(|row| !has(&web, row)));
    assert!(!has(&web, "display.window_size"));
    assert!(has(&web, "display.canvas_size") && has(&web, "display.window_mode"));
    web.relabel_choice("display.window_mode", "borderless", "FULLSCREEN");
    let labels: Vec<String> = web
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .filter(|entry| entry.setting() == Some("display.window_mode"))
        .filter_map(|entry| match entry {
            Entry::Choice { values, .. } => Some(values.iter().map(|c| c.label.clone())),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(labels.iter().any(|l| l == "FULLSCREEN"), "{labels:?}");
    let mut desktop = built_in();
    desktop.drop_settings(&WEB_ONLY_ROWS);
    assert!(!has(&desktop, "display.canvas_size"));
    assert!(has(&desktop, "display.window_size") && has(&desktop, "display.vsync"));
}
