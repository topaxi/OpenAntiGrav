//! [`Menu::trail`] and [`Menu::restore_trail`]: a menu reopened on a rebuilt tree.

use super::*;

/// The fixture with one row more on OPTIONS, before the one the cursor is on: what a mod
/// turned on adds.
fn grown() -> Definition {
    let mut definition = fixture();
    let options = definition
        .pages
        .iter_mut()
        .find(|page| page.id == "options")
        .unwrap();
    options.entries.insert(0, options.entries[0].clone());
    options.meta.insert(0, options.meta[0].clone());
    definition
}

#[test]
fn a_rebuilt_tree_reopens_on_the_same_page_and_row_and_back_leads_where_it_led() {
    let mut menu = Menu::new(fixture());
    press(&mut menu, &[Button::Cross]);
    press(&mut menu, &[Button::Down]);
    assert_eq!(menu.page().id, "options");
    assert_eq!(menu.selected(), 1, "SWITCH");
    let trail = menu.trail();
    assert_eq!(trail[1].row.as_deref(), Some("graphics.switch"));

    let mut rebuilt = Menu::new(grown());
    assert!(rebuilt.restore_trail(&trail));
    assert_eq!(rebuilt.page().id, "options");
    assert_eq!(
        rebuilt.selected(),
        2,
        "found by its setting, not its old index"
    );
    assert_eq!(rebuilt.depth(), 2);
    press(&mut rebuilt, &[Button::Circle]);
    assert_eq!(rebuilt.page().id, "main", "BACK leads where it led");
}

#[test]
fn a_page_the_rebuilt_tree_lacks_stops_the_restore_where_it_was_found() {
    let mut menu = Menu::new(fixture());
    press(&mut menu, &[Button::Cross]);
    let mut trail = menu.trail();
    trail[1].page = "gone".to_string();
    let mut rebuilt = Menu::new(fixture());
    assert!(!rebuilt.restore_trail(&trail));
    assert_eq!(rebuilt.page().id, "main");
}
