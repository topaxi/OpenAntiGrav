//! The barrel roll is reachable from a real input snapshot.
//!
//! Every other test of this mechanic drives `oag_physics::ShipState` or
//! `oag_physics::barrel_roll` directly, which proves the state machine and
//! proves nothing about whether a pilot can ever get to it. Until this file
//! existed the whole mechanic was unreachable in a live race - `ShipControls`
//! carried no tap and `crate::forces::evaluate` never called `record_tap` -
//! and no test in the tree could have noticed, because none of them started
//! from an [`InputSnapshot`].
//!
//! So this one starts there deliberately, and goes through
//! [`oag_gameplay::controls::ship_controls`] and [`oag_physics::step`] - the
//! same two calls `oag_game`'s race loop makes - rather than building a
//! `ShipControls` by hand. Building one by hand would skip the binding under
//! test.
//!
//! The parameter set is `oag_physics::probe::handling()`, not `Handling::ZERO`:
//! with every tunable zeroed the roll costs nothing, `arm` charges nothing and
//! the phase never ramps, so a test could pass while no roll ever happened.

use oag_gameplay::controls::{ControlScheme, ship_controls};
use oag_gameplay::input::{Button, Input, InputSnapshot};
use oag_physics::probe;
use oag_physics::{Environment, Handling, ShipState, step};

/// Our own fixed timestep. ADR-0007.
const TICK: f32 = 1.0 / 60.0;

/// [`probe::start`]'s craft, lifted clear of the corridor floor.
///
/// **Every gesture in this file has to start airborne**, because the original
/// zeroes the whole tap history on every tick the contact bit is set
/// (`0x08846bd0`; see `oag_physics::barrel_roll::advance_gesture`) and this
/// port does the same. Tapping from `probe::start`'s resting craft would
/// measure that gate rather than the binding this file exists to test.
/// A kilometre of air is far more than the six ticks a gesture takes.
fn airborne(handling: &Handling) -> ShipState {
    let mut state = probe::start(handling);
    state.body.position.y = 1_000.0;
    state.grounded = 0.0;
    state.grounded_prev = 0.0;
    state
}

/// What a pilot is doing on one tick, in the two terms this gesture reads.
#[derive(Clone, Copy, Default)]
struct Tick {
    dpad: u32,
    stick_x: f32,
}

/// Steps the real simulation once, from a snapshot rather than from controls.
fn drive(state: &mut ShipState, buttons: &mut Input, tick: Tick) {
    buttons.begin_frame(tick.dpad);
    let snapshot = InputSnapshot {
        buttons: *buttons,
        stick_x: tick.stick_x,
        ..InputSnapshot::new()
    }
    .sanitised();

    let controls = ship_controls(&snapshot, ControlScheme::Veteran);
    step(
        state,
        &controls,
        &probe::handling(),
        &Environment::default(),
        &probe::corridor(),
        TICK,
    );
}

/// The d-pad leg, end to end.
///
/// Three alternating presses with a released tick between them - a press is an
/// edge, so a held direction is one tap and not one a tick. The shield charge
/// is the proof that `arm` ran rather than merely `record_tap`: `roll_cost` is
/// 8 % of a 100-unit pool in `probe::handling()`.
#[test]
fn three_d_pad_taps_from_an_input_snapshot_arm_a_roll() {
    let handling = probe::handling();
    let mut state = airborne(&handling);
    let mut buttons = Input::new();
    let full_pool = state.shield;

    for button in [Button::Left, Button::Right, Button::Left] {
        drive(
            &mut state,
            &mut buttons,
            Tick {
                dpad: button.bit(),
                ..Tick::default()
            },
        );
        drive(&mut state, &mut buttons, Tick::default());
    }

    assert_eq!(state.roll_target, -1.0, "left-right-left rolls to -1");
    assert_eq!(
        state.shield,
        full_pool - handling.roll_cost * 0.01 * handling.dimensions.shield,
        "the arm charged `roll_cost` percent of the pool"
    );
    assert!(
        state.roll_phase < 0.0,
        "and the phase is already ramping: {}",
        state.roll_phase
    );
}

/// The steering-axis leg, end to end, with no button pressed at all.
///
/// The original feeds the same tap history from "the steering axis crossing
/// below `-90`" as from the d-pad bit, so a pad with only an analog stick can
/// roll. Each push returns to centre so the next one is a fresh crossing.
#[test]
fn three_stick_crossings_from_an_input_snapshot_arm_a_roll() {
    let handling = probe::handling();
    let mut state = airborne(&handling);
    let mut buttons = Input::new();

    for push in [1.0, -1.0, 1.0] {
        drive(
            &mut state,
            &mut buttons,
            Tick {
                stick_x: push,
                ..Tick::default()
            },
        );
        drive(&mut state, &mut buttons, Tick::default());
    }

    assert_eq!(state.roll_target, 1.0, "right-left-right rolls to +1");
    assert!(state.shield < handling.dimensions.shield, "it was charged");
}

/// Cornering is not a barrel roll.
///
/// The control this whole file needs: a pilot holding the stick over through a
/// long corner crosses the threshold once and never alternates, so nothing
/// arms and nothing is charged. Without this, a test that only ever asserts
/// "the roll armed" cannot tell a working gesture from one that fires on any
/// steering at all.
#[test]
fn holding_the_stick_through_a_corner_arms_nothing() {
    let handling = probe::handling();
    // Airborne, so this measures the alternation and not the grounded gate.
    let mut state = airborne(&handling);
    let mut buttons = Input::new();

    for _ in 0..180 {
        drive(
            &mut state,
            &mut buttons,
            Tick {
                stick_x: -1.0,
                ..Tick::default()
            },
        );
    }

    assert_eq!(state.roll_target, 0.0);
    assert_eq!(state.roll_phase, 0.0);
    assert_eq!(state.shield, handling.dimensions.shield, "never charged");
}

/// A gap past the inter-tap timeout breaks the gesture, through the real path.
///
/// `INTER_TAP_TIMEOUT` is 0.6 s, so 40 idle ticks between taps is comfortably
/// past it. This is what stops three unrelated corners over ten seconds from
/// adding up to a roll.
#[test]
fn taps_spaced_past_the_timeout_never_complete() {
    let handling = probe::handling();
    // Airborne, so this measures the timeout and not the grounded gate.
    let mut state = airborne(&handling);
    let mut buttons = Input::new();

    for button in [Button::Left, Button::Right, Button::Left] {
        drive(
            &mut state,
            &mut buttons,
            Tick {
                dpad: button.bit(),
                ..Tick::default()
            },
        );
        for _ in 0..40 {
            drive(&mut state, &mut buttons, Tick::default());
        }
    }

    assert_eq!(state.roll_target, 0.0, "nothing armed");
    assert_eq!(state.shield, handling.dimensions.shield, "never charged");
}

/// And the gate itself, from a snapshot: the same gesture that arms in the air
/// arms nothing at all on the ground, and costs nothing either.
///
/// This is the fix for the `07_Track` playability regression - see
/// `oag_physics::barrel_roll::advance_gesture` - so it is worth one test at the
/// level a pilot actually reaches the mechanic from.
#[test]
fn the_same_gesture_on_the_ground_arms_nothing() {
    let handling = probe::handling();
    // Inside the hover probe's reach: `probe::start` leaves the craft at
    // exactly its ride height, where the probe finds nothing and `grounded`
    // reads `0.0` from the first tick on - which would silently turn this into
    // the airborne case. Ten units up is in contact and stays there.
    let mut state = probe::start(&handling);
    state.body.position.y = 10.0;
    let mut buttons = Input::new();

    assert!(
        state.grounded > 0.0,
        "the fixture is meant to start resting"
    );
    for button in [Button::Left, Button::Right, Button::Left] {
        for tick in [
            Tick {
                dpad: button.bit(),
                ..Tick::default()
            },
            Tick::default(),
        ] {
            drive(&mut state, &mut buttons, tick);
            assert!(
                state.grounded > 0.0,
                "the fixture left the ground, so this stopped measuring the gate"
            );
        }
    }

    assert_eq!(state.roll_target, 0.0, "a grounded craft armed a roll");
    assert_eq!(state.shield, handling.dimensions.shield, "and paid for it");
}
