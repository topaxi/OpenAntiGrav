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
//! **The full-screen HUD is hidden when `N > 1`** (chosen, not measured): one
//! HUD for slot 0 spread across a quadrant layout reads as every player's. A
//! per-player HUD is the next piece of work.

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
/// [`Lobby::seat`]'s. Returns the views the race settled on.
///
/// A lobby with one seat or none is an ordinary race: nothing changes, and the
/// assignment stays merged.
pub fn seat<D: Copy + PartialEq>(
    race: &mut Race,
    lobby: &Lobby<D>,
    assignment: &mut Assignment<D>,
) -> usize {
    if lobby.count() <= 1 {
        return 1;
    }
    let count = apply(race, lobby.count());
    lobby.seat(assignment);
    count
}
