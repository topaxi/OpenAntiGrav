//! What [`super::add`] is asserted to do.

use super::*;

/// The plain case: a hit under the ceiling is worth its full `slowdown_time`.
#[test]
fn a_hit_under_the_ceiling_adds_all_of_itself() {
    let mut state = ShipState::default();
    add(&mut state, 0.75, 2.0);
    assert_eq!(state.slowdown_timer, 0.75);
}

/// The ceiling is on seconds *outstanding*, so two hits inside the window are
/// worth the cap and not their sum.
#[test]
fn two_hits_inside_the_window_do_not_stack_past_the_ceiling() {
    let mut state = ShipState::default();
    add(&mut state, 1.5, 2.0);
    add(&mut state, 1.5, 2.0);
    assert_eq!(state.slowdown_timer, 2.0, "clamped to the authored limit");
}

/// A weapon whose own `slowdown_time` equals the cap saturates on one hit -
/// which is what the Plasma authors on both shipped tables.
#[test]
fn a_weapon_authored_at_the_cap_saturates_on_one_hit() {
    let mut state = ShipState::default();
    add(&mut state, 2.0, 2.0);
    assert_eq!(state.slowdown_timer, 2.0);

    // And a second one adds nothing at all: the timer is refilled to the same
    // ceiling rather than extended past it.
    add(&mut state, 2.0, 2.0);
    assert_eq!(state.slowdown_timer, 2.0);
}

/// A craft part-way through a slowdown is topped back up to the ceiling by a
/// new hit rather than left where it was - "each new impact refills the timer".
#[test]
fn a_later_hit_refills_a_partly_expired_timer() {
    let mut state = ShipState {
        slowdown_timer: 0.5,
        ..ShipState::default()
    };
    add(&mut state, 2.0, 2.0);
    assert_eq!(state.slowdown_timer, 2.0);
}

/// The residue `forces::evaluate` leaves below zero is spent against the next
/// hit, which is the whole reason that decrement is not clamped.
#[test]
fn the_negative_residue_is_subtracted_from_the_next_hit() {
    let dt = 1.0 / 60.0;
    let mut state = ShipState {
        slowdown_timer: -dt,
        ..ShipState::default()
    };
    add(&mut state, 1.0, 2.0);
    assert_eq!(state.slowdown_timer, 1.0 - dt);
}

/// The clamp is one-sided, as the original's is: a limit below where the timer
/// already sits pulls it down.
#[test]
fn the_clamp_has_no_floor() {
    let mut state = ShipState {
        slowdown_timer: 3.0,
        ..ShipState::default()
    };
    add(&mut state, 0.0, 1.0);
    assert_eq!(state.slowdown_timer, 1.0);
}
