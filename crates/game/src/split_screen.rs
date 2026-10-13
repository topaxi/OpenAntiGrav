//! Split screen from the command line: `--players N`.
//!
//! One window, one race, one tick, `N` views of it - the camera and draw halves
//! are `oag_raceplay`'s ([`Race::set_viewport_count`], `Scene::render_viewports`)
//! and the layout is `oag_render::viewport`. What lives here is the one place the
//! composition root turns a player count into a race: slots `0..N` become
//! [`Controller::Local`] and the screen is cut into `N` views, view `i` following
//! slot `i`.
//!
//! **Each `Local` slot is flown from its own snapshot**: `Race::tick` steps every
//! person-flown slot, not only the primary. Which device feeds which slot is
//! `oag_input::pad::Assignment`'s: `--players N` seats the keyboard and the first
//! pad on slot 0 and each further pad on the next slot ([`oag_input::Controls::seat_default`]),
//! and the join screen (`oag_ui_screens::join`, `oag_game`'s `session::join`)
//! seats whoever pressed a button, in the order they did ([`seat`]). Both are
//! **chosen, not measured**; the original's split screen takes its players from
//! the pads in port order.
//!
//! **Each view draws its own slot's HUD**, [`crate::hud_views`].

use oag_gameplay::{Controller, MAX_PLAYERS};
use oag_input::lobby::Lobby;
use oag_input::pad::Assignment;
use oag_raceplay::Race;

/// Makes `race` an `players`-way split screen and returns the count it settled
/// on, which is clamped to the craft fielded and to the grid's own cap. `1` -
/// the default - leaves the race exactly as it was.
pub fn apply(race: &mut Race, players: usize) -> usize {
    if players <= 1 {
        return 1;
    }
    let count = race.set_viewport_count(players);
    for slot in 0..count.min(MAX_PLAYERS) {
        race.sim.world.controllers[slot] = Controller::Local;
    }
    if count < players {
        log::warn!("--players {players}: only {count} craft are fielded, so {count} views");
    }
    log::info!("split screen: {count} views, slots 0..{count} are local players");
    count
}

/// Makes `race` the split screen `lobby` asked for and puts each device on the
/// slot it joined: slots `0..seated` become local players and `assignment` is
/// [`Lobby::seat`]'s, with `connected` the pads plugged in now. Returns the views the race settled on.
///
/// A lobby with one seat or none is an ordinary race: nothing changes, and the
/// assignment stays merged.
pub fn seat<D: Copy + PartialEq>(
    race: &mut Race,
    lobby: &Lobby<D>,
    assignment: &mut Assignment<D>,
    connected: &[D],
) -> usize {
    if lobby.count() <= 1 {
        return 1;
    }
    let count = apply(race, lobby.count());
    lobby.seat(assignment, connected);
    count
}

/// Keeps `race`'s local slots to the ones a device is on: a slot below the seated
/// count with nothing driving it is flown by the grid's driver
/// ([`Controller::Ai`]), and goes back to [`Controller::Local`] when a device
/// takes it again. Returns how many slots changed.
///
/// **Why.** A race ends when every person-flown slot has finished
/// (`oag_raceplay`'s `results.rs`), so a local slot nobody can steer - a pad
/// unplugged mid-race, or `--players 4` on a machine with one pad - would hold
/// the end-of-race screens for ever. **Chosen, not measured**: a slot whose
/// device left reverts to the AI rather than coasting or ending the race. A
/// remote slot is never touched. Run once a tick in the windowed session, before
/// [`Race::tick`], after the devices were polled; a no-op while the devices are
/// merged.
///
/// This writes `World::controllers`, which the state hash covers, from outside
/// the tick: a replay or a network client would have to carry it as an input.
pub fn follow_devices<D: Copy + PartialEq>(race: &mut Race, assignment: &Assignment<D>) -> usize {
    let Some(humans) = assignment.humans() else {
        return 0;
    };
    let mut changed = 0;
    for slot in 0..humans.min(MAX_PLAYERS) {
        let want = if assignment.drives(slot) {
            Controller::Local
        } else {
            Controller::Ai
        };
        let have = race.sim.world.controllers[slot];
        if have != want && matches!(have, Controller::Local | Controller::Ai) {
            race.sim.world.controllers[slot] = want;
            changed += 1;
            log::info!("split screen: slot {slot} is now {want:?}");
        }
    }
    changed
}
