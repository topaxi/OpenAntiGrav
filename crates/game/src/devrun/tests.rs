use super::*;

#[test]
fn placeholders_seat_the_keyboard_first_and_never_collide_with_a_real_id() {
    let seats = placeholder_devices(3);
    assert_eq!(seats[0], Device::Keyboard);
    assert!(matches!(seats[1], Device::Keys(KeyboardId(id)) if id >= PLACEHOLDER_BASE));
    assert_ne!(seats[1], seats[2]);
    assert!(placeholder_devices(0).is_empty());
}

#[test]
fn a_script_hands_each_step_out_once_in_time_order() {
    let mut script = Script::parse("4:right, 1:down,10:quit").unwrap();
    assert!(script.due(0.5, None).is_empty());
    assert_eq!(script.due(1.0, None), vec![Step::Tap(Button::Down)]);
    assert!(script.due(3.0, None).is_empty());
    assert_eq!(
        script.due(99.0, None),
        vec![Step::Tap(Button::Right), Step::Quit]
    );
    assert!(script.due(100.0, None).is_empty());
}

#[test]
fn a_plus_entry_waits_for_the_anchor_and_counts_from_it() {
    let mut script = Script::parse("2:start,+1:down,+4:quit").unwrap();
    assert_eq!(script.due(50.0, None), vec![Step::Tap(Button::Start)]);
    assert!(script.due(51.0, None).is_empty());
    assert!(script.due(60.0, Some(0.5)).is_empty());
    assert_eq!(script.due(61.0, Some(1.0)), vec![Step::Tap(Button::Down)]);
    assert_eq!(script.due(65.0, Some(4.0)), vec![Step::Quit]);
}

#[test]
fn a_bad_script_names_the_entry() {
    assert!(Script::parse("3").unwrap_err().contains("SECONDS:BUTTON"));
    assert!(Script::parse("x:down").unwrap_err().contains("seconds"));
    assert!(Script::parse("-1:down").is_err());
    assert!(Script::parse("1:jump").unwrap_err().contains("jump"));
    assert!(Script::parse("").unwrap().is_empty());
}

#[test]
fn the_line_carries_every_field_and_dashes_what_was_not_read() {
    let line = format_line(&Reading {
        stage: "menu:window-layout",
        fps: 59.9,
        frame_ms: 16.7,
        p99_ms: 20.0,
        cpu_ms: Some(2.0),
        gpu_ms: None,
        present_ms: Some(0.5),
        target_hz: 60,
        window: (3840, 2160),
        surface: (3840, 2160),
        render: None,
        governor: 0,
    });
    assert!(line.starts_with("frame-log stage=menu:window-layout fps=59.9"));
    assert!(line.contains("gpu_ms=- "));
    assert!(line.contains("window=3840x2160 surface=3840x2160 render=- governor=0"));
}

#[test]
fn a_plain_tap_not_yet_due_is_dropped_once_the_anchor_lands() {
    let mut script = Script::parse("9:cross,+1:down").unwrap();
    assert!(script.due(2.0, Some(0.0)).is_empty());
    assert_eq!(script.due(10.0, Some(1.0)), vec![Step::Tap(Button::Down)]);
}

#[test]
fn a_seat_tap_names_its_seat() {
    let mut script = Script::parse("1:cross@2").unwrap();
    assert_eq!(script.due(1.0, None), vec![Step::SeatTap(2, Button::Cross)]);
    assert!(Script::parse("1:cross@x").unwrap_err().contains("seat"));
}
