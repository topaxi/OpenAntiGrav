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
    split_screen::apply(&mut race, 2, oag_render::viewport::Cut::Horizontal);
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
    assert_eq!(
        split_screen::seat(
            &mut race,
            &lobby,
            &mut assignment,
            &[5, 9, 1],
            oag_render::viewport::Cut::Horizontal
        ),
        3
    );

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
    assert_eq!(
        split_screen::seat(
            &mut race,
            &lobby,
            &mut assignment,
            &[5, 9, 1],
            oag_render::viewport::Cut::Horizontal
        ),
        1
    );
    assert_eq!(race.viewport_count(), 1);
    assert_eq!(race.sim.world.human_slots().count(), 1);
    assert!(assignment.is_single_player());
}

/// A race ends when every person-flown slot has finished, so a local slot with
/// no device would hold the results forever. With three seated and no pad on
/// slot 2 the grid flies it; a pad that appears takes it back, and one that
/// goes away gives it up. Remote slots are left alone.
#[test]
fn a_local_slot_without_a_device_is_flown_by_the_ai_and_returns_with_its_pad() {
    let mut race = race(3);
    let mut assignment = Assignment::<u32>::seated(3);
    assignment.claim_keyboard(0);
    assignment.assign_pad(5, 1);
    split_screen::apply(&mut race, 3, oag_render::viewport::Cut::Horizontal);
    let local = |race: &Race, slot: usize| race.sim.world.controllers[slot] == Controller::Local;

    assignment.reconcile(&[5]);
    assert_eq!(split_screen::follow_devices(&mut race, &assignment), 1);
    assert!(local(&race, 0), "the keyboard's slot");
    assert!(local(&race, 1), "pad 5's slot");
    assert!(!local(&race, 2), "nobody is on slot 2");
    assert!(
        !race.sim.world.person_flown(2),
        "the end rule would wait on it"
    );

    assignment.reconcile(&[5, 9]);
    split_screen::follow_devices(&mut race, &assignment);
    assert!(local(&race, 2), "a pad that connected took the slot back");

    assignment.reconcile(&[9]);
    split_screen::follow_devices(&mut race, &assignment);
    assert!(!local(&race, 1), "pad 5 left, so its slot flies as the AI");
    assert!(race.sim.world.person_flown(0));

    race.sim.world.controllers[2] = Controller::Remote;
    assignment.reconcile(&[]);
    split_screen::follow_devices(&mut race, &assignment);
    assert_eq!(race.sim.world.controllers[2], Controller::Remote);
}

/// Closing a window hands every seat it held to the AI **even though their pads are still
/// plugged in**, and a pad that reconnects does not take one back: nobody could see the craft.
/// Window 2 here holds two seats. Fails if closing a window only forgets the window and leaves
/// the people at it flying blind.
#[test]
fn a_closed_window_hands_all_its_seats_to_the_ai_and_the_pads_cannot_take_them_back() {
    use oag_game::split_screen::{WindowSeats, deal_windows};
    let cut = oag_render::viewport::Cut::Horizontal;
    let mut race = race(3);
    let mut assignment = Assignment::<u32>::seated(3);
    assignment.claim_keyboard(0);
    split_screen::apply(&mut race, 3, cut);
    deal_windows(
        &mut race,
        &[WindowSeats { count: 1, cut }, WindowSeats { count: 2, cut }],
    );
    assert_eq!(
        race.windows()[1].seats,
        vec![1, 2],
        "window 2 holds slots 1 and 2"
    );
    assignment.reconcile(&[7, 8]);
    split_screen::follow_devices(&mut race, &assignment);
    let controller = |race: &Race, slot: usize| race.sim.world.controllers[slot];
    assert_eq!(controller(&race, 1), Controller::Local);
    assert_eq!(controller(&race, 2), Controller::Local);

    // What `Session::close_screen` does for window 2.
    for slot in race.windows()[1].seats.clone() {
        assignment.unseat(slot);
    }
    race.remove_window(1);
    assert_eq!(split_screen::follow_devices(&mut race, &assignment), 2);
    assert_eq!(controller(&race, 1), Controller::Ai);
    assert_eq!(controller(&race, 2), Controller::Ai);
    assert_eq!(
        controller(&race, 0),
        Controller::Local,
        "window 1's person flies on"
    );

    assignment.reconcile(&[]);
    assignment.reconcile(&[7, 8]);
    split_screen::follow_devices(&mut race, &assignment);
    assert_eq!(controller(&race, 1), Controller::Ai);
    assert_eq!(controller(&race, 2), Controller::Ai);
    assert!(
        race.windowed(),
        "the race itself is still windowed, by window 1 alone"
    );
    assert_eq!(race.windows().len(), 1);
}

/// Merged devices are a single person: nothing is reverted.
#[test]
fn merged_devices_leave_the_controllers_alone() {
    let mut race = race(2);
    split_screen::apply(&mut race, 2, oag_render::viewport::Cut::Horizontal);
    let before = race.sim.state_hash();
    assert_eq!(
        split_screen::follow_devices(&mut race, &Assignment::<u32>::default()),
        0
    );
    assert_eq!(race.sim.state_hash(), before);
}

/// A restart from the end screens skips the menus: the seats the first race left
/// are the split of the second, and leaving for the menus is what lets the next
/// launch be one view again. This is the function the race build calls
/// (`Session`'s load, `split_screen::seat_for_launch`).
#[test]
fn a_restart_rebuilds_the_same_split_and_a_visit_to_the_menus_ends_it() {
    let mut controls = Controls::without_pad();
    // `--players 2` seats the devices without a lobby; the join screen's own seating is
    // `split_screen::seat`, tested above over stand-in pad ids.
    controls.seat_default(2);
    let mut lobby = None;

    let mut first = race(2);
    split_screen::seat_for_launch(
        &mut lobby,
        &mut controls,
        &mut first,
        oag_render::viewport::Cut::Horizontal,
    );
    assert_eq!(first.viewport_count(), 2);
    assert_eq!(controls.assignment().humans(), Some(2));

    // `RACE AGAIN`: no lobby, the seats are still held.
    let mut again = race(2);
    split_screen::seat_for_launch(
        &mut lobby,
        &mut controls,
        &mut again,
        oag_render::viewport::Cut::Horizontal,
    );
    assert_eq!(again.viewport_count(), 2, "the restart is the same split");
    assert_eq!(again.sim.world.controllers[0], Controller::Local);
    assert_eq!(again.sim.world.controllers[1], Controller::Local);

    // Back on the menus (`Session::unseat_after_split_race`): the next race is one view.
    controls.merge_devices();
    let mut fresh = race(2);
    split_screen::seat_for_launch(
        &mut lobby,
        &mut controls,
        &mut fresh,
        oag_render::viewport::Cut::Horizontal,
    );
    assert_eq!(fresh.viewport_count(), 1);
}

/// The cut a SPLIT SCREEN VERTICAL row chose reaches the race through the seating call
/// the race build makes, for two views and for no other count. Where the race draws it is
/// the viewport layout's (`Race::cut`); without a cut the race is cut top and bottom.
#[test]
fn the_cut_the_row_chose_reaches_the_race_for_two_views_only() {
    use oag_render::viewport::Cut;
    let mut controls = Controls::without_pad();
    controls.seat_default(2);
    let mut vertical = race(2);
    split_screen::seat_for_launch(&mut None, &mut controls, &mut vertical, Cut::Vertical);
    assert_eq!(vertical.viewport_count(), 2);
    assert_eq!(vertical.cut(), Cut::Vertical);

    controls.seat_default(3);
    let mut three = race(3);
    split_screen::seat_for_launch(&mut None, &mut controls, &mut three, Cut::Vertical);
    assert_eq!(three.viewport_count(), 3);
    assert_eq!(
        three.cut(),
        Cut::Horizontal,
        "only two views can be cut left and right"
    );

    let ordinary = race(2);
    assert_eq!(
        ordinary.cut(),
        Cut::Horizontal,
        "an ordinary race is cut as it always was"
    );
}
/// The cut reaches the race through both ways a split is made: `apply` (the command line and
/// a restart) and `seat` (the join screen's lobby). Two views cut vertically are columns,
/// the same race cut horizontally is rows, and a lobby of one is no split at all.
#[test]
fn the_cut_reaches_the_race_through_apply_and_seat() {
    use oag_render::viewport::Cut;
    const AREA: (f32, f32, f32, f32) = (0.0, 0.0, 1920.0, 1080.0);
    let mut by_apply = race(2);
    assert_eq!(split_screen::apply(&mut by_apply, 2, Cut::Vertical), 2);
    let columns = by_apply.view_rects(AREA);
    assert!(
        columns[1].0 > columns[0].0 && columns[0].3 == 1080.0,
        "{columns:?}"
    );

    let mut lobby = Lobby::new();
    lobby.join(Device::Keyboard);
    lobby.join(Device::Pad(5));
    let mut by_seat = race(2);
    let mut assignment = Assignment::<u32>::default();
    assert_eq!(
        split_screen::seat(&mut by_seat, &lobby, &mut assignment, &[5], Cut::Vertical),
        2
    );
    assert_eq!(
        by_seat.view_rects(AREA),
        columns,
        "a lobby cuts as apply does"
    );

    let mut rows = race(2);
    split_screen::apply(&mut rows, 2, Cut::Horizontal);
    assert_eq!(rows.view_rects(AREA)[0].2, 1920.0);

    let mut alone = race(2);
    let mut one = Lobby::new();
    one.join(Device::Keyboard);
    let mut assignment = Assignment::<u32>::default();
    assert_eq!(
        split_screen::seat(&mut alone, &one, &mut assignment, &[], Cut::Vertical),
        1
    );
    assert_eq!(alone.view_rects(AREA), vec![AREA], "one person is no split");
}
