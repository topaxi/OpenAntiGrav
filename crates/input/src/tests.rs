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
    let snapshot = controls.merge_one(pad::PadState {
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
    let snapshot = controls.merge_one(pad::PadState {
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
    controls.merge_one(pad::PadState::default());
    assert!(!controls.pad_spoke(), "a synthesised tap is not the pad");
    controls.merge_one(pad::PadState {
        stick_x: 0.3,
        ..pad::PadState::default()
    });
    assert!(controls.pad_spoke());
    controls.merge_one(pad::PadState {
        held: Button::Cross.bit(),
        ..pad::PadState::default()
    });
    assert!(controls.pad_spoke());
    controls.merge_one(pad::PadState::default());
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

/// The claim the whole per-slot change rests on: under the default assignment
/// slot 0's snapshot is what the single-snapshot path produced, and the other
/// seven are empty. If this ever fails, a single-player race has changed.
#[test]
fn the_default_assignment_puts_everything_in_slot_zero() {
    let mut controls = Controls::without_pad();
    controls.set_key(&Key::Character("a".into()), true);
    controls.set_key(&Key::Character("q".into()), true);

    let inputs = controls.player_snapshots();
    let slot_zero = inputs.get(pad::DEFAULT_SLOT);
    assert_eq!(slot_zero.stick_x, -1.0);
    assert_eq!(slot_zero.airbrake_left, 1.0);
    assert!(slot_zero.buttons.is_held(Button::Left));

    for slot in 1..oag_gameplay::MAX_PLAYERS {
        assert_eq!(
            *inputs.get(slot),
            InputSnapshot::default(),
            "slot {slot} took input nobody assigned to it"
        );
    }
}

/// A second slot's edges are its own: `pressed = held & !held_last` computed
/// against one shared `Input` would let one player's release clear another
/// player's unconsumed press.
#[test]
fn a_second_slot_keeps_its_own_button_edges() {
    let mut controls = Controls::without_pad();
    controls.assignment_mut().assign_keyboard(1);
    assert_eq!(controls.keyboard_slot(), 1);

    controls.set_key(&Key::Named(NamedKey::Space), true);
    let inputs = controls.player_snapshots();
    assert!(inputs.get(1).buttons.is_pressed(Button::Start));
    assert!(
        !inputs.get(0).buttons.is_held(Button::Start),
        "slot 0 saw a key the keyboard no longer drives"
    );

    // And a second read is held rather than pressed, on slot 1's own history.
    let inputs = controls.player_snapshots();
    assert!(!inputs.get(1).buttons.is_pressed(Button::Start));
    assert!(inputs.get(1).buttons.is_held(Button::Start));
}

/// A split race's pause: a press on a slot other than the keyboard's (a seated
/// pad's, stood in for here by a press made on slot 1 before the keyboard moves
/// back to slot 0) is invisible to `buttons()` and reported by `take_presses`
/// as slot 1's, which consumes it so the next frame does not toggle again.
#[test]
fn take_presses_names_the_slot_the_keyboard_does_not_drive() {
    let mut controls = Controls::without_pad();
    controls.assignment_mut().assign_keyboard(1);
    controls.set_key(&Key::Named(NamedKey::Space), true);
    controls.player_snapshots();
    controls.assignment_mut().assign_keyboard(0);

    assert!(
        !controls.buttons().is_pressed(Button::Start),
        "the keyboard's slot never saw slot 1's Start"
    );
    assert_eq!(controls.take_presses(Button::Start), 0b10);
    assert_eq!(controls.take_presses(Button::Start), 0, "consumed once");
}

#[test]
fn an_android_pad_button_is_a_pad_press_and_hides_nothing_it_should_not() {
    let mut controls = Controls::without_pad();
    assert!(controls.android_key(android::BUTTON_A, true));
    let snapshot = controls.snapshot();
    assert!(snapshot.buttons.is_held(Button::Cross));
    assert!(controls.pad_spoke(), "an Android pad is the pad in use");
    assert!(controls.android_pad_seen());
    controls.android_key(android::BUTTON_A, false);
    assert!(!controls.snapshot().buttons.is_held(Button::Cross));
}

#[test]
fn an_android_pad_press_shorter_than_a_tick_still_lands() {
    let mut controls = Controls::without_pad();
    controls.android_key(android::BUTTON_B, true);
    controls.android_key(android::BUTTON_B, false);
    assert!(controls.snapshot().buttons.is_pressed(Button::Circle));
}

#[test]
fn an_android_stick_steers_and_pushes_up_positive() {
    let mut controls = Controls::without_pad();
    controls.android_stick(1.0, -1.0);
    let snapshot = controls.snapshot();
    assert_eq!(snapshot.stick_x, 1.0);
    assert_eq!(snapshot.stick_y, 1.0);
}

#[test]
fn a_non_pad_android_key_is_not_spent() {
    let mut controls = Controls::without_pad();
    assert!(!controls.android_key(19, true), "DPAD_UP is the keyboard's");
    assert!(!controls.pad_spoke());
}

#[test]
fn the_touch_overlay_drives_a_ship_but_is_not_a_pad() {
    let mut controls = Controls::without_pad();
    controls.set_touch(Reading {
        buttons: Button::Cross.bit() | Button::L.bit(),
        stick_x: -0.8,
        ..Reading::default()
    });
    let snapshot = controls.snapshot();
    assert!(snapshot.buttons.is_held(Button::Cross));
    assert_eq!(snapshot.airbrake_left, 1.0);
    assert!(snapshot.stick_x < -0.5);
    assert!(!controls.pad_spoke(), "a finger is not a pad");
    controls.set_touch(Reading::default());
    assert!(!controls.snapshot().buttons.is_held(Button::Cross));
}

#[test]
fn a_partial_touch_airbrake_reaches_the_snapshot_as_an_axis() {
    let mut controls = Controls::without_pad();
    controls.set_touch(Reading {
        buttons: Button::Cross.bit(),
        airbrake_right: 0.4,
        ..Reading::default()
    });
    let snapshot = controls.snapshot();
    assert!((snapshot.airbrake_right - 0.4).abs() < 1e-6);
    assert_eq!(snapshot.airbrake_left, 0.0);
    assert!(snapshot.buttons.is_held(Button::Cross));
}

#[test]
fn release_all_lets_go_of_an_android_pad_and_the_overlay() {
    let mut controls = Controls::without_pad();
    controls.android_key(android::BUTTON_A, true);
    controls.set_touch(Reading {
        buttons: Button::Square.bit(),
        ..Reading::default()
    });
    controls.release_all();
    let snapshot = controls.snapshot();
    assert!(!snapshot.buttons.is_held(Button::Cross));
    assert!(!snapshot.buttons.is_held(Button::Square));
}

#[test]
fn an_android_press_shorter_than_a_tick_still_counts_as_the_pad_speaking() {
    let mut controls = Controls::without_pad();
    controls.android_key(android::BUTTON_A, true);
    controls.android_key(android::BUTTON_A, false);
    controls.snapshot();
    assert!(controls.pad_spoke());
    controls.snapshot();
    assert!(!controls.pad_spoke(), "and only for the tick it landed in");
}

/// What the join screen reads: the keyboard's newly pressed keys are a press by
/// the keyboard device, once, and a held key is not pressed again.
#[test]
fn a_key_going_down_is_one_press_by_the_keyboard() {
    let mut controls = Controls::without_pad();
    controls.set_key(&Key::Named(NamedKey::Space), true);
    controls.player_snapshots();
    assert_eq!(
        controls.device_presses(),
        &[lobby::Press {
            device: lobby::Device::Keyboard,
            buttons: Button::Start.bit(),
        }]
    );
    controls.player_snapshots();
    assert!(
        controls.device_presses().is_empty(),
        "a held key is not pressed twice"
    );
}

/// A click reaches the game as a tap, which is the keyboard speaking: it must be
/// a press even when the key is up again by the time the tick reads it.
#[test]
fn a_tap_shorter_than_a_tick_is_a_press() {
    let mut controls = Controls::without_pad();
    controls.tap(Button::Cross);
    controls.player_snapshots();
    assert_eq!(controls.device_presses().len(), 1);
    assert_eq!(controls.device_presses()[0].buttons, Button::Cross.bit());
}

/// The point of the lane at the boundary a race reads: with two people seated, a
/// reading on pad 2's slot steers slot 1's snapshot and leaves slot 0's alone,
/// and the keyboard still steers slot 0.
#[test]
fn a_second_pads_stick_reaches_the_second_slots_snapshot() {
    let mut controls = Controls::without_pad();
    controls.seat_default(2);
    controls.set_key(&Key::Named(NamedKey::ArrowLeft), true);

    let mut pads = [pad::PadState::default(); oag_gameplay::MAX_PLAYERS];
    pads[1] = pad::PadState {
        stick_x: 0.8,
        held: Button::Cross.bit(),
        ..pad::PadState::default()
    };
    let inputs = controls.merge_players(pads);
    assert_eq!(inputs.get(0).stick_x, -1.0, "the keyboard drives slot 0");
    assert_eq!(inputs.get(1).stick_x, 0.8);
    assert!(inputs.get(1).buttons.is_held(Button::Cross));
    assert!(!inputs.get(0).buttons.is_held(Button::Cross));
}

/// Seating a lobby that holds only the keyboard seats one person, and merging
/// afterwards undoes any seating: the menus have one cursor and every device
/// drives it. (A lobby with a pad in it needs a `GamepadId`, which no test can
/// make; `lobby`'s own tests cover those over a stand-in id.)
#[test]
fn merging_undoes_a_seating() {
    let mut lobby = lobby::Lobby::new();
    lobby.join(lobby::Device::Keyboard);
    let mut controls = Controls::without_pad();
    lobby.seat(controls.assignment_mut(), &[]);
    assert_eq!(controls.assignment().humans(), Some(1));
    assert_eq!(controls.keyboard_slot(), 0);

    controls.seat_default(3);
    assert_eq!(controls.assignment().humans(), Some(3));
    controls.merge_devices();
    assert_eq!(controls.assignment().humans(), None);
    assert!(controls.assignment().is_single_player());
}

/// The focus moving between windows moves the keyboard to the other window's seat, and
/// what was held on the seat it left is released, not carried: nothing is held or
/// freshly pressed on the seat it arrives at, and the seat it left sees the key come up.
#[test]
fn moving_the_keyboard_releases_what_was_held_on_the_seat_it_left() {
    let mut controls = Controls::without_pad();
    controls.seat_default(2);
    controls.set_key(&key(NamedKey::ArrowUp), true);
    controls.set_key(&key(NamedKey::Space), true);
    let held = controls.player_snapshots();
    assert_eq!(held.get(0).stick_y, 1.0);
    assert!(held.get(0).buttons.is_held(Button::Start));

    controls.set_key(&key(NamedKey::Enter), true);
    controls.keyboard_to(1);
    assert_eq!(controls.keyboard_slot(), 1);
    let moved = controls.player_snapshots();
    assert_eq!(moved.get(0).stick_y, 0.0, "the seat it left lets go");
    assert!(!moved.get(0).buttons.is_held(Button::Start));
    assert_eq!(moved.get(1).stick_y, 0.0, "nothing carries to the new seat");
    assert!(!moved.get(1).buttons.is_held(Button::Start));
    assert!(!moved.get(1).buttons.is_pressed(Button::Start), "and no press edge");
    assert!(!moved.get(1).buttons.is_pressed(Button::Cross), "a tap latched before the move is dropped");

    controls.set_key(&key(NamedKey::ArrowUp), true);
    let steered = controls.player_snapshots();
    assert_eq!(steered.get(1).stick_y, 1.0, "a key pressed after the move drives the new seat");
    assert_eq!(steered.get(0).stick_y, 0.0, "and not the old one");
}

/// Moving onto the seat it is on, or off the grid, changes nothing - the keys held stay held.
#[test]
fn moving_the_keyboard_to_where_it_is_keeps_its_keys() {
    let mut controls = Controls::without_pad();
    controls.seat_default(2);
    controls.set_key(&key(NamedKey::ArrowUp), true);
    controls.keyboard_to(0);
    controls.keyboard_to(oag_gameplay::MAX_PLAYERS);
    assert_eq!(controls.keyboard_slot(), 0);
    assert_eq!(controls.player_snapshots().get(0).stick_y, 1.0);
}
