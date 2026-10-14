use super::*;

fn skin(space: oag_display::space::Space) -> Skin {
    Skin::new(oag_pure::FRONT_END.menu.unwrap(), space, 22.0)
}

fn labels() -> Labels {
    let say = |text: &str| text.to_string();
    Labels {
        title: say("WINDOW LAYOUT"),
        hint: say("HOW MANY WINDOWS"),
        windows: say("WINDOWS"),
        window: say("WINDOW"),
        seats: say("SEATS"),
        player: say("PLAYER"),
        cut: say("CUT"),
        horizontal: say("HORIZONTAL"),
        vertical: say("VERTICAL"),
        monitor: say("MONITOR"),
        mode: say("MODE"),
        windowed: say("WINDOWED"),
        fullscreen: say("FULLSCREEN"),
        start: say("START"),
        back: say("BACK"),
    }
}

fn names(count: usize) -> Vec<String> {
    (1..=count).map(|n| format!("Monitor {n}")).collect()
}

fn picker(people: usize, monitors: usize) -> Picker {
    Picker::new(labels(), people, Vec::new(), names(monitors), false)
}

fn seats(picker: &Picker) -> Vec<usize> {
    picker.windows().iter().map(|window| window.seats).collect()
}

fn tick(buttons: &[Button]) -> Input {
    let mut input = Input::new();
    let mask = buttons.iter().fold(0u32, |mask, &b| mask | 1 << b.index());
    input.begin_frame(mask);
    input
}

fn press(picker: &mut Picker, button: Button) -> Vec<Event> {
    picker.update(&mut tick(&[button]))
}

/// Every cell the whole window, which is enough to number seats by.
fn cells(seats: usize, _vertical: bool) -> Vec<[f32; 4]> {
    vec![[0.0, 0.0, 1.0, 1.0]; seats]
}

#[test]
fn one_window_holds_everybody_and_is_where_it_starts() {
    let picker = picker(4, 1);
    assert_eq!(seats(&picker), [4]);
    assert!(!picker.fullscreen());
    assert_eq!(
        picker.rows(),
        [Row::Windows, Row::Mode, Row::Start, Row::Back]
    );
}

/// START and BACK are buttons: no value is drawn beside them. START used to show the head count
/// (the join screen's row, copied), which read as a stray number here.
#[test]
fn a_button_row_shows_no_value() {
    let picker = picker(4, 1);
    assert_eq!(picker.value(Row::Start), "");
    assert_eq!(picker.value(Row::Back), "");
    assert_eq!(picker.value(Row::Windows), "1");
}

#[test]
fn windows_run_from_one_to_the_head_count_and_never_past_eight() {
    assert_eq!(picker(1, 1).most_windows(), 1);
    assert_eq!(picker(3, 1).most_windows(), 3);
    assert_eq!(picker(8, 1).most_windows(), 8);
    let mut picker = picker(4, 1);
    picker.set_window_count(99);
    assert_eq!(picker.windows().len(), 4);
    picker.set_window_count(0);
    assert_eq!(picker.windows().len(), 1);
}

#[test]
fn every_window_keeps_a_seat_and_the_seats_add_up_to_the_people() {
    for people in 1..=8 {
        let mut picker = picker(people, 2);
        for count in 1..=picker.most_windows() {
            picker.set_window_count(count);
            assert_eq!(
                seats(&picker).iter().sum::<usize>(),
                people,
                "{people} over {count}"
            );
            assert!(
                seats(&picker).iter().all(|&n| n >= 1),
                "{people} over {count}"
            );
        }
    }
}

#[test]
fn four_people_can_be_three_and_one() {
    let mut picker = picker(4, 1);
    picker.set_window_count(2);
    assert_eq!(
        seats(&picker),
        [3, 1],
        "a new window takes a seat from the fullest"
    );
    // Window 1 is picked and one seat is moved to it from window 2: 4 and 0 is refused.
    picker.adjust(Row::Seats, 1);
    assert_eq!(seats(&picker), [3, 1], "window 2 keeps its only seat");
    picker.adjust(Row::Seats, -1);
    assert_eq!(seats(&picker), [2, 2]);
    picker.adjust(Row::Seats, 1);
    assert_eq!(seats(&picker), [3, 1]);
}

#[test]
fn the_pad_changes_the_window_count_and_the_seats() {
    let mut picker = picker(4, 1);
    assert_eq!(press(&mut picker, Button::Right), [Event::Changed]);
    assert_eq!(picker.windows().len(), 2);
    press(&mut picker, Button::Down);
    press(&mut picker, Button::Down);
    assert_eq!(picker.selected(), Row::Window);
    press(&mut picker, Button::Down);
    assert_eq!(picker.selected(), Row::Seats);
    press(&mut picker, Button::Right);
    assert_eq!(seats(&picker), [3, 1]);
}

#[test]
fn the_cut_row_is_there_only_for_a_window_of_two_seats() {
    let mut picker = picker(4, 1);
    assert!(!picker.rows().contains(&Row::Cut), "one window of four");
    picker.set_window_count(2);
    assert!(!picker.rows().contains(&Row::Cut), "three and one");
    picker.adjust(Row::Seats, -1);
    assert!(picker.rows().contains(&Row::Cut), "two and two");
    picker.adjust(Row::Window, 1);
    assert!(picker.rows().contains(&Row::Cut), "window 2 holds two too");
    picker.adjust(Row::Seats, 1);
    assert!(!picker.rows().contains(&Row::Cut), "window 2 holds three");
    let mut two = super::Picker::new(labels(), 2, Vec::new(), names(1), false);
    assert!(
        two.rows().contains(&Row::Cut),
        "two people in one window is the split's own cut"
    );
    two.adjust(Row::Cut, 1);
    assert!(two.windows()[0].vertical);
}

#[test]
fn a_monitor_row_is_offered_only_where_there_is_a_choice() {
    assert!(!picker(2, 1).rows().contains(&Row::Monitor));
    let mut two = picker(2, 2);
    assert!(two.rows().contains(&Row::Monitor));
    two.set_window_count(2);
    assert_eq!(
        two.windows().iter().map(|w| w.monitor).collect::<Vec<_>>(),
        [0, 1],
        "a new window lands on the next monitor"
    );
    two.adjust(Row::Monitor, 1);
    assert_eq!(two.windows()[0].monitor, 1);
    two.adjust(Row::Monitor, 1);
    assert_eq!(two.windows()[0].monitor, 0, "the monitors wrap");
}

#[test]
fn a_saved_layout_is_made_to_fit_the_people_and_the_monitors_present() {
    let saved = vec![
        Window {
            seats: 3,
            vertical: true,
            monitor: 5,
        },
        Window {
            seats: 1,
            vertical: false,
            monitor: 1,
        },
    ];
    let same = Picker::new(labels(), 4, saved.clone(), names(2), true);
    assert_eq!(seats(&same), [3, 1]);
    assert_eq!(
        same.windows()[0].monitor,
        1,
        "a monitor that is gone is clamped"
    );
    assert!(same.windows()[0].vertical && same.fullscreen());
    let fewer = Picker::new(labels(), 3, saved.clone(), names(2), false);
    assert_eq!(seats(&fewer), [2, 1], "another head count is dealt anew");
    assert!(fewer.windows()[0].vertical, "and keeps each window's cut");
    let one = Picker::new(labels(), 1, saved, names(2), false);
    assert_eq!(seats(&one), [1], "more windows than people is cut down");
}

#[test]
fn start_and_back_act_from_their_rows_and_from_the_buttons() {
    let mut picker = picker(2, 1);
    assert_eq!(press(&mut picker, Button::Start), [Event::Start]);
    assert_eq!(press(&mut picker, Button::Circle), [Event::Back]);
    while picker.selected() != Row::Start {
        press(&mut picker, Button::Down);
    }
    assert_eq!(press(&mut picker, Button::Cross), [Event::Start]);
    press(&mut picker, Button::Down);
    assert_eq!(press(&mut picker, Button::Cross), [Event::Back]);
}

fn click(at: (f32, f32)) -> Pointer {
    Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        ..Pointer::default()
    }
}

#[test]
fn a_click_on_a_row_steps_its_value_and_the_wheel_changes_it() {
    let skin = skin(oag_display::space::Space::PSP);
    let mut picker = picker(4, 1);
    let geometry = Geometry::new(&skin);
    let at = |index: usize| {
        (
            10.0,
            geometry.layout.top + (index as f32 + 0.5) * geometry.layout.line,
        )
    };
    assert_eq!(picker.pointer(&click(at(0)), &skin), [Event::Changed]);
    assert_eq!(picker.windows().len(), 2);
    let wheel = Pointer {
        at: Some(at(0)),
        scroll: 1,
        ..Pointer::default()
    };
    assert_eq!(picker.pointer(&wheel, &skin), [Event::Changed]);
    assert_eq!(picker.windows().len(), 3);
    let last = picker.rows().len() - 1;
    assert!(
        picker
            .pointer(&click(at(last)), &skin)
            .contains(&Event::Back)
    );
    let start = picker.rows().len() - 2;
    assert!(
        picker
            .pointer(&click(at(start)), &skin)
            .contains(&Event::Start)
    );
    let secondary = Pointer {
        back: true,
        ..Pointer::default()
    };
    assert_eq!(picker.pointer(&secondary, &skin), [Event::Back]);
}

#[test]
fn a_click_on_a_box_in_the_preview_picks_that_window() {
    let skin = skin(oag_display::space::Space::PSP);
    let mut picker = picker(4, 1);
    picker.set_window_count(3);
    let boxes = boxes(
        picker.windows().len(),
        picker.monitors.len() > 1,
        Geometry::new(&skin).preview,
    );
    assert_eq!(boxes.len(), 3);
    let centre = |rect: [f32; 4]| (rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0);
    assert_eq!(
        picker.pointer(&click(centre(boxes[2])), &skin),
        [Event::Moved]
    );
    assert_eq!(picker.edited(), 2);
}

#[test]
fn the_preview_boxes_hold_inside_their_area_and_do_not_overlap() {
    let skin = skin(oag_display::space::Space::PSP);
    for people in 1..=8 {
        let mut picker = picker(people, 2);
        picker.set_window_count(people);
        let area = Geometry::new(&skin).preview;
        let boxes = boxes(picker.windows().len(), picker.monitors.len() > 1, area);
        for (index, rect) in boxes.iter().enumerate() {
            assert!(rect[0] >= area[0] - 0.01 && rect[0] + rect[2] <= area[0] + area[2] + 0.01);
            assert!(rect[1] >= area[1] - 0.01 && rect[1] + rect[3] <= area[1] + area[3] + 0.01);
            for other in &boxes[index + 1..] {
                let apart = rect[0] + rect[2] <= other[0]
                    || other[0] + other[2] <= rect[0]
                    || rect[1] + rect[3] <= other[1]
                    || other[1] + other[3] <= rect[1];
                assert!(apart, "{people} windows: {rect:?} meets {other:?}");
            }
        }
    }
}

#[test]
fn the_preview_numbers_every_seat_once_across_the_windows() {
    let skin = skin(oag_display::space::Space::PSP);
    let frame = Frame::default();
    let mut picker = picker(5, 1);
    picker.set_window_count(2);
    let draws = picker.preview(&skin, &frame, &cells);
    let numbers: Vec<String> = draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(numbers, ["1", "2", "3", "4", "5"]);
}

#[test]
fn every_row_has_words_and_the_screen_draws_in_both_grids() {
    for space in [
        oag_display::space::Space::PSP,
        oag_display::space::Space::HD,
    ] {
        let skin = skin(space);
        let frame = Frame::default();
        let mut picker = picker(8, 3);
        picker.set_window_count(3);
        let body = picker.body(&skin, &frame);
        for row in picker.rows() {
            let label = picker.row_label(row);
            assert!(!label.is_empty());
            assert!(
                body.iter()
                    .any(|draw| matches!(draw, Draw::Text { text, .. } if text == label)),
                "{row:?} is drawn"
            );
        }
        let layers = draw_list(&picker, &skin, &frame, None, false, &cells);
        assert!(!layers.flatten().is_empty());
    }
}
