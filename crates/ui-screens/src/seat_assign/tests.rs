use super::*;
use oag_ui::menu::Frame;

fn labels() -> Labels {
    Labels {
        title: "WHO SITS WHERE".into(),
        hint: "MOVE".into(),
        player: "PLAYER".into(),
        window: "WINDOW".into(),
        view: "VIEW".into(),
        auto: "AUTO".into(),
        start: "START".into(),
        back: "BACK".into(),
    }
}

fn skin() -> Skin {
    Skin::new(
        oag_pure::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    )
}

/// Three people over two windows: a two-pane window then a one-pane window.
fn three() -> SeatAssign {
    SeatAssign::new(
        labels(),
        vec![
            Pane {
                seats: 2,
                vertical: false,
            },
            Pane {
                seats: 1,
                vertical: false,
            },
        ],
        None,
    )
}

fn cells(seats: usize, vertical: bool) -> Vec<[f32; 4]> {
    match (seats, vertical) {
        (1, _) => vec![[0.0, 0.0, 1.0, 1.0]],
        (2, true) => vec![[0.0, 0.0, 0.5, 1.0], [0.5, 0.0, 0.5, 1.0]],
        _ => vec![[0.0, 0.0, 1.0, 0.5], [0.0, 0.5, 1.0, 0.5]],
    }
}

fn press(button: Button) -> u32 {
    button.bit()
}

#[test]
fn a_fresh_screen_keeps_the_join_order_and_everybody_confirmed() {
    let screen = three();
    assert_eq!(screen.pane_people(), [0, 1, 2]);
    assert!(screen.can_start());
}

#[test]
fn confirm_on_a_fresh_screen_goes_on() {
    let mut screen = three();
    assert_eq!(screen.press(1, press(Button::Cross)), [Event::Start]);
}

#[test]
fn moving_onto_a_held_pane_swaps_and_both_confirm_again() {
    let mut screen = three();
    // Person 2 steps back from pane 2 (window 2) onto pane 1: the one they swap with is person 1.
    let events = screen.press(2, press(Button::Left));
    assert_eq!(events, [Event::Changed]);
    assert_eq!(screen.pane_people(), [0, 2, 1]);
    assert!(!screen.is_ready(2) && !screen.is_ready(1));
    assert!(screen.is_ready(0));
    assert!(!screen.can_start());
}

#[test]
fn start_waits_for_everybody_to_confirm_again() {
    let mut screen = three();
    screen.press(2, press(Button::Left));
    assert_eq!(
        screen.press(2, press(Button::Cross)),
        [Event::Changed, Event::Refused]
    );
    assert_eq!(
        screen.press(1, press(Button::Cross)),
        [Event::Changed, Event::Start]
    );
}

#[test]
fn every_person_is_always_in_exactly_one_pane() {
    let mut screen = three();
    for step in 0..40 {
        let person = step % 3;
        let button = [Button::Left, Button::Right, Button::Up, Button::Down][(step / 3) % 4];
        screen.press(person, press(button));
        let mut panes = screen.pane_people();
        panes.sort_unstable();
        assert_eq!(panes, [0, 1, 2], "after step {step}");
    }
}

#[test]
fn the_steps_wrap_round_the_panes() {
    let mut screen = three();
    screen.press(0, press(Button::Left));
    assert_eq!(screen.pane_of(0), 2);
    screen.press(0, press(Button::Right));
    assert_eq!(screen.pane_of(0), 0);
}

#[test]
fn circle_goes_back_and_a_stranger_does_nothing() {
    let mut screen = three();
    assert_eq!(screen.press(0, press(Button::Circle)), [Event::Back]);
    assert!(screen.press(7, press(Button::Cross)).is_empty());
}

#[test]
fn the_last_assignment_is_used_when_it_is_a_permutation_and_ignored_otherwise() {
    let windows = three().windows().to_vec();
    let remembered = SeatAssign::new(labels(), windows.clone(), Some(&[0, 2, 1]));
    assert_eq!(remembered.pane_people(), [0, 2, 1]);
    assert_eq!(remembered.pane_of(2), 1);
    assert!(
        remembered.can_start(),
        "a remembered assignment opens confirmed"
    );
    for bad in [&[0, 0, 1][..], &[0, 1][..], &[0, 1, 5][..]] {
        let screen = SeatAssign::new(labels(), windows.clone(), Some(bad));
        assert_eq!(screen.pane_people(), [0, 1, 2], "{bad:?}");
    }
}

#[test]
fn auto_puts_everybody_back_in_join_order_confirmed() {
    let mut screen = three();
    screen.press(2, press(Button::Left));
    screen.reset();
    assert_eq!(screen.pane_people(), [0, 1, 2]);
    assert!(screen.can_start());
}

#[test]
fn the_rows_are_the_people_then_auto_start_and_back() {
    assert_eq!(
        three().rows(),
        [
            Row::Person(0),
            Row::Person(1),
            Row::Person(2),
            Row::Auto,
            Row::Start,
            Row::Back
        ]
    );
}

#[test]
fn a_row_says_the_window_and_the_pane_in_it() {
    let mut screen = three();
    assert_eq!(screen.place(0), "WINDOW 1 VIEW 1");
    assert_eq!(screen.place(1), "WINDOW 1 VIEW 2");
    assert_eq!(screen.place(2), "WINDOW 2");
    screen.press(2, press(Button::Left));
    assert_eq!(screen.place(2), "WINDOW 1 VIEW 2");
    assert_eq!(screen.place(1), "WINDOW 2");
}

fn click(at: (f32, f32)) -> Pointer {
    Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        ..Pointer::default()
    }
}

/// The centre of flat pane `pane` of `screen` on `skin`.
fn pane_centre(screen: &SeatAssign, skin: &Skin, pane: usize) -> (f32, f32) {
    let geometry = Geometry::new(skin);
    let rects = boxes(screen.windows().len(), false, geometry.preview);
    let mut first = 0;
    for (window, rect) in screen.windows().iter().zip(rects) {
        let panes = cell_rects(rect, window.seats, window.vertical, &cells);
        if pane < first + window.seats {
            let cell = panes[pane - first];
            return (cell[0] + cell[2] / 2.0, cell[1] + cell[3] / 2.0);
        }
        first += window.seats;
    }
    panic!("no pane {pane}");
}

#[test]
fn a_click_on_a_pane_picks_up_its_person_and_a_second_puts_them_down() {
    let skin = skin();
    let mut screen = three();
    let picked = screen.pointer(&click(pane_centre(&screen, &skin, 2)), &skin, &cells);
    assert!(picked.contains(&Event::Moved));
    assert_eq!(screen.held(), Some(2));
    let placed = screen.pointer(&click(pane_centre(&screen, &skin, 0)), &skin, &cells);
    assert!(placed.contains(&Event::Changed), "{placed:?}");
    assert_eq!(screen.pane_people(), [2, 1, 0]);
    assert_eq!(screen.held(), None);
    assert!(screen.can_start(), "a click is the confirmation of both");
}

#[test]
fn a_click_outside_any_pane_or_row_does_nothing() {
    let skin = skin();
    let mut screen = three();
    let (width, _) = skin.space().size;
    let events = screen.pointer(&click((width - 1.0, 1.0)), &skin, &cells);
    assert!(!events.contains(&Event::Changed));
    assert_eq!(screen.held(), None);
}

#[test]
fn the_draw_list_has_a_cell_per_person_and_the_numbers_in_pane_order() {
    let mut screen = three();
    screen.press(2, press(Button::Left));
    let skin = skin();
    let numbers: Vec<String> = screen
        .preview(&skin, &Frame::default(), &cells)
        .into_iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(numbers, ["1", "3", "2"], "pane 2 now shows person 3");
}

#[test]
fn start_and_back_are_buttons_with_no_value() {
    let skin = skin();
    let rights: Vec<String> = three()
        .body(&skin, &Frame::default())
        .into_iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, align, .. } if align == Align::Right => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(rights.len(), 3, "{rights:?}");
}

#[test]
fn the_pointer_start_refuses_while_somebody_has_not_confirmed() {
    let skin = skin();
    let mut screen = three();
    screen.press(2, press(Button::Left));
    let geometry = Geometry::new(&skin);
    let start = screen
        .rows()
        .iter()
        .position(|row| *row == Row::Start)
        .unwrap();
    let at = (
        geometry.column / 2.0,
        geometry.layout.top + (start as f32 + 0.5) * geometry.layout.line,
    );
    assert_eq!(
        screen.pointer(&click(at), &skin, &cells),
        [Event::Moved, Event::Refused]
    );
}
