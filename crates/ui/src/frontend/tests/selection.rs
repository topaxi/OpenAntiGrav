//! How the picker marks its selected row: by ink where the title measured one
//! (the menus' own rule), by a band the pointer also hit-tests where it did not.

use super::*;

use crate::menu::selected_ink;

fn input_down(frontend: &mut Frontend) {
    let mut input = Input::new();
    input.begin_frame(Button::Down.bit());
    frontend.update(FRAME, &mut input, None);
}

fn on_the_picker() -> Frontend {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    input.begin_frame(Button::Start.bit());
    frontend.update(FRAME, &mut input, None);
    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);
    assert!(frontend.machine().is(states::LANGUAGE_SELECTION));
    frontend
}

fn fills_over_the_rows(frontend: &Frontend) -> Vec<[f32; 4]> {
    frontend
        .draw_list()
        .iter()
        .filter_map(|d| match d {
            Draw::Fill { rect, color } if color[3] < 0.9 && rect[1] > 0.0 => Some(*rect),
            _ => None,
        })
        .collect()
}

fn ink_of(frontend: &Frontend, name: &str) -> [f32; 4] {
    frontend
        .draw_list()
        .iter()
        .find_map(|d| match d {
            Draw::Text { text, color, .. } if text == name => Some(*color),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{name} is drawn"))
}

/// The band is the rect the pointer hit-tests: its inside is the selected row
/// and the strip just outside it is not, for every row and with the face's ink
/// measured. It used to start a guessed 4 units above the pen and sat over the
/// row above the text it marked.
#[test]
fn the_band_is_the_rect_the_pointer_hits() {
    let mut frontend = on_the_picker();
    frontend.set_row_ink(Some(crate::pointer::RowInk {
        top: 7.0,
        bottom: 16.0,
    }));
    for row in 0..frontend.languages().len() {
        while frontend.selected() < row {
            input_down(&mut frontend);
        }
        let bands = fills_over_the_rows(&frontend);
        assert_eq!(bands.len(), 1, "one band, row {row}: {bands:?}");
        let [x, y, w, h] = bands[0];
        let inside = (x + w * 0.5, y + h * 0.5);
        assert_eq!(frontend.language_row_at(inside), Some(row), "{bands:?}");
        assert_eq!(
            frontend.language_row_at((x + w * 0.5, y + 0.5)),
            Some(row),
            "top edge"
        );
        assert_ne!(
            frontend.language_row_at((x + w * 0.5, y - 0.5)),
            Some(row),
            "just above"
        );
        assert_ne!(
            frontend.language_row_at((x + w * 0.5, y + h + 0.5)),
            Some(row),
            "just below"
        );
    }
}

/// A title with a measured selected ink draws no band and marks the row with
/// the menus' own `selected_ink`; dropping the wiring leaves the Fill back or
/// the ink equal to the unselected rows'.
#[test]
fn a_title_with_a_measured_ink_marks_the_row_by_ink_and_no_band() {
    let mut frontend = on_the_picker();
    frontend.set_menu_skin(oag_pulse::frontend::MENU_SKIN);
    assert!(fills_over_the_rows(&frontend).is_empty());
    let first = frontend.languages()[0].native_name.clone();
    let second = frontend.languages()[1].native_name.clone();
    let normal = ink_of(&frontend, &second);
    let skin = oag_pulse::frontend::MENU_SKIN;
    // The clock is the screen's own: at 0.55 s the pulse is at its peak.
    let mut input = Input::new();
    input.begin_frame(0);
    frontend.update(0.55, &mut input, None);
    let peak = selected_ink(skin, normal, frontend.on_screen_for as f32);
    let drawn = ink_of(&frontend, &first);
    assert_eq!(drawn, peak);
    assert_ne!(drawn, normal, "the selected row differs from the others");
    assert_eq!(ink_of(&frontend, &second), normal);
}
