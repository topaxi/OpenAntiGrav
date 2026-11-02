//! A template row expands to one row per item, in its place, with every parallel list in step.

use super::*;

const TEMPLATE: &str = "audio.music_pool";

fn item(id: &str, label: &str) -> RowItem {
    RowItem {
        id: id.to_string(),
        label: label.to_string(),
        detail: Some(format!("/path/{id}")),
        values: vec![Choice::labelled("on", "ON"), Choice::labelled("off", "OFF")],
        current: "off".to_string(),
    }
}

#[test]
fn a_template_becomes_a_row_per_item_in_its_place() {
    let mut definition = built_in();
    let made = definition.expand_rows(
        ValueSource::MusicPool,
        &[item("a", "ALPHA"), item("b", "BETA")],
    );
    assert_eq!(made, 2);
    let mut menu = Menu::new(definition);
    assert!(menu.open("music"));
    let page = menu.page();
    assert_eq!(page.entries.len(), page.subtitles.len());
    assert_eq!(page.entries.len(), page.meta.len());
    assert_eq!(page.entries[0].label(), "ALPHA");
    assert_eq!(page.entries[1].setting(), Some("audio.music_pool.b"));
    assert_eq!(page.subtitles[1].as_deref(), Some("/path/b"));
    assert_eq!(
        menu.expanded_settings(TEMPLATE),
        ["audio.music_pool.a", "audio.music_pool.b"]
    );
    // A row steps on its own and reports its own setting.
    let event = menu.adjust(1);
    assert_eq!(
        event,
        Some(MenuEvent::Changed {
            setting: "audio.music_pool.a".to_string(),
            value: Value::Text("on".to_string()),
        })
    );
}

#[test]
fn no_items_leave_no_row_and_no_link_to_the_page() {
    let mut definition = built_in();
    assert_eq!(definition.expand_rows(ValueSource::MusicPool, &[]), 0);
    definition.drop_submenus_to("music");
    let menu = Menu::new(definition);
    assert!(menu.expanded_settings(TEMPLATE).is_empty());
}
