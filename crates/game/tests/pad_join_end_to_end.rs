//! From a device's seat to its craft, with no disc, window or pad attached.
//!
//! Three links, each checked where it breaks. `oag-input`'s own tests cover a pad
//! reading on slot 1 becoming slot 1's snapshot and the lobby's rules. This file
//! covers the two the composition root owns: the lobby making a race (which
//! slots are local, how many views), and a snapshot on slot 1 steering slot 1's
//! craft and nobody else's through the same `Controls` -> `Race::tick` path a
//! window runs.

use oag_game::split_screen;
use oag_gameplay::{Controller, MAX_PLAYERS};
use oag_input::Controls;
use oag_input::lobby::{Device, Lobby};
use oag_input::pad::Assignment;
use oag_race::Mode;
use oag_raceplay::{Race, Setup};
use winit::keyboard::{Key, NamedKey};

const SEED: u64 = 0xC0FFEE;

/// A headless race with `craft` craft on it, each able to turn at all: the
/// fixture authors no handling, so without that a stick changes nothing and
/// "slot 1 steered" proves nothing, and it fields one craft, which a split
/// screen would clamp to one view. The others are slot 0's craft copied well to
/// its right, because sharing a spawn point would have the pair resolver fling
/// them apart and hide what a stick did.
fn race(craft: usize) -> Race {
    let mut race = Race::start(Setup::headless(Mode::SingleRace, SEED));
    race.sim.world.ships[0].handling.turning = oag_physics::params::Turning {
        amount: 60.0,
        falloff: 5.0,
        gain: 5.0,
    };
    for slot in 1..craft {
        let mut ship = race.sim.world.ships[0];
        let pose = oag_gameplay::Pose {
            position: ship.physics.body.position + ship.physics.body.right() * (60.0 * slot as f32),
            orientation: ship.physics.body.orientation,
        };
        ship.place_at(pose);
        ship.active = true;
        race.sim.world.ships[slot] = ship;
    }
    race.sim.world.ship_count = u8::try_from(craft).unwrap();
    race
}

/// Sixty ticks of the real input path: `keyboard_slot`'s device holds right.
/// `None` is nobody touching anything.
fn fly(keyboard_slot: Option<usize>) -> Race {
    let mut race = race(2);
    split_screen::apply(&mut race, 2);
    let mut controls = Controls::without_pad();
    if let Some(slot) = keyboard_slot {
        controls.assignment_mut().assign_keyboard(slot);
        controls.set_key(&Key::Named(NamedKey::ArrowRight), true);
    }
    for _ in 0..60 {
        let inputs = controls.player_snapshots();
        race.tick(&inputs);
        race.drain_cues();
    }
    race
}

/// The claim of the lane at the boundary a race reads: the device assigned to
/// slot 1 turns slot 1's craft, and slot 0's is where it was with nobody
/// steering.
#[test]
fn the_device_on_slot_one_steers_slot_ones_craft_only() {
    let idle = fly(None);
    let on_one = fly(Some(1));
    let on_zero = fly(Some(0));

    let heading = |race: &Race, slot: usize| race.sim.world.ships[slot].physics.body.forward();
    assert_ne!(
        heading(&on_one, 1),
        heading(&idle, 1),
        "slot 1's device did not turn slot 1's craft"
    );
    assert_eq!(
        heading(&on_one, 0),
        heading(&idle, 0),
        "slot 1's device turned slot 0's craft"
    );
    // And the control: the same key on slot 0 turns slot 0 and leaves slot 1.
    assert_ne!(heading(&on_zero, 0), heading(&idle, 0));
    assert_eq!(heading(&on_zero, 1), heading(&idle, 1));
}

/// Three joined devices make a three-way split: slots `0..3` are local, three
/// views, and each device is on the slot it joined at - the pad that pressed
/// first on slot 0, the keyboard on 1.
#[test]
fn a_three_seat_lobby_makes_a_three_way_split_with_each_device_on_its_slot() {
    let mut lobby = Lobby::<u32>::new();
    lobby.join(Device::Pad(5));
    lobby.join(Device::Keyboard);
    lobby.join(Device::Pad(9));

    let mut race = race(3);
    let mut assignment = Assignment::<u32>::default();
    assert_eq!(split_screen::seat(&mut race, &lobby, &mut assignment), 3);

    assert_eq!(race.viewport_count(), 3);
    for slot in 0..MAX_PLAYERS {
        let local = race.sim.world.controllers[slot] == Controller::Local;
        assert_eq!(local, slot < 3, "slot {slot}");
    }
    assert_eq!(assignment.slot_of(5), Some(0));
    assert_eq!(assignment.keyboard_slot(), 1);
    assert_eq!(assignment.slot_of(9), Some(2));
    assert_eq!(assignment.slot_of(1), None, "a pad that never joined");
}

/// One seat is not a split: the race and the assignment stay as they were, so a
/// player who opens the screen and starts alone gets an ordinary race.
#[test]
fn a_lobby_of_one_changes_nothing() {
    let mut lobby = Lobby::<u32>::new();
    lobby.join(Device::Keyboard);
    let mut race = race(3);
    let mut assignment = Assignment::<u32>::default();
    assert_eq!(split_screen::seat(&mut race, &lobby, &mut assignment), 1);
    assert_eq!(race.viewport_count(), 1);
    assert_eq!(race.sim.world.human_slots().count(), 1);
    assert!(assignment.is_single_player());
}
