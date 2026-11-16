//! The SCREEN SIZE row on the DISPLAY page, driven the way a player drives it: the range is the
//! booted title's `Safe Area Setting` slider, it opens on the value the title draws at, and a
//! step emits the setting the session writes to `[graphics] hud_safe_area`.
//!
//! An integration test because it checks `oag-ui`'s definition against `oag-game`'s seeds and
//! the title packages' data, a claim about all three at once.

use oag_game::settings::{Settings, menu_seeds};
use oag_gameplay::input::{Button, Input};
use oag_ui::menu::*;

const SETTING: &str = "graphics.hud_safe_area";

const TITLES: [(&oag_title::Title, oag_disc::Platform, &str, u8); 5] = [
    (oag_hd::TITLE, oag_disc::Platform::Ps3, "HD", 90),
    (oag_pulse::TITLE, oag_disc::Platform::Psp, "Pulse", 100),
    (oag_pure::TITLE, oag_disc::Platform::Psp, "Pure", 100),
    (oag_2048::TITLE, oag_disc::Platform::Vita, "2048", 100),
    (oag_omega::TITLE, oag_disc::Platform::Ps4, "Omega", 100),
];

fn press(menu: &mut Menu, buttons: &[Button]) -> Vec<MenuEvent> {
    let mut input = Input::new();
    let mask = buttons.iter().fold(0u32, |mask, &b| mask | 1 << b.index());
    input.begin_frame(mask);
    menu.update(&mut input)
}

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

/// The DISPLAY page as `Session::open_menus` builds it: the title's values supplied and the
/// saved value seeded.
fn display(title: &oag_title::Title, platform: oag_disc::Platform, saved: Option<u8>) -> Menu {
    let mut definition = Definition::built_in(&oag_ui::language::StringTable::default())
        .expect("the built-in menu must parse");
    definition.drop_unavailable_hud_style(title);
    definition.drop_unavailable_dynamic_hud(title);
    let mut menu = Menu::new(definition);
    assert!(menu.open("display"), "the display page exists");
    menu.supply(ValueSource::SafeAreas, &safe_area_choices(title));
    let mut settings = Settings::default();
    settings.graphics.hud_safe_area = saved;
    for (setting, value) in menu_seeds(&settings, Default::default(), title, platform) {
        menu.seed(setting, &value);
    }
    oag_game::settings::supply_reset(
        &mut menu,
        title,
        platform,
        &oag_ui::language::StringTable::default(),
    );
    menu
}

fn value(menu: &Menu) -> String {
    menu.value_of(SETTING)
        .expect("the SCREEN SIZE row holds a value")
        .to_string()
}

fn offered(menu: &Menu) -> Vec<String> {
    match menu
        .page()
        .entries
        .iter()
        .find(|entry| entry.setting() == Some(SETTING))
        .expect("the SCREEN SIZE row is on the DISPLAY page")
    {
        Entry::Choice { values, .. } => values.iter().map(|choice| choice.value.clone()).collect(),
        other => panic!("not a choice row: {other:?}"),
    }
}

#[test]
fn every_title_offers_the_row_with_the_sliders_range() {
    for (title, platform, name, _) in TITLES {
        let want: Vec<String> = (80..=100).step_by(2).map(|v: u8| v.to_string()).collect();
        assert_eq!(offered(&display(title, platform, None)), want, "{name}");
    }
}

#[test]
fn the_row_opens_on_what_the_title_draws_at() {
    for (title, platform, name, default) in TITLES {
        assert_eq!(
            value(&display(title, platform, None)),
            default.to_string(),
            "{name}"
        );
        assert_eq!(
            value(&display(title, platform, Some(96))),
            "96",
            "{name}: a saved value wins"
        );
    }
}

#[test]
fn a_step_emits_the_setting_the_session_writes() {
    for (title, platform, name, default) in TITLES {
        let mut menu = display(title, platform, None);
        select(&mut menu, SETTING);
        let events = press(&mut menu, &[Button::Left]);
        let [MenuEvent::Changed { setting, value }] = events.as_slice() else {
            panic!("{name}: one change: {events:?}");
        };
        assert_eq!(setting, SETTING, "{name}");
        let lower = default - 2;
        assert_eq!(
            value.to_string(),
            lower.to_string(),
            "{name}: one step of two"
        );
    }
}

/// The pointer steps the row through the arrows the layout puts on it, as it steps every choice
/// row: a click on the forward arrow emits the next value and one on the back arrow the last.
#[test]
fn the_pointer_steps_the_row_through_its_arrows() {
    use oag_ui::menu::pointer::{Part, Region};
    use oag_ui::pointer::Pointer;
    for (title, platform, name, default) in TITLES {
        let mut menu = display(title, platform, None);
        let row = menu
            .page()
            .entries
            .iter()
            .position(|entry| entry.setting() == Some(SETTING))
            .unwrap_or_else(|| panic!("{name}: no SCREEN SIZE row"));
        let arrows = |part| {
            vec![Region {
                rect: [0.0, 0.0, 10.0, 10.0],
                row,
                part,
            }]
        };
        let click = Pointer {
            at: Some((5.0, 5.0)),
            moved: true,
            clicked: true,
            ..Pointer::default()
        };
        let events = menu.pointer(&click, &arrows(Part::StepBack));
        let [MenuEvent::Changed { setting, value }] = events.as_slice() else {
            panic!("{name}: one change: {events:?}");
        };
        assert_eq!(setting, SETTING, "{name}");
        assert_eq!(value.to_string(), (default - 2).to_string(), "{name}: back");
        let events = menu.pointer(&click, &arrows(Part::StepForward));
        let [MenuEvent::Changed { value, .. }] = events.as_slice() else {
            panic!("{name}: one change: {events:?}");
        };
        assert_eq!(value.to_string(), default.to_string(), "{name}: forward");
    }
}

/// The reset button puts the row back on what the title draws at with nothing saved: HD's 90 and
/// every other title's 100.
#[test]
fn reset_goes_to_the_titles_own_default() {
    for (title, platform, name, default) in TITLES {
        let mut menu = display(title, platform, Some(84));
        select(&mut menu, SETTING);
        let events = press(&mut menu, &[Button::Square]);
        let [MenuEvent::Changed { setting, value }] = events.as_slice() else {
            panic!("{name}: one change: {events:?}");
        };
        assert_eq!(setting, SETTING, "{name}");
        assert_eq!(value.to_string(), default.to_string(), "{name}");
    }
}
