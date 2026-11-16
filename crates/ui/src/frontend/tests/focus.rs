//! 2048's grids from a controller alone: the focus is always drawn, every
//! direction moves it, Cross reaches the focused control and Circle backs out.

use super::wipeout2048::{boot, focused, press, reach_the_grid};
use super::*;
use crate::pointer::Pointer;
use oag_2048::frontend::states as w2048;

const UPPER: (f32, f32) = (86.0, 190.0);
const LOWER: (f32, f32) = (86.0, 360.0);
const TICK: (f32, f32) = (822.0, 432.0);

fn on_home() -> (Frontend, Input) {
    let mut frontend = boot(0);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Triangle);
    assert!(frontend.machine().is(w2048::HOME));
    (frontend, input)
}

#[test]
fn a_screen_entered_by_pad_marks_its_first_control_before_any_direction() {
    let (frontend, _) = on_home();
    assert!(focused(&frontend, UPPER.0, UPPER.1));
    assert!(!focused(&frontend, LOWER.0, LOWER.1));
}

#[test]
fn down_and_up_move_the_focus_by_layout_and_cross_reaches_the_focused_tile() {
    let (mut frontend, mut input) = on_home();
    press(&mut frontend, &mut input, Button::Down);
    assert!(focused(&frontend, LOWER.0, LOWER.1));
    assert!(!focused(&frontend, UPPER.0, UPPER.1));
    press(&mut frontend, &mut input, Button::Down);
    assert!(
        focused(&frontend, TICK.0, TICK.1),
        "the tick sits below the column and is reached by Down"
    );
    press(&mut frontend, &mut input, Button::Down);
    assert!(
        focused(&frontend, TICK.0, TICK.1),
        "the bottom is the end: the grid does not wrap vertically"
    );
    press(&mut frontend, &mut input, Button::Up);
    assert!(focused(&frontend, LOWER.0, LOWER.1));
    press(&mut frontend, &mut input, Button::Up);
    assert!(focused(&frontend, UPPER.0, UPPER.1));
    press(&mut frontend, &mut input, Button::Down);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(
        frontend.machine().is(w2048::GAME_MODE_CHOICE),
        "the lower tile's own redirect, not the upper one's"
    );
}

#[test]
fn circle_backs_out_of_home_through_its_own_tick() {
    let (mut frontend, mut input) = on_home();
    press(&mut frontend, &mut input, Button::Circle);
    assert!(frontend.machine().is(w2048::NEW_FE_SHELL));
}

#[test]
fn circle_on_the_root_grid_stays_put() {
    let mut frontend = boot(0);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Circle);
    assert!(frontend.machine().is(w2048::GAME_MODE_CHOICE));
}

#[test]
fn cross_on_the_team_screen_presses_its_confirm_tick() {
    let (mut frontend, mut input) = on_home();
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::TEAM));
    press(&mut frontend, &mut input, Button::Cross);
    assert!(
        frontend.machine().is(w2048::HOME),
        "the tick's PreviousScreen, as Circle's"
    );
}

#[test]
fn a_compare_shot_hides_the_focus_the_pad_would_draw() {
    let (mut frontend, _input) = on_home();
    assert!(focused(&frontend, UPPER.0, UPPER.1));
    frontend.hide_pad_focus();
    assert!(!focused(&frontend, UPPER.0, UPPER.1));
    frontend.set_pad_driving(true);
    assert!(!focused(&frontend, UPPER.0, UPPER.1));
}

#[test]
fn a_pointer_takes_the_focus_away_and_the_pad_gives_it_back() {
    let (mut frontend, mut input) = on_home();
    let hover = Pointer {
        at: Some((600.0, 100.0)),
        moved: true,
        ..Pointer::default()
    };
    assert!(frontend.pointer(&hover));
    assert!(!focused(&frontend, UPPER.0, UPPER.1));
    frontend.set_pad_driving(true);
    assert!(focused(&frontend, UPPER.0, UPPER.1));
    press(&mut frontend, &mut input, Button::Right);
    assert!(
        !focused(&frontend, UPPER.0, UPPER.1),
        "Right moved the focus off the first tile"
    );
}

/// Back on the mode grid after the map, Circle must not run the grid's own redirect again or pop
/// the stack the first confirm left behind.
#[test]
fn circle_on_the_mode_grid_after_a_visit_to_the_map_stays_on_the_grid() {
    let mut frontend = boot(0);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::NEW_FE_SHELL));
    for _ in 0..2 {
        press(&mut frontend, &mut input, Button::Circle);
        assert!(frontend.machine().is(w2048::GAME_MODE_CHOICE));
        press(&mut frontend, &mut input, Button::Circle);
        assert!(
            frontend.machine().is(w2048::GAME_MODE_CHOICE),
            "Circle on the root grid does nothing"
        );
        press(&mut frontend, &mut input, Button::Cross);
        assert!(frontend.machine().is(w2048::NEW_FE_SHELL));
        press(&mut frontend, &mut input, Button::Triangle);
        assert!(frontend.machine().is(w2048::HOME));
        press(&mut frontend, &mut input, Button::Circle);
        assert!(
            frontend.machine().is(w2048::NEW_FE_SHELL),
            "Home's tick returns to the map"
        );
    }
}

/// A tile beside the focused one that sits a few units lower is not "below" it: the extra tiles
/// stand in a column left of the grid, and ADHOC-like bottom-row tiles must not jump to them.
#[test]
fn down_does_not_jump_sideways_to_a_tile_a_few_units_lower() {
    let mut frontend = boot(0);
    frontend.set_extra_tiles(vec![
        ExtraTile {
            label: "ONE".into(),
            launch: Launch::Page("one".into()),
        },
        ExtraTile {
            label: "TWO".into(),
            launch: Launch::Page("two".into()),
        },
    ]);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Right);
    assert!(focused(&frontend, 329.0, 190.0), "the second mode tile");
    press(&mut frontend, &mut input, Button::Down);
    assert!(
        focused(&frontend, 329.0, 190.0),
        "nothing is below it: the extra tile at y 226 is a column away, only 36 units lower"
    );
}
