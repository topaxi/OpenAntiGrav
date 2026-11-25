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
        passes: &[],
        reflection: None,
        chains: Chains::default(),
    });
    assert!(line.starts_with("frame-log stage=menu:window-layout fps=59.9"));
    assert!(line.contains("gpu_ms=- "));
    assert!(line.contains(
        "window=3840x2160 surface=3840x2160 render=- governor=0 pass_ms=- reflection=- \
             bloom_ms=- blur_ms=- upscale_ms=- upscale_presented_ms=-"
    ));
}

/// Every timed pass is a `name:ms` pair in one space-free field appended after `governor`, in
/// the overlay's order and with the rows the overlay hides, so a script that splits the line on
/// spaces and reads the fields before it is unaffected; the glass and sea target's step comes
/// after it.
#[test]
fn the_pass_field_lists_every_timed_pass_after_the_old_fields() {
    let line = format_line(&Reading {
        stage: "race",
        fps: 60.0,
        frame_ms: 16.7,
        p99_ms: 17.0,
        cpu_ms: Some(2.0),
        gpu_ms: Some(4.0),
        present_ms: Some(0.5),
        target_hz: 60,
        window: (1280, 720),
        surface: (1280, 720),
        render: Some((1280, 720)),
        governor: 0,
        passes: &[("SHADOW MAP", 1.234), ("HUD AND UI", 0.05)],
        reflection: Some("quarter"),
        chains: Chains::default(),
    });
    assert!(
        line.contains(" governor=0 pass_ms=shadow_map:1.23,hud_and_ui:0.05 reflection=quarter "),
        "{line}"
    );
    assert!(line.contains(" gpu_ms=4.00 "), "{line}");
}

/// The chains timed apart from the scene pass (HD/Fury's bloom, the motion blur, FSR 3's two
/// halves) are appended after `reflection`, so every field a script read before keeps its place.
#[test]
fn the_chains_come_last_and_leave_the_old_fields_in_place() {
    let line = format_line(&Reading {
        stage: "race",
        fps: 30.0,
        frame_ms: 33.3,
        p99_ms: 40.0,
        cpu_ms: Some(20.0),
        gpu_ms: Some(9.5),
        present_ms: Some(13.0),
        target_hz: 240,
        window: (2320, 1080),
        surface: (2320, 1080),
        render: Some((476, 270)),
        governor: 0,
        passes: &[("GLASS", 1.5)],
        reflection: Some("quarter"),
        chains: Chains {
            bloom_ms: Some(2.345),
            blur_ms: None,
            upscale_ms: Some(0.5),
            upscale_presented_ms: Some(0.25),
        },
    });
    assert!(
        line.ends_with(
            " pass_ms=glass:1.50 reflection=quarter bloom_ms=2.35 blur_ms=- upscale_ms=0.50 \
             upscale_presented_ms=0.25"
        ),
        "{line}"
    );
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
