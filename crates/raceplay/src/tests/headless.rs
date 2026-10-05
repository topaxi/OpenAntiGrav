//! [`Setup::headless`] and what the `oag-headless-sim` binary asserts about it.
//!
//! The binary is not run by `just test` - a `[[bin]]` is compiled by
//! `--all-targets` and executed by nothing - so its claim lives here as well,
//! where a regression fails the gate rather than waiting for somebody to run
//! it by hand. The binary stays, because "can this be driven from outside a
//! test harness with no renderer linked" is a different question from "does
//! the tick run", and only a real binary answers it.

use super::*;
use oag_gameplay::{Controller, PlayerInputs};

/// The whole claim: a race builds, ticks and changes state with no disc, no
/// GPU and no window anywhere in the process.
#[test]
fn a_disc_free_race_ticks_and_moves_its_hash() {
    let mut race = Race::start(Setup::headless(Mode::SingleRace, 0xC0FFEE));
    let before = race.sim.state_hash();

    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
        race.drain_cues();
    }

    assert_eq!(race.sim.world.tick, 30, "the clock did not advance");
    assert_ne!(
        before,
        race.sim.state_hash(),
        "thirty ticks changed no state at all"
    );
}

/// The binary marks three slots `Local`, and the point of doing so is that the
/// world hash sees it - otherwise the per-slot plumbing could be inert and
/// nothing would say so.
#[test]
fn marking_extra_slots_human_is_visible_to_the_hash() {
    let one = Race::start(Setup::headless(Mode::SingleRace, 0xC0FFEE));
    let mut three = Race::start(Setup::headless(Mode::SingleRace, 0xC0FFEE));
    assert_eq!(
        one.sim.state_hash(),
        three.sim.state_hash(),
        "two identical headless races disagree before either is touched"
    );

    for slot in 1..3 {
        three.sim.world.controllers[slot] = Controller::Local;
    }
    assert_eq!(three.sim.world.human_slots().count(), 3);
    assert_ne!(
        one.sim.state_hash(),
        three.sim.state_hash(),
        "who flies a slot did not reach the hash"
    );
}

/// A snapshot in a slot nobody flies steers nothing, which is what makes
/// `PlayerInputs`' dense array safe: seven unused entries cost eight bytes and
/// change no craft.
#[test]
fn a_snapshot_in_an_unflown_slot_changes_nothing() {
    let mut quiet = Race::start(Setup::headless(Mode::SingleRace, 0xC0FFEE));
    let mut noisy = Race::start(Setup::headless(Mode::SingleRace, 0xC0FFEE));

    let mut loud = PlayerInputs::none();
    for slot in 1..oag_gameplay::MAX_PLAYERS {
        let mut snapshot = oag_gameplay::InputSnapshot::EMPTY;
        snapshot.stick_x = 1.0;
        snapshot
            .buttons
            .begin_frame(oag_gameplay::input::Button::Cross.bit());
        loud.set(slot, snapshot);
    }

    for _ in 0..30 {
        quiet.tick(&PlayerInputs::none());
        quiet.drain_cues();
        noisy.tick(&loud);
        noisy.drain_cues();
    }

    assert_eq!(
        quiet.sim.state_hash(),
        noisy.sim.state_hash(),
        "input in a slot the AI flies reached the simulation"
    );
}
