//! The boot sequence under a mouse or a finger: the picker's rows answer
//! it where they are drawn, and every other screen reports the click as
//! not its own.

use super::*;

use crate::pointer::Pointer;

fn click_at(at: (f32, f32)) -> Pointer {
    Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        ..Pointer::default()
    }
}

/// Skips the movie so the machine sits on the picker.
fn reach_the_picker(frontend: &mut Frontend, input: &mut Input) {
    input.begin_frame(Button::Start.bit());
    frontend.update(FRAME, input, None);
    input.begin_frame(0);
    frontend.update(FRAME, input, None);
    assert!(frontend.machine().is(states::LANGUAGE_SELECTION));
}

/// Where each language's row is drawn: the `Draw::Text` whose text is the
/// language's native name.
fn drawn_rows(frontend: &Frontend) -> Vec<(f32, f32)> {
    let list = frontend.draw_list();
    frontend
        .languages()
        .iter()
        .map(|language| {
            list.iter()
                .find_map(|draw| match draw {
                    Draw::Text { x, y, text, .. } if *text == language.native_name => {
                        Some((*x, *y))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{} is drawn", language.native_name))
        })
        .collect()
}

/// A click on the movie is not the picker's, so the caller may press
/// start for it.
#[test]
fn a_click_on_a_movie_screen_is_reported_as_not_handled() {
    let mut frontend = frontend(300);
    assert!(!frontend.pointer(&click_at((240.0, 136.0))));
    assert!(frontend.machine().is(states::LOGO_FMV), "and nothing moved");
}

/// The drift guard: every row's drawn pen resolves to that row.
#[test]
fn every_drawn_language_row_is_under_its_own_pen() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    reach_the_picker(&mut frontend, &mut input);
    let pens = drawn_rows(&frontend);
    assert_eq!(pens.len(), 3);
    for (index, pen) in pens.iter().enumerate() {
        assert_eq!(
            frontend.language_row_at((pen.0 + 2.0, pen.1 + 2.0)),
            Some(index),
            "row {index} at {pen:?}"
        );
    }
    assert_eq!(
        frontend.language_row_at((pens[0].0 + 2.0, pens[0].1 - 30.0)),
        None
    );
    assert_eq!(frontend.language_row_at((470.0, pens[0].1 + 2.0)), None);
}

/// With the face's ink measured, the band follows the glyphs: a point
/// just under a row's pen - the accent room above its capitals - belongs
/// to the row above, and the capitals' own centre is the row itself. The
/// Pure `Default` face's shape: caps on rows 7 to 16 of a 16-row cell.
#[test]
fn a_measured_face_moves_the_bands_down_onto_the_capitals() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    reach_the_picker(&mut frontend, &mut input);
    frontend.set_row_ink(Some(crate::pointer::RowInk {
        top: 7.0,
        bottom: 16.0,
    }));
    let pens = drawn_rows(&frontend);
    let scale = 1.0;
    let cap_centre = |index: usize| (pens[index].0 + 2.0, pens[index].1 + 11.5 * scale);
    for index in 0..pens.len() {
        assert_eq!(frontend.language_row_at(cap_centre(index)), Some(index));
    }
    assert_eq!(
        frontend.language_row_at((pens[0].0 + 2.0, pens[0].1 + 2.0)),
        None,
        "above the first row's capitals is nothing"
    );
    assert_eq!(
        frontend.language_row_at((pens[1].0 + 2.0, pens[1].1 + 2.0)),
        Some(0),
        "the accent room over row 1 is still row 0's band"
    );
}

#[test]
fn hovering_selects_and_a_second_click_confirms() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    reach_the_picker(&mut frontend, &mut input);
    let pens = drawn_rows(&frontend);
    let over = |index: usize| (pens[index].0 + 2.0, pens[index].1 + 2.0);

    let hover = Pointer {
        at: Some(over(1)),
        moved: true,
        ..Pointer::default()
    };
    assert!(frontend.pointer(&hover));
    assert_eq!(frontend.selected(), 1);
    assert_eq!(frontend.chosen(), None, "a hover chooses nothing");

    // A tap on a row that is not selected selects it and stops.
    assert!(frontend.pointer(&click_at(over(2))));
    assert_eq!(frontend.selected(), 2);
    assert_eq!(frontend.chosen(), None);

    // A tap on the selected row confirms it, and the machine leaves on
    // the next update the way a cross press does.
    assert!(frontend.pointer(&click_at(over(2))));
    assert_eq!(frontend.chosen(), Some("English"));
    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);
    assert!(frontend.machine().is(states::SHOW_LOGO));
}

#[test]
fn the_wheel_walks_the_list_and_stops_at_its_ends() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    reach_the_picker(&mut frontend, &mut input);
    let wheel = |scroll: i32| Pointer {
        scroll,
        ..Pointer::default()
    };
    assert!(frontend.pointer(&wheel(1)));
    assert_eq!(frontend.selected(), 1);
    frontend.pointer(&wheel(7));
    assert_eq!(frontend.selected(), 2);
    frontend.pointer(&wheel(-7));
    assert_eq!(frontend.selected(), 0);
}

/// A click beside the list is the picker's - it is on screen - and does
/// nothing, so a stray tap cannot confirm.
#[test]
fn a_click_off_the_rows_is_handled_and_does_nothing() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    reach_the_picker(&mut frontend, &mut input);
    assert!(frontend.pointer(&click_at((470.0, 260.0))));
    assert_eq!(frontend.selected(), 0);
    assert_eq!(frontend.chosen(), None);
}
