//! A list page (`super::super::items`): rows that open, Square and the ON/OFF column that
//! toggle, and an order that is decided on entry.

use super::*;

use crate::menu::pointer::{self, Part};
use crate::pointer::Pointer;

const TREE: &str = r#"
    toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/menu.tosd", version = "1.0.0" }
    root = "main"
    [[page]]
    id = "main"
    title = "MAIN"
    [[page.entry]]
    kind = "submenu"
    label = "LIST"
    target = "list"
    [[page]]
    id = "list"
    title = "LIST"
    item_page = "detail"
    [[page.entry]]
    kind = "choice"
    label = "ITEM"
    setting = "x.row"
    values_from = "mod_list"
    [[page.entry]]
    kind = "back"
    label = "BACK"
    [[page]]
    id = "detail"
    title = "DETAIL"
    [[page.entry]]
    kind = "back"
    label = "BACK"
"#;

/// `id`, holding `on` or `off`, or `fixed`: one value, nothing to step to.
fn item(id: &str, value: &str) -> RowItem {
    let values = match value {
        "fixed" => vec![Choice::labelled("on", "ALWAYS ON")],
        _ => vec![Choice::labelled("on", "ON"), Choice::labelled("off", "OFF")],
    };
    RowItem {
        id: id.to_string(),
        label: id.to_uppercase(),
        detail: None,
        values,
        current: if value == "fixed" { "on" } else { value }.to_string(),
    }
}

/// The tree with one row per `(id, value)`, standing on `main`.
fn model(items: &[(&str, &str)]) -> Menu {
    let mut definition =
        Definition::parse(TREE, &crate::language::StringTable::default()).expect("parses");
    let items: Vec<RowItem> = items.iter().map(|&(id, value)| item(id, value)).collect();
    assert_eq!(
        definition.expand_rows(ValueSource::ModList, &items),
        items.len()
    );
    let mut menu = Menu::new(definition);
    menu.set_item_toggle_label("TOGGLE");
    menu.set_visible_rows(visible());
    menu
}

fn ids(menu: &Menu) -> Vec<&str> {
    menu.page()
        .meta
        .iter()
        .filter_map(|meta| meta.id.as_deref())
        .collect()
}

fn focused(menu: &Menu) -> &str {
    menu.page().meta[menu.selected()]
        .id
        .as_deref()
        .unwrap_or("-")
}

/// The list entered from `main`, the cursor on `id`.
fn entered(items: &[(&str, &str)], id: &str) -> Menu {
    let mut menu = model(items);
    press(&mut menu, &[Button::Cross]);
    assert_eq!(menu.page().id, "list");
    menu.point_item("list", id);
    assert_eq!(focused(&menu), id);
    menu
}

const FOUR: [(&str, &str); 4] = [("a", "off"), ("b", "on"), ("c", "fixed"), ("d", "off")];

#[test]
fn entering_the_list_puts_the_rows_that_are_on_first_each_group_as_built() {
    let mut menu = model(&FOUR);
    assert!(menu.push("list"), "a jump enters it too");
    assert_eq!(ids(&menu), ["b", "c", "a", "d"]);
    let mut menu = model(&FOUR);
    press(&mut menu, &[Button::Cross]);
    assert_eq!(ids(&menu), ["b", "c", "a", "d"]);
}

#[test]
fn square_steps_the_row_where_it_is_and_the_next_entry_sorts_it() {
    let mut menu = entered(&FOUR, "b");
    let events = press(&mut menu, &[Button::Square]);
    assert_eq!(
        events,
        [MenuEvent::Changed {
            setting: "x.row.b".to_string(),
            value: Value::Text("off".to_string()),
        }]
    );
    assert_eq!(ids(&menu), ["b", "c", "a", "d"], "nothing moves on screen");
    assert_eq!(focused(&menu), "b");

    press(&mut menu, &[Button::Circle]);
    press(&mut menu, &[Button::Cross]);
    assert_eq!(ids(&menu), ["c", "a", "b", "d"], "off now, in build order");
    assert_eq!(focused(&menu), "b", "the cursor followed its row");

    press(&mut menu, &[Button::Square]);
    press(&mut menu, &[Button::Circle]);
    press(&mut menu, &[Button::Cross]);
    assert_eq!(
        ids(&menu),
        ["b", "c", "a", "d"],
        "back on: where it was built, not where it had been moved to"
    );
}

#[test]
fn accept_opens_the_row_and_does_not_step_it() {
    let mut menu = entered(&FOUR, "a");
    for button in [Button::Cross, Button::Start] {
        assert_eq!(
            press(&mut menu, &[button]),
            [MenuEvent::OpenItem("a".to_string())]
        );
    }
    assert_eq!(
        menu.page().entries[menu.selected()].chosen(),
        Some(Value::Text("off".to_string()))
    );
    assert_eq!(menu.page().id, "list", "the owner opens the page");
}

#[test]
fn a_row_with_one_value_opens_and_has_nothing_to_toggle() {
    let mut menu = entered(&FOUR, "c");
    assert!(press(&mut menu, &[Button::Square]).is_empty());
    assert_eq!(menu.reset_offered(), None);
    assert_eq!(
        press(&mut menu, &[Button::Cross]),
        [MenuEvent::OpenItem("c".to_string())]
    );
}

#[test]
fn the_footer_names_square_only_on_a_row_it_toggles() {
    let mut menu = entered(&FOUR, "a");
    assert_eq!(menu.reset_offered(), Some("TOGGLE"));
    menu.point_item("list", "c");
    assert_eq!(menu.reset_offered(), None);
    press(&mut menu, &[Button::Down]);
    press(&mut menu, &[Button::Down]);
    press(&mut menu, &[Button::Down]);
    assert_eq!(menu.page().entries[menu.selected()].label(), "BACK");
    assert_eq!(menu.reset_offered(), None);
}

/// A rebuilt tree (the owner rebuilds on every toggle) puts the rows, the cursor, the window
/// and a pointer's mark back where they were.
#[test]
fn a_rebuilt_list_keeps_the_order_the_cursor_and_the_window() {
    let many: Vec<(String, &str)> = (0..20).map(|n| (format!("m{n:02}"), "on")).collect();
    let many: Vec<(&str, &str)> = many.iter().map(|(id, v)| (id.as_str(), *v)).collect();
    let mut menu = entered(&many, "m00");
    let shown = menu.visible_rows();
    for _ in 0..12 {
        press(&mut menu, &[Button::Down]);
    }
    let window = menu.scroll();
    // The pointer rests on the last row on screen and toggles it.
    let edge = window + shown - 1;
    let regions = pointer::regions(&menu, &skin(), &Frame::default(), &measure);
    let toggle = regions
        .iter()
        .find(|r| r.row == edge && r.part == Part::StepForward)
        .expect("a toggle target");
    let at = (toggle.rect[0] + toggle.rect[2] * 0.5, toggle.rect[1] + 1.0);
    let events = menu.pointer(
        &Pointer {
            at: Some(at),
            moved: true,
            clicked: true,
            ..Pointer::default()
        },
        &regions,
    );
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(menu.scroll(), window, "the click scrolled nothing");
    let held = focused(&menu).to_string();

    let mut rebuilt_items = many.clone();
    rebuilt_items[edge].1 = "off";
    let orders = menu.item_orders();
    let trail = menu.trail();
    let mut rebuilt = model(&rebuilt_items);
    rebuilt.set_item_orders(&orders);
    assert!(rebuilt.restore_trail(&trail));
    assert_eq!(ids(&rebuilt), ids(&menu), "the row did not sink");
    assert_eq!(focused(&rebuilt), held);
    assert_eq!(rebuilt.scroll(), window, "the list did not move");
}

fn click(menu: &mut Menu, skin: &Skin, frame: &Frame, row: usize, part: Part) -> Vec<MenuEvent> {
    let regions = pointer::regions(menu, skin, frame, &measure);
    let region = regions
        .iter()
        .find(|r| r.row == row && r.part == part)
        .unwrap_or_else(|| panic!("row {row} {part:?}: {regions:?}"));
    // Off to the left of a row's middle, where the name is.
    let x = match part {
        Part::Row => region.rect[0] + region.rect[2] * 0.1,
        _ => region.rect[0] + region.rect[2] * 0.5,
    };
    menu.pointer(
        &Pointer {
            at: Some((x, region.rect[1] + region.rect[3] * 0.5)),
            moved: true,
            clicked: true,
            ..Pointer::default()
        },
        &regions,
    )
}

#[test]
fn a_click_on_the_on_off_column_toggles_and_on_the_name_opens() {
    let mut menu = entered(&FOUR, "b");
    let frame = Frame::default();
    // Row 2 is `a`, an off row.
    let events = click(&mut menu, &skin(), &frame, 2, Part::StepForward);
    assert_eq!(
        events,
        [MenuEvent::Changed {
            setting: "x.row.a".to_string(),
            value: Value::Text("on".to_string()),
        }]
    );
    assert_eq!(
        click(&mut menu, &skin(), &frame, 2, Part::Row),
        [MenuEvent::OpenItem("a".to_string())]
    );
    // The one-value row has no toggle target: all of it opens.
    let regions = pointer::regions(&menu, &skin(), &frame, &measure);
    assert!(regions.iter().any(|r| r.row == 1));
    assert!(
        regions
            .iter()
            .all(|r| ![1, 4].contains(&r.row) || r.part == Part::Row),
        "ALWAYS ON and BACK: {regions:?}"
    );
}

/// A bare-text row's toggle target is its right-hand end, where the value is drawn, and not
/// the name.
#[test]
fn the_toggle_target_of_a_text_row_is_where_its_value_is_drawn() {
    let menu = entered(&FOUR, "b");
    let frame = Frame::default();
    let regions = pointer::regions(&menu, &skin(), &frame, &measure);
    let row = regions
        .iter()
        .find(|r| r.row == 0 && r.part == Part::Row)
        .expect("a row");
    let toggle = regions
        .iter()
        .find(|r| r.row == 0 && r.part == Part::StepForward)
        .expect("a toggle");
    assert!((toggle.rect[0] + toggle.rect[2] - (row.rect[0] + row.rect[2])).abs() < 0.01);
    assert!(toggle.rect[0] > row.rect[0] + row.rect[2] * 0.5);
}

/// HD's blocks: the label block opens, the value block toggles.
#[test]
fn on_hds_list_blocks_the_value_block_toggles_and_the_label_block_opens() {
    let skin = Skin::new(
        oag_hd::frontend::MENU_SKIN,
        oag_display::space::Space::HD,
        22.0,
    );
    let frame = super::pointer::hd_frame();
    let mut menu = entered(&FOUR, "b");
    assert_eq!(
        click(&mut menu, &skin, &frame, 2, Part::StepForward),
        [MenuEvent::Changed {
            setting: "x.row.a".to_string(),
            value: Value::Text("on".to_string()),
        }]
    );
    assert_eq!(
        click(&mut menu, &skin, &frame, 2, Part::Row),
        [MenuEvent::OpenItem("a".to_string())]
    );
}
