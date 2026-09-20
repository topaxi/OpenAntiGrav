//! The two SCREEN FILTER rows on the GRAPHICS page, driven the way a player
//! drives them: the list is the catalogue's, moving the row emits the
//! setting the frame loop reads, and the strength row is greyed until a
//! filter is picked.
//!
//! An integration test rather than a unit in either crate because it checks
//! `oag-ui`'s menu definition against `oag-game`'s catalogue - a claim about
//! both sides at once, which only a crate that can see both can make.

use oag_game::screen::Catalogue;
use oag_game::settings::SCREEN_FILTER_OFF;
use oag_gameplay::input::{Button, Input};
use oag_ui::frontend::Draw;
use oag_ui::menu::*;

fn built_in() -> Definition {
    Definition::parse(BUILT_IN, &oag_ui::language::StringTable::default())
        .expect("the built-in menu must parse")
}

fn press(menu: &mut Menu, buttons: &[Button]) -> Vec<MenuEvent> {
    let mut input = Input::new();
    let mask = buttons.iter().fold(0u32, |mask, &b| mask | 1 << b.index());
    input.begin_frame(mask);
    menu.update(&mut input)
}

/// Moves the cursor down until the selected row edits `setting`.
fn select(menu: &mut Menu, setting: &str) {
    for _ in 0..menu.page().entries.len() {
        if menu.page().entries[menu.selected()].setting() == Some(setting) {
            return;
        }
        press(menu, &[Button::Down]);
        press(menu, &[]);
    }
    panic!("{setting} is not on this page");
}

fn row(menu: &Menu, setting: &str) -> Entry {
    menu.page()
        .entries
        .iter()
        .find(|entry| entry.setting() == Some(setting))
        .unwrap_or_else(|| panic!("{setting} is on the graphics page"))
        .clone()
}

/// The GRAPHICS page with the catalogue's list supplied and the profile's
/// default seeded, which is what `Session::open_menus` does.
fn graphics() -> Menu {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("graphics"), "the graphics page exists");
    menu.supply(ValueSource::ScreenFilters, &Catalogue::built_in().choices());
    menu.seed(
        "graphics.screen_filter",
        &Value::Text(SCREEN_FILTER_OFF.to_string()),
    );
    menu.seed(
        "graphics.screen_filter_strength",
        &Value::Text("100".to_string()),
    );
    menu
}

#[test]
fn the_row_offers_off_then_every_built_in_and_moving_it_names_a_preset() {
    let mut menu = graphics();
    select(&mut menu, "graphics.screen_filter");

    let events = press(&mut menu, &[Button::Right]);
    let [MenuEvent::Changed { setting, value }] = events.as_slice() else {
        panic!("one change: {events:?}");
    };
    assert_eq!(setting, "graphics.screen_filter");
    assert_eq!(
        value.to_string(),
        oag_game::screen::BUILT_IN[0].0,
        "the first row after off is the first shipped preset"
    );

    // All the way round: every built-in, then back to off.
    let mut seen = vec![value.to_string()];
    for _ in 0..oag_game::screen::BUILT_IN.len() {
        press(&mut menu, &[]);
        let events = press(&mut menu, &[Button::Right]);
        let [MenuEvent::Changed { value, .. }] = events.as_slice() else {
            panic!("one change: {events:?}");
        };
        seen.push(value.to_string());
    }
    let shipped: Vec<&str> = oag_game::screen::BUILT_IN
        .iter()
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(&seen[..shipped.len()], shipped.as_slice());
    assert_eq!(seen.last().map(String::as_str), Some(SCREEN_FILTER_OFF));
}

#[test]
fn the_strength_row_is_greyed_until_a_filter_is_picked() {
    let mut menu = graphics();
    let strength = row(&menu, "graphics.screen_filter_strength");
    assert!(
        menu.is_disabled(&strength),
        "off means there is nothing for a strength to apply to"
    );

    select(&mut menu, "graphics.screen_filter");
    press(&mut menu, &[Button::Right]);
    assert!(
        !menu.is_disabled(&strength),
        "a picked filter makes the strength row live"
    );

    // And the strength row itself emits its own key when moved.
    press(&mut menu, &[]);
    select(&mut menu, "graphics.screen_filter_strength");
    let events = press(&mut menu, &[Button::Left]);
    let [MenuEvent::Changed { setting, value }] = events.as_slice() else {
        panic!("one change: {events:?}");
    };
    assert_eq!(setting, "graphics.screen_filter_strength");
    assert_eq!(value.to_string(), "90");
}

#[test]
fn a_user_preset_the_catalogue_offers_is_on_the_row_too() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("graphics"));
    let mut choices = Catalogue::built_in().choices();
    choices.push(Choice::labelled("mine", "Mine"));
    menu.supply(ValueSource::ScreenFilters, &choices);
    menu.seed("graphics.screen_filter", &Value::Text("mine".to_string()));
    select(&mut menu, "graphics.screen_filter");
    // Right off the last entry wraps to the first, which is `off`: the row
    // did hold the user's preset rather than falling to the top of the list.
    let events = press(&mut menu, &[Button::Right]);
    let [MenuEvent::Changed { value, .. }] = events.as_slice() else {
        panic!("one change: {events:?}");
    };
    assert_eq!(value.to_string(), SCREEN_FILTER_OFF);
}

/// The rows sit below the GRAPHICS page's fold - eleventh and twelfth of
/// fourteen, against a seven-row window - so this is the check that a player
/// who scrolls to them sees the label, the preset's *name* rather than its
/// id, and the strength beside it. A `--menu-page` still cannot show this:
/// that path places no cursor.
#[test]
fn scrolled_to_the_rows_draw_the_label_and_the_presets_name() {
    let mut menu = graphics();
    menu.set_visible_rows(7);
    select(&mut menu, "graphics.screen_filter");
    press(&mut menu, &[Button::Right]);
    press(&mut menu, &[]);
    select(&mut menu, "graphics.screen_filter_strength");

    let skin = Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let drawn = draw_list(
        &menu,
        &skin,
        &|_| vec!["X"],
        &|text| text.chars().count() as f32 * 6.0,
        None,
        &Frame::default(),
        false,
    )
    .flatten();
    let text = |wanted: &str| {
        drawn
            .iter()
            .any(|draw| matches!(draw, Draw::Text { text, .. } if text == wanted))
    };
    assert!(text("SCREEN FILTER"), "the row's label is drawn");
    assert!(text("PSP-3000 LCD"), "the preset's name, not its id");
    assert!(
        !text("psp-3000"),
        "the id is what is stored, not what is shown"
    );
    assert!(text("SCREEN FILTER STRENGTH"));
    assert!(text("100"));
}
