//! The reset button: Square puts the selected value row back on its default, and is
//! otherwise silent.

use super::*;
use crate::menu::nav::Nav;
use crate::menu::pointer;
use crate::pointer::Pointer;

fn options_menu() -> Menu {
    let mut menu = Menu::new(fixture());
    menu.set_default("graphics.anisotropy", Value::Text("4x".to_string()));
    menu.set_default("graphics.switch", Value::Flag(true));
    press(&mut menu, &[Button::Cross]);
    menu
}

#[test]
fn square_on_a_choice_off_its_default_restores_it_and_reports_the_change() {
    let mut menu = options_menu();
    assert!(menu.seed("graphics.anisotropy", &Value::Text("16x".to_string())));
    assert!(menu.can_reset());
    let events = press(&mut menu, &[Button::Square]);
    assert_eq!(
        events,
        vec![MenuEvent::Changed {
            setting: "graphics.anisotropy".to_string(),
            value: Value::Text("4x".to_string()),
        }]
    );
    assert_eq!(
        menu.page().entries[0].value(),
        Some(Value::Text("4x".to_string()))
    );
    assert!(!menu.can_reset());
}

#[test]
fn square_on_a_toggle_off_its_default_flips_it_back() {
    let mut menu = options_menu();
    press(&mut menu, &[Button::Down]);
    let events = press(&mut menu, &[Button::Square]);
    assert_eq!(
        events,
        vec![MenuEvent::Changed {
            setting: "graphics.switch".to_string(),
            value: Value::Flag(true),
        }]
    );
}

#[test]
fn square_on_a_row_already_at_its_default_does_nothing() {
    let mut menu = options_menu();
    assert!(menu.seed("graphics.anisotropy", &Value::Text("4x".to_string())));
    assert!(!menu.can_reset());
    assert!(press(&mut menu, &[Button::Square]).is_empty());
    assert!(
        menu.take_nav()
            .iter()
            .all(|nav| !matches!(nav, Nav::Stepped(_)))
    );
}

#[test]
fn square_on_a_row_with_no_recorded_default_does_nothing() {
    let mut menu = Menu::new(fixture());
    press(&mut menu, &[Button::Cross]);
    assert!(press(&mut menu, &[Button::Square]).is_empty());
}

#[test]
fn square_on_an_action_or_submenu_row_does_nothing() {
    let mut menu = Menu::new(fixture());
    menu.set_default("graphics.anisotropy", Value::Text("4x".to_string()));
    assert!(press(&mut menu, &[Button::Square]).is_empty());
    press(&mut menu, &[Button::Down]);
    assert!(press(&mut menu, &[Button::Square]).is_empty());
}

#[test]
fn square_on_a_disabled_row_does_nothing() {
    let definition = Definition::parse(
        r#"
        toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/menu.tosd", version = "1.0.0" }
        root = "main"
        [[page]]
        id = "main"
        title = "MAIN"
        [[page.entry]]
        kind = "toggle"
        label = "VSYNC"
        setting = "vsync"
        [[page.entry]]
        kind = "choice"
        label = "LIMIT"
        setting = "limit"
        values = ["60", "120"]
        disabled_by = { setting = "vsync", value = true }
        "#,
        &crate::language::StringTable::default(),
    )
    .expect("parses");
    let mut menu = Menu::new(definition);
    menu.set_default("limit", Value::Text("60".to_string()));
    assert!(menu.seed("vsync", &Value::Flag(true)));
    assert!(menu.seed("limit", &Value::Text("120".to_string())));
    press(&mut menu, &[Button::Down]);
    assert!(!menu.can_reset());
    assert!(press(&mut menu, &[Button::Square]).is_empty());
}

#[test]
fn a_default_the_row_does_not_offer_changes_nothing() {
    let mut menu = options_menu();
    menu.set_default("graphics.anisotropy", Value::Text("64x".to_string()));
    assert!(press(&mut menu, &[Button::Square]).is_empty());
}

#[test]
fn the_middle_button_resets_the_row_under_the_pointer() {
    let mut menu = options_menu();
    assert!(menu.seed("graphics.anisotropy", &Value::Text("16x".to_string())));
    let regions = pointer::regions(&menu, &skin(), &Frame::default(), &measure);
    let at = (regions[0].rect[0] + 1.0, regions[0].rect[1] + 1.0);
    let middle = Pointer {
        at: Some(at),
        reset: true,
        ..Pointer::default()
    };
    let events = menu.pointer(&middle, &regions);
    assert_eq!(
        events,
        vec![MenuEvent::Changed {
            setting: "graphics.anisotropy".to_string(),
            value: Value::Text("4x".to_string()),
        }]
    );
}
