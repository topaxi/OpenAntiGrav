//! Rows attached to another, on a strip title: shown below their parent while the
//! parent or one of its attached rows has focus, steered by the pad and the pointer,
//! and not there at all when nothing in their family is focused.

use super::*;
use crate::menu::pointer;

fn revealing() -> Menu {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    menu.set_reveal_attached(true);
    menu.settle();
    menu
}

fn skin() -> Skin {
    Skin::new(
        oag_hd::frontend::MENU_SKIN,
        oag_display::space::Space::HD,
        22.0,
    )
}

fn id(menu: &Menu) -> String {
    menu.page().meta[menu.selected()]
        .id
        .clone()
        .unwrap_or_default()
}

fn drawn(menu: &Menu) -> Vec<String> {
    menu.tabs()
        .iter()
        .map(|tab| menu.page().entries[tab.row].label().to_string())
        .collect()
}

#[test]
fn only_the_focused_rows_attached_rows_are_drawn_below_it() {
    let mut menu = revealing();
    // On RACE CAMPAIGN nothing is attached to anything focused.
    assert_eq!(id(&menu), "campaign");
    assert_eq!(
        drawn(&menu),
        ["RACE CAMPAIGN", "RACEBOX", "OPTIONS", "RECORDS"]
    );
    // On RACEBOX, REMIX and SPLIT SCREEN show under it.
    press(&mut menu, &[Button::Right]);
    assert_eq!(id(&menu), "racebox");
    assert_eq!(
        drawn(&menu),
        [
            "RACE CAMPAIGN",
            "RACEBOX",
            "REMIX",
            "SPLIT SCREEN",
            "OPTIONS",
            "RECORDS"
        ]
    );
    assert_eq!(menu.tabs().last().map(|t| t.depth), Some(0));
    assert_eq!(menu.tabs()[2].depth, 1, "REMIX hangs one tab below RACEBOX");
    // On OPTIONS it is QUIT's turn.
    press(&mut menu, &[Button::Right]);
    assert_eq!(
        drawn(&menu),
        [
            "RACE CAMPAIGN",
            "RACEBOX",
            "OPTIONS",
            "MODS",
            "QUIT",
            "RECORDS"
        ]
    );
}

#[test]
fn the_pad_steps_the_top_level_left_and_right_and_a_family_up_and_down() {
    let mut menu = revealing();
    press(&mut menu, &[Button::Right]);
    press(&mut menu, &[Button::Down]);
    assert_eq!(id(&menu), "remix");
    press(&mut menu, &[Button::Down]);
    assert_eq!(id(&menu), "split_screen");
    // The end of a family is an end: no wrap into the next row.
    press(&mut menu, &[Button::Down]);
    assert_eq!(id(&menu), "split_screen");
    // Right from an attached row goes to the next top-level row, not its sibling.
    press(&mut menu, &[Button::Right]);
    assert_eq!(id(&menu), "options");
    press(&mut menu, &[Button::Down]);
    assert_eq!(id(&menu), "mods");
    press(&mut menu, &[Button::Down]);
    assert_eq!(id(&menu), "quit");
    press(&mut menu, &[Button::Up]);
    press(&mut menu, &[Button::Up]);
    press(&mut menu, &[Button::Up]);
    assert_eq!(id(&menu), "options", "up from the head stays on it");
    // Left wraps the top level; right wraps back.
    press(&mut menu, &[Button::Left]);
    press(&mut menu, &[Button::Left]);
    press(&mut menu, &[Button::Left]);
    assert_eq!(id(&menu), "records");
    press(&mut menu, &[Button::Right]);
    assert_eq!(id(&menu), "campaign");
}

#[test]
fn up_and_down_still_step_a_row_with_nothing_attached() {
    let mut menu = revealing();
    assert_eq!(id(&menu), "campaign");
    press(&mut menu, &[Button::Down]);
    assert_eq!(id(&menu), "racebox", "down from RACE CAMPAIGN steps on");
    // RACEBOX has REMIX under it: up from the head is the family's start and stays.
    press(&mut menu, &[Button::Up]);
    assert_eq!(id(&menu), "racebox");
    press(&mut menu, &[Button::Left]);
    assert_eq!(id(&menu), "campaign");
    press(&mut menu, &[Button::Up]);
    assert_eq!(id(&menu), "records", "and wraps, as a strip always has");
    // RECORDS has nothing attached either.
    press(&mut menu, &[Button::Down]);
    assert_eq!(id(&menu), "campaign");
}

#[test]
fn a_strip_page_with_no_attached_row_behaves_as_it_always_did_with_reveal_on() {
    let text = "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/menu.tosd\", version = \"1.0.0\" }\n\
        root = \"main\"\n[[page]]\nid = \"main\"\n\
        [[page.entry]]\nkind = \"action\"\nlabel = \"A\"\naction = \"quit\"\n\
        [[page.entry]]\nkind = \"action\"\nlabel = \"B\"\naction = \"quit\"\n\
        [[page.entry]]\nkind = \"action\"\nlabel = \"C\"\naction = \"quit\"\n";
    let definition = Definition::parse(text, &crate::language::StringTable::default()).unwrap();
    let mut menu = Menu::new(definition);
    menu.set_strip_layout(true);
    menu.set_reveal_attached(true);
    assert!(menu.tabs().iter().all(|tab| tab.depth == 0));
    press(&mut menu, &[Button::Down]);
    assert_eq!(menu.selected(), 1);
    press(&mut menu, &[Button::Right]);
    assert_eq!(menu.selected(), 2);
    press(&mut menu, &[Button::Down]);
    assert_eq!(menu.selected(), 0, "wraps");
}

#[test]
fn confirm_on_a_revealed_row_opens_it() {
    let mut menu = revealing();
    press(&mut menu, &[Button::Right]);
    press(&mut menu, &[Button::Down]);
    assert_eq!(id(&menu), "remix");
    press(&mut menu, &[Button::Cross]);
    assert_eq!(menu.page().id, "remix");
}

#[test]
fn a_hidden_row_has_no_pointer_region_and_a_revealed_one_is_clickable() {
    let mut menu = revealing();
    let skin = skin();
    let frame = Frame {
        ink: Some([0.4, 0.4, 0.4, 1.0]),
        tab_selected: Some([0.54, 0.75, 0.79, 1.0]),
        ..Frame::default()
    };
    let rows = |menu: &Menu| -> Vec<usize> {
        pointer::regions(menu, &skin, &frame, &measure)
            .iter()
            .map(|region| region.row)
            .collect()
    };
    let remix = menu.page().row_with_id("remix").unwrap();
    let quit = menu.page().row_with_id("quit").unwrap();
    assert!(
        !rows(&menu).contains(&remix),
        "REMIX is hidden under CAMPAIGN"
    );
    assert!(!rows(&menu).contains(&quit));

    press(&mut menu, &[Button::Right]);
    let regions = pointer::regions(&menu, &skin, &frame, &measure);
    let under = regions
        .iter()
        .find(|r| r.row == remix)
        .expect("REMIX revealed");
    let head = regions
        .iter()
        .find(|r| r.row == menu.page().row_with_id("racebox").unwrap())
        .unwrap();
    assert!(under.rect[1] > head.rect[1], "below its parent");
    assert!(
        (under.rect[0] - head.rect[0]).abs() < 0.01,
        "under the same tab"
    );
    assert!(
        !rows(&menu).contains(&quit),
        "QUIT stays hidden under RACEBOX"
    );

    // A click on the revealed row opens its page.
    let at = (under.rect[0] + 4.0, under.rect[1] + 4.0);
    let click = crate::pointer::Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        ..crate::pointer::Pointer::default()
    };
    menu.pointer(&click, &regions);
    assert_eq!(menu.page().id, "remix");
}

/// A finger has no hover: its tap is a move and a click in one tick. On a row whose
/// attached rows are hidden, the first tap only reveals them (else RACEBOX opens
/// with REMIX and SPLIT SCREEN unreachable), and a second tap activates.
#[test]
fn a_tap_on_a_row_with_hidden_attached_rows_reveals_them_first() {
    let mut menu = revealing();
    let skin = skin();
    let frame = Frame {
        ink: Some([0.4, 0.4, 0.4, 1.0]),
        tab_selected: Some([0.54, 0.75, 0.79, 1.0]),
        ..Frame::default()
    };
    assert_eq!(id(&menu), "campaign");
    let racebox = menu.page().row_with_id("racebox").unwrap();
    let tap_on = |menu: &mut Menu, row: usize| {
        let regions = pointer::regions(menu, &skin, &frame, &measure);
        let region = regions.iter().find(|r| r.row == row).expect("drawn");
        let tap = crate::pointer::Pointer {
            at: Some((region.rect[0] + 4.0, region.rect[1] + 4.0)),
            moved: true,
            clicked: true,
            ..crate::pointer::Pointer::default()
        };
        menu.pointer(&tap, &regions);
    };

    tap_on(&mut menu, racebox);
    assert_eq!(
        menu.page().id,
        "main",
        "the first tap does not open RACEBOX"
    );
    assert_eq!(id(&menu), "racebox");
    assert!(
        drawn(&menu).contains(&"SPLIT SCREEN".to_string()),
        "its rows show"
    );

    // A revealed child is one tap away.
    let remix = menu.page().row_with_id("remix").unwrap();
    tap_on(&mut menu, remix);
    assert_eq!(
        menu.page().id,
        "remix",
        "a revealed row activates on its first tap"
    );
}

#[test]
fn a_column_title_or_no_reveal_flag_draws_every_row_in_order() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    // Not revealing: the flat order, attached rows included.
    assert_eq!(
        drawn(&menu),
        [
            "RACE CAMPAIGN",
            "RACEBOX",
            "REMIX",
            "SPLIT SCREEN",
            "OPTIONS",
            "MODS",
            "QUIT",
            "RECORDS"
        ]
    );
    assert!(menu.tabs().iter().all(|tab| tab.depth == 0));
}
