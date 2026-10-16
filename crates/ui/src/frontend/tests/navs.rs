//! The sounds the boot screens call for. `LogoFMV` and `Show Logo` are
//! measured on Pulse (USA): each plays `ACCEPT`. The language picker's are by
//! widget class, and nothing was walked.

use super::*;

use crate::menu::nav::{Dir, Nav};
use crate::pointer::Pointer;

fn press(frontend: &mut Frontend, input: &mut Input, button: Button) {
    input.begin_frame(button.bit());
    frontend.update(FRAME, input, None);
    input.begin_frame(0);
    frontend.update(FRAME, input, None);
}

#[test]
fn skipping_the_movie_plays_accept_on_either_button() {
    for button in [Button::Start, Button::Cross] {
        let mut frontend = frontend(60);
        let mut input = Input::new();
        press(&mut frontend, &mut input, button);
        assert_eq!(frontend.take_nav(), [Nav::Accept], "{button:?}");
        assert!(frontend.take_nav().is_empty(), "taking drains");
    }
}

#[test]
fn a_movie_that_ends_on_its_own_is_silent() {
    let mut frontend = frontend(2);
    let mut input = Input::new();
    assert!(run_until(&mut frontend, &mut input, 1200, |f| {
        f.machine().is(states::LANGUAGE_SELECTION)
    }));
    assert!(frontend.take_nav().is_empty());
}

#[test]
fn the_picker_steps_and_confirms_with_the_menu_and_confirm_button_cues() {
    let mut frontend = frontend(60);
    let mut input = Input::new();
    press(&mut frontend, &mut input, Button::Start);
    let _ = frontend.take_nav();
    assert!(frontend.machine().is(states::LANGUAGE_SELECTION));
    press(&mut frontend, &mut input, Button::Down);
    press(&mut frontend, &mut input, Button::Up);
    assert_eq!(
        frontend.take_nav(),
        [Nav::Moved(Some(Dir::Down)), Nav::Moved(Some(Dir::Up))]
    );
    press(&mut frontend, &mut input, Button::Cross);
    assert_eq!(frontend.take_nav(), [Nav::Accept]);
}

#[test]
fn pressing_on_the_logo_plays_accept() {
    let mut frontend = frontend(60);
    let mut input = Input::new();
    pick_a_language(&mut frontend, &mut input);
    assert!(frontend.machine().is(states::SHOW_LOGO));
    let _ = frontend.take_nav();
    press(&mut frontend, &mut input, Button::Start);
    assert_eq!(frontend.take_nav(), [Nav::Accept]);
}

#[test]
fn the_picker_answers_the_pointer_with_the_pads_cues() {
    let mut frontend = frontend(60);
    let mut input = Input::new();
    press(&mut frontend, &mut input, Button::Start);
    let _ = frontend.take_nav();
    let rows: Vec<(f32, f32)> = {
        let list = frontend.draw_list();
        frontend
            .languages()
            .iter()
            .map(|language| {
                list.iter()
                    .find_map(|draw| match draw {
                        Draw::Text { x, y, text, .. } if *text == language.native_name => {
                            Some((*x + 1.0, *y + 1.0))
                        }
                        _ => None,
                    })
                    .expect("every language is drawn")
            })
            .collect()
    };
    let over = |at| Pointer {
        at: Some(at),
        moved: true,
        ..Pointer::default()
    };
    frontend.pointer(&over(rows[1]));
    frontend.pointer(&over(rows[1]));
    assert_eq!(
        frontend.take_nav(),
        [Nav::Moved(None)],
        "one step, not per event"
    );
    let click = Pointer {
        clicked: true,
        ..over(rows[1])
    };
    frontend.pointer(&click);
    assert_eq!(frontend.take_nav(), [Nav::Accept]);
}
