use super::*;
use crate::Controls;
use crate::pad::TriggerMode;
use oag_gameplay::input::Button;

fn pulled(l2: f32, r2: f32) -> ViewRaw {
    ViewRaw {
        l2,
        r2,
        pad: true,
        ..ViewRaw::default()
    }
}

fn controls(mode: TriggerMode) -> Controls {
    let mut controls = Controls::without_pad();
    controls.set_trigger_mode(mode);
    controls
}

/// Under `Airbrakes` a pulled L2 sets L's bit in the snapshot a craft is flown from (a
/// sideshift), and must not be L1 on the screens that cycle craft with it.
#[test]
fn l2_is_l1_to_the_race_and_not_to_the_view_in_airbrake_mode() {
    let mut controls = controls(TriggerMode::Airbrakes);
    controls.inject_pad(pulled(0.8, 0.0));
    let flying = controls.player_snapshots();
    assert!(
        flying.get(0).buttons.is_pressed(Button::L),
        "the race still sees the trigger as L: that is the airbrake's sideshift"
    );
    assert!(!controls.view_held(Button::L), "the view does not");
    assert!(!controls.view_take(Button::L));
    assert!(controls.view().l2 > 0.8, "and reads it as travel instead");
}

/// `ThrustBrake` makes R2 the CROSS: a viewer would toggle play and photo mode would take a shot
/// on every scrub.
#[test]
fn r2_is_cross_to_the_race_and_not_to_the_view_in_thrust_brake_mode() {
    let mut controls = controls(TriggerMode::ThrustBrake);
    controls.inject_pad(pulled(0.0, 0.9));
    let flying = controls.player_snapshots();
    assert!(flying.get(0).buttons.is_pressed(Button::Cross));
    assert!(!controls.view_take(Button::Cross));
    assert!(controls.view().r2 > 0.9);
}

/// The shoulder itself still reaches the view, alone and beside a pulled trigger.
#[test]
fn the_shoulder_buttons_reach_the_view_and_a_trigger_does_not_hide_them() {
    let mut controls = controls(TriggerMode::Airbrakes);
    controls.press_button(Button::L, true);
    controls.inject_pad(pulled(0.9, 0.0));
    controls.player_snapshots();
    assert!(controls.view_take(Button::L), "L1 with L2 pulled is L1");
    assert!(!controls.view_take(Button::L), "taken once");
}

/// What a view-side read must never do: reach the snapshot. The same pad with and without a
/// right stick makes the same snapshot, bit for bit.
#[test]
fn the_right_stick_is_not_in_the_snapshot() {
    let flown = |right: (f32, f32)| {
        let mut controls = controls(TriggerMode::Airbrakes);
        controls.inject_pad(ViewRaw {
            right,
            left: (0.5, -0.25),
            pad: true,
            ..ViewRaw::default()
        });
        let inputs = controls.player_snapshots();
        let seen = *inputs.get(0);
        (seen, controls.view().right)
    };
    let (still, none) = flown((0.0, 0.0));
    let (pushed, seen) = flown((1.0, -1.0));
    assert_eq!(still, pushed, "the snapshot a craft is flown from moved");
    assert_eq!(none, (0.0, 0.0));
    assert_eq!(seen, (1.0, -1.0), "and the view read it");
}

#[test]
fn a_trigger_is_conditioned_and_a_stick_deadzoned() {
    let read = ViewRaw {
        right: (0.05, 0.6),
        l2: 0.05,
        r2: 1.0,
        ..ViewRaw::default()
    }
    .read(crate::pad::TriggerConfig::default());
    assert_eq!(read.right.0, 0.0, "inside the deadzone");
    assert!(read.right.1 > 0.0 && read.right.1 < 0.6);
    assert_eq!(read.l2, 0.0, "a resting trigger that reads 0.05");
    assert_eq!(read.r2, 1.0);
    assert!(read.r2 >= PLAY_PULL);
}
