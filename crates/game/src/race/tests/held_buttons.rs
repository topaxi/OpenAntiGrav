//! `HeldButtons` and the keyboard map behind it: the button path a headless
//! run drives, and the controls it derives.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the 200-line
//! rule in `scripts/check-file-size.py`. Shared fixtures live in the parent
//! `tests.rs`.

use super::*;
use oag_gameplay::input::Button;
use oag_input::keys::map_key;

/// `HeldButtons::set_held` drives the keyboard path with per-tick levels:
/// the axes derive exactly as a real player's would, and releasing shows
/// up on the very next snapshot - what `--input-script` needs.
#[test]
fn set_held_levels_reach_the_snapshot_and_release() {
    let mut held = HeldButtons::new(0);
    held.set_held(Button::Cross.bit() | Button::Left.bit());
    let snap = held.snapshot();
    assert!(snap.buttons.is_held(Button::Cross));
    assert!(snap.stick_x < 0.0, "left must steer negative x");
    held.set_held(0);
    let snap = held.snapshot();
    assert!(!snap.buttons.is_held(Button::Cross));
    assert_eq!(snap.stick_x, 0.0);
}

/// The inverse table must really be the inverse, or the headless capture and the
/// window disagree about what a key means and only one of them is ever tested.
#[test]
fn every_key_in_the_inverse_table_maps_back_to_its_button() {
    for index in 0..32u8 {
        let Some(key) = key_for_button(Button::from_index(index)) else {
            continue;
        };
        assert_eq!(
            map_key(&key).map(|b| b.index()),
            Some(index),
            "button {index} maps to {key:?}, which maps back to something else"
        );
    }
}

/// The buttons a race needs must all be reachable with no window.
#[test]
fn thrust_steering_and_both_airbrakes_all_have_a_key() {
    for wanted in [
        Button::Cross,
        Button::Left,
        Button::Right,
        Button::Up,
        Button::Down,
        Button::L,
        Button::R,
    ] {
        assert!(key_for_button(wanted).is_some(), "no key for {wanted}");
    }
}

/// A pulsed button produces a *press* every other tick, a held one does not.
///
/// The whole reason `pulse` exists: the veteran sideshift reads
/// `Input::is_pressed`, and `--press` was reaching only the front end, so
/// the manoeuvre could not be exercised in the mode a player uses. If this
/// ever reports one edge and then silence, `--press l` has quietly become a
/// hold again.
#[test]
fn a_pulsed_button_keeps_producing_edges_and_a_held_one_does_not() {
    let mut pulsed = HeldButtons::new(0);
    let mut held = HeldButtons::new(Button::L.bit());
    let (mut pulsed_edges, mut held_edges) = (0, 0);

    for tick in 0..8u32 {
        pulsed.pulse(Button::L.bit(), 0, tick.is_multiple_of(2));
        if pulsed.snapshot().buttons.is_pressed(Button::L) {
            pulsed_edges += 1;
        }
        if held.snapshot().buttons.is_pressed(Button::L) {
            held_edges += 1;
        }
    }

    assert_eq!(pulsed_edges, 4, "one rising edge every other tick");
    assert_eq!(held_edges, 1, "a hold rises once and never again");
}

/// Holding and pulsing the same button resolves toward held.
///
/// Otherwise `--hold q --press q` would silently drop the hold on every odd
/// tick, which reads as an airbrake that stutters for no visible reason.
#[test]
fn pulsing_a_button_that_is_also_held_leaves_it_down() {
    let mut buttons = HeldButtons::new(Button::L.bit());
    buttons.pulse(Button::L.bit(), Button::L.bit(), false);
    assert!(buttons.snapshot().buttons.is_held(Button::L));
}

#[test]
fn a_held_cross_becomes_thrust() {
    let mut held = HeldButtons::new(Button::Cross.bit());
    let controls = ship_controls(&held.snapshot(), ControlScheme::default());
    assert_eq!(controls.thrust, 1.0);
    assert_eq!(controls.steer_x, 0.0);
}

#[test]
fn holding_left_steers_left() {
    let mut held = HeldButtons::new(Button::Left.bit());
    assert_eq!(
        ship_controls(&held.snapshot(), ControlScheme::default()).steer_x,
        -1.0
    );
}

/// A mask naming a button no key produces must not panic, and must not leak into
/// the controls either.
#[test]
fn a_button_with_no_key_is_ignored() {
    let mut held = HeldButtons::new(Button::Start.bit());
    assert_eq!(
        ship_controls(&held.snapshot(), ControlScheme::default()),
        oag_physics::ShipControls::default()
    );
}
