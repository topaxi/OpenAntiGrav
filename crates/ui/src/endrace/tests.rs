//! Model-level tests: [`EndRaceMenu`] navigation and [`Rewards`]' own
//! glyph predicate - everything that needs no [`crate::menu::Layers`]. See
//! `endrace/draw/tests.rs` for the draw-list ones, split the same way
//! `campaign`'s own tests are.

use super::*;
use oag_gameplay::input::Input;
use oag_tables::race_campaign::Medal;

fn options() -> EndRaceMenu {
    EndRaceMenu::new(
        vec![
            MenuOption::ReturnToGrid,
            MenuOption::RaceAgain,
            MenuOption::ViewResultsAgain,
        ],
        Some(2960),
    )
}

/// One press of `button`, the same `begin_frame(0)`-then-`begin_frame(mask)`
/// idiom `crate::campaign::tests::press` uses to raise a rising edge
/// [`Input::take`] reads as a tap.
fn press(input: &mut Input, button: Button) {
    input.begin_frame(0);
    input.begin_frame(1 << button as u32);
}

#[test]
fn down_then_up_returns_to_the_first_row() {
    let mut model = options();
    let mut input = Input::new();
    press(&mut input, Button::Down);
    let events = model.update(&mut input);
    assert_eq!(events, vec![Event::Moved]);
    assert_eq!(model.index(), 1);

    press(&mut input, Button::Up);
    model.update(&mut input);
    assert_eq!(model.index(), 0);
}

#[test]
fn navigation_wraps_at_both_ends() {
    let mut model = options();
    let mut input = Input::new();
    press(&mut input, Button::Up);
    model.update(&mut input);
    assert_eq!(model.selected(), Some(MenuOption::ViewResultsAgain));
}

#[test]
fn cross_confirms_the_selected_row() {
    let mut model = options();
    let mut input = Input::new();
    press(&mut input, Button::Cross);
    assert_eq!(model.update(&mut input), vec![Event::Confirmed]);
}

#[test]
fn circle_backs_out() {
    let mut model = options();
    let mut input = Input::new();
    press(&mut input, Button::Circle);
    assert_eq!(model.update(&mut input), vec![Event::Back]);
}

/// The one measured case (`results-02.png`): a campaign race with no medal
/// shows the hex-dash glyph. A non-campaign race never shows it - the
/// decompile's own "hides `MedalImg` unconditionally" branch - and a
/// medal that *is* earned resolves to a trophy instead, so the glyph stays
/// off there too. See `docs::ui::endrace-screens.md`.
#[test]
fn the_no_medal_glyph_shows_only_on_a_medal_less_campaign_race() {
    assert!(
        Rewards {
            medal: None,
            campaign: true,
            loyalty: None,
        }
        .shows_no_medal_glyph()
    );
    assert!(
        !Rewards {
            medal: None,
            campaign: false,
            loyalty: None,
        }
        .shows_no_medal_glyph()
    );
    assert!(
        !Rewards {
            medal: Some(Medal::Gold),
            campaign: true,
            loyalty: None,
        }
        .shows_no_medal_glyph()
    );
}
