//! Split screen from the command line: `--players N`.
//!
//! One window, one race, one tick, `N` views of it - the camera and draw halves
//! are `oag_raceplay`'s ([`Race::set_viewport_count`], `Scene::render_viewports`)
//! and the layout is `oag_render::viewport`. What lives here is the one place the
//! composition root turns a player count into a race: slots `0..N` become
//! [`Controller::Local`] and the screen is cut into `N` views, view `i` following
//! slot `i`.
//!
//! **Slot `i > 0` is still flown by the grid's driver until a second human slot's
//! input steers its craft**, which is the sim-side lane's work, not this one's:
//! today only [`oag_gameplay::World::primary_slot`] is stepped from a snapshot.
//! Marking the slot `Local` is the declaration; the views already show it flying.
//!
//! **Each view draws its own slot's HUD**, [`crate::hud_views`].

use oag_gameplay::{Controller, MAX_PLAYERS};
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
