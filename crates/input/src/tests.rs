use super::*;
use winit::keyboard::NamedKey;

fn key(name: NamedKey) -> Key {
    Key::Named(name)
}

#[test]
fn a_held_key_becomes_a_held_button() {
    let mut keyboard = Keyboard::new();
    keyboard.set_key(&key(NamedKey::Space), true);
    let snapshot = keyboard.snapshot();
    assert!(snapshot.buttons.is_pressed(Button::Start));
    assert!(snapshot.buttons.is_held(Button::Start));

    let snapshot = keyboard.snapshot();
    assert!(
        !snapshot.buttons.is_pressed(Button::Start),
        "held, not pressed"
    );
    assert!(snapshot.buttons.is_held(Button::Start));
}

#[test]
fn steering_comes_out_of_the_arrow_keys() {
    let mut keyboard = Keyboard::new();
    keyboard.set_key(&key(NamedKey::ArrowRight), true);
    assert_eq!(keyboard.snapshot().stick_x, 1.0);

    keyboard.set_key(&key(NamedKey::ArrowRight), false);
    keyboard.set_key(&key(NamedKey::ArrowLeft), true);
    assert_eq!(keyboard.snapshot().stick_x, -1.0);
}

#[test]
fn opposing_keys_held_together_cancel() {
    let mut keyboard = Keyboard::new();
    keyboard.set_key(&key(NamedKey::ArrowLeft), true);
    keyboard.set_key(&key(NamedKey::ArrowRight), true);
    let snapshot = keyboard.snapshot();
    assert_eq!(snapshot.stick_x, 0.0);
    // The buttons are still both held: only the derived axis cancels, since
    // a menu that binds left and right separately must still see both.
    assert!(snapshot.buttons.is_held(Button::Left));
    assert!(snapshot.buttons.is_held(Button::Right));
}

#[test]
fn the_shoulders_are_the_airbrakes() {
    let mut keyboard = Keyboard::new();
    keyboard.set_key(&Key::Character("q".into()), true);
    let snapshot = keyboard.snapshot();
    assert_eq!(snapshot.airbrake_left, 1.0);
    assert_eq!(snapshot.airbrake_right, 0.0);
}

/// A key held across a focus loss is never seen to come up, so without this
/// the ship keeps turning while the player is in another window.
#[test]
fn releasing_everything_clears_held_keys() {
    let mut keyboard = Keyboard::new();
    keyboard.set_key(&key(NamedKey::ArrowLeft), true);
    assert_eq!(keyboard.snapshot().stick_x, -1.0);

    keyboard.release_all();
    let snapshot = keyboard.snapshot();
    assert_eq!(snapshot.stick_x, 0.0);
    assert!(snapshot.buttons.is_released(Button::Left));
}

/// The merge has to be one `Input`, or a press on one device would be an
/// edge the other device's `consume_press` cannot clear.
#[test]
fn controls_merge_the_keyboard_into_one_button_state() {
    let mut controls = Controls::without_pad();
    controls.set_key(&Key::Named(NamedKey::Space), true);
    let snapshot = controls.snapshot();
    assert!(snapshot.buttons.is_pressed(Button::Start));

    controls.buttons_mut().consume_press(Button::Start);
    assert!(!controls.buttons().is_pressed(Button::Start));
    assert!(controls.buttons().is_held(Button::Start));
}

/// A pad is read fresh every tick, so `Controls` still works as a keyboard
/// on a machine with none - which is every headless capture and CI run.
#[test]
fn controls_steer_from_the_keyboard_with_no_pad_attached() {
    let mut controls = Controls::without_pad();
    controls.set_key(&Key::Character("a".into()), true);
    assert_eq!(controls.snapshot().stick_x, -1.0);

    controls.set_key(&Key::Character("q".into()), true);
    assert_eq!(controls.snapshot().airbrake_left, 1.0);

    controls.release_all();
    let snapshot = controls.snapshot();
    assert_eq!(snapshot.stick_x, 0.0);
    assert_eq!(snapshot.airbrake_left, 0.0);
}

#[test]
fn an_unbound_key_changes_nothing() {
    let mut keyboard = Keyboard::new();
    keyboard.set_key(&key(NamedKey::F1), true);
    assert_eq!(keyboard.snapshot().buttons.held_mask(), 0);
}

/// Every axis a keyboard produces is already in range, so this pins that the
/// sanitiser is on the path rather than that it is needed here.
#[test]
fn every_axis_a_keyboard_produces_is_in_range() {
    let mut keyboard = Keyboard::new();
    for named in [
        NamedKey::ArrowUp,
        NamedKey::ArrowDown,
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
    ] {
        keyboard.set_key(&key(named), true);
    }
    keyboard.set_key(&Key::Character("q".into()), true);
    keyboard.set_key(&Key::Character("e".into()), true);
    let snapshot = keyboard.snapshot();
    for axis in [
        snapshot.stick_x,
        snapshot.stick_y,
        snapshot.airbrake_left,
        snapshot.airbrake_right,
    ] {
        assert!((-1.0..=1.0).contains(&axis), "axis out of range: {axis}");
    }
}

/// Finding U5's guard: a press and release entirely between two reads is
/// still a press.
///
/// Both halves matter - the tap has to *arrive*, and it has to arrive as an
/// edge that goes away again, or a menu confirm would repeat for ever.
#[test]
fn a_tap_between_two_snapshots_is_not_lost() {
    let mut keyboard = Keyboard::new();
    let key = Key::Named(NamedKey::Enter);

    keyboard.set_key(&key, true);
    keyboard.set_key(&key, false);
    assert_eq!(keyboard.held_mask(), 0, "nothing is held any more");

    let first = keyboard.snapshot();
    assert!(
        first.buttons.is_held(Button::Cross),
        "the tap was dropped between the two reads"
    );

    let second = keyboard.snapshot();
    assert!(
        !second.buttons.is_held(Button::Cross),
        "a latched tap must release, or it repeats for ever"
    );
}

/// Focus loss clears the latch too: a tap the player made on the way out
/// belongs to whatever they switched to.
#[test]
fn release_all_drops_a_latched_tap() {
    let mut keyboard = Keyboard::new();
    keyboard.set_key(&Key::Named(NamedKey::Enter), true);
    keyboard.set_key(&Key::Named(NamedKey::Enter), false);
    keyboard.release_all();
    assert!(!keyboard.snapshot().buttons.is_held(Button::Cross));
}

/// The pad half of the merge, which no CI machine can reach through `snapshot`
/// itself. Under [`pad::TriggerMode::Airbrakes`] a trigger contributes its
/// shoulder's *bit* as well as an analog value, so a merge that read the
/// shoulders off the merged mask would answer `1.0` for a trigger at `0.4`.
#[test]
fn a_pads_analog_airbrake_survives_the_merge() {
    let mut controls = Controls::without_pad();
    let snapshot = controls.merge(pad::PadState {
        held: Button::L.bit(),
        airbrake_left: 0.4,
        ..pad::PadState::default()
    });
    assert_eq!(snapshot.airbrake_left, 0.4, "the bit quantised the pull");
    assert!(
        snapshot.buttons.is_pressed(Button::L),
        "and the edge still arrives, or the sideshift cannot be asked for"
    );
}

/// A key is digital, so it wins over any partial pull - the `.max` doing what
/// it always did, now that the two sides can disagree.
#[test]
fn a_held_key_wins_over_a_lighter_pad_pull() {
    let mut controls = Controls::without_pad();
    controls.set_key(&Key::Character("q".into()), true);
    let snapshot = controls.merge(pad::PadState {
        held: Button::L.bit(),
        airbrake_left: 0.4,
        ..pad::PadState::default()
    });
    assert_eq!(snapshot.airbrake_left, 1.0);
}

/// Finding U5's latch reaches the *axes*, not only the buttons: a Q pressed
/// and released between two reads is a full airbrake for its frame. Sourcing
/// the shoulder term from `held_mask()` alone would drop it silently.
#[test]
fn a_tapped_airbrake_key_still_reaches_the_axis() {
    let mut controls = Controls::without_pad();
    let key = Key::Character("q".into());
    controls.set_key(&key, true);
    controls.set_key(&key, false);

    assert_eq!(
        controls.snapshot().airbrake_left,
        1.0,
        "the tap was dropped"
    );
    assert_eq!(
        controls.snapshot().airbrake_left,
        0.0,
        "a latched tap must release, or the airbrake sticks on"
    );
}

/// A pad reading with anything in it says the pad spoke; a tap through
/// the keyboard's latch does not, because that is the mouse's doing.
#[test]
fn the_pad_speaks_only_when_it_contributed() {
    let mut controls = Controls::without_pad();
    controls.tap(Button::Cross);
    controls.merge(pad::PadState::default());
    assert!(!controls.pad_spoke(), "a synthesised tap is not the pad");
    controls.merge(pad::PadState {
        stick_x: 0.3,
        ..pad::PadState::default()
    });
    assert!(controls.pad_spoke());
    controls.merge(pad::PadState {
        held: Button::Cross.bit(),
        ..pad::PadState::default()
    });
    assert!(controls.pad_spoke());
    controls.merge(pad::PadState::default());
    assert!(!controls.pad_spoke());
}

/// A synthesised tap is one press-then-release edge, exactly what a key
/// tapped between two reads produces - and it is gone by the tick after.
#[test]
fn a_synthesised_tap_is_one_press_edge() {
    let mut controls = Controls::without_pad();
    controls.tap(Button::Start);
    controls.snapshot();
    assert!(controls.buttons().is_pressed(Button::Start));
    controls.snapshot();
    assert!(!controls.buttons().is_pressed(Button::Start));
    assert!(!controls.buttons().is_held(Button::Start));
}
