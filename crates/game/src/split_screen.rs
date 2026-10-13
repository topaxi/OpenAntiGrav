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

/// Whether `title` offers a split screen on an adapter of kind `device`.
///
/// **The maintainer's product rule (2026-10-13)**: on an integrated GPU a split is offered
/// for the titles that say so (`oag_title::Title::split_on_integrated_gpu`: Pulse and
/// Pure) and for no other; every other kind of adapter, a software one included, offers
/// every title. The rule is data on the title, not a comparison of names.
#[must_use]
pub fn offered(title: &oag_title::Title, device: wgpu::DeviceType) -> bool {
    device != wgpu::DeviceType::IntegratedGpu || title.split_on_integrated_gpu
}

/// Takes the SPLIT SCREEN row out of `definition` when `title` offers no split on `device`
/// ([`offered`]); returns whether it did. Run on every menu a run installs, once the
/// adapter is known.
pub fn trim_menu(
    definition: &mut oag_ui::menu::Definition,
    title: &oag_title::Title,
    device: wgpu::DeviceType,
) -> bool {
    if offered(title, device) {
        return false;
    }
    definition.drop_split_screen();
    true
}

/// The view count `--players players` settles on for `title` on `device`: `players` where
/// the split is [`offered`], else one, said in the log.
#[must_use]
pub fn players_offered(title: &oag_title::Title, device: wgpu::DeviceType, players: u8) -> u8 {
    if players <= 1 || offered(title, device) {
        return players;
    }
    log::warn!(
        "--players {players}: split screen is not offered for {} on an integrated GPU, \
         so one view",
        title.name
    );
    1
}

/// The head count the race box's `NUMBER OF PLAYERS` row holds: `pick` read as the disc's
/// own entry of `counts`, on a title whose race box authors the list
/// ([`oag_title::FrontEnd::local_split`]); `1` for a pick the list does not hold and on every
/// other title, where the generic SPLIT SCREEN row is the entry.
#[must_use]
pub fn authored_people(
    title: &oag_title::Title,
    counts: &[oag_ui::menu::Choice],
    pick: &str,
) -> usize {
    let authored = title
        .front_end
        .is_some_and(|front_end| front_end.local_split.is_some());
    if authored && counts.iter().any(|choice| choice.value == pick) {
        pick.parse().unwrap_or(1)
    } else {
        1
    }
}

/// The cut the race box's `SPLIT SCREEN` row holds: `RB_VERT` is left and right, and anything
/// the disc's own list does not name is the horizontal cut it defaults to.
#[must_use]
pub fn authored_cut(cuts: &[oag_ui::menu::Choice], pick: &str) -> oag_render::viewport::Cut {
    if pick == "RB_VERT" && cuts.iter().any(|choice| choice.value == pick) {
        oag_render::viewport::Cut::Vertical
    } else {
        oag_render::viewport::Cut::Horizontal
    }
}

/// Makes `race` an `players`-way split screen and returns the count it settled
/// on, which is clamped to the craft fielded and to the grid's own cap. `1` -
/// the default - leaves the race exactly as it was. `cut` is how two views are
/// cut (`oag_render::viewport::Cut`); it means nothing for any other count.
pub fn apply(race: &mut Race, players: usize, cut: oag_render::viewport::Cut) -> usize {
    if players <= 1 {
        return 1;
    }
    let count = race.set_viewport_count(players);
    race.set_cut(cut);
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
    cut: oag_render::viewport::Cut,
) -> usize {
    if lobby.count() <= 1 {
        return 1;
    }
    let count = apply(race, lobby.count(), cut);
    lobby.seat(assignment, connected);
    count
}

/// Hands the seats to the race a launch has just built: the join screen's lobby if
/// Start left one ([`seat`]), else the seats the last race kept - a restart from the
/// end screens skips the menus, so the devices are still seated and the new race is
/// the same split ([`apply`]). With neither it is an ordinary race and nothing
/// changes. Leaving a split race for the menus merges the devices
/// ([`Controls::merge_devices`]), which is what makes the next launch one view
/// again.
pub fn seat_for_launch(
    lobby: &mut Option<oag_input::lobby::Lobby>,
    controls: &mut oag_input::Controls,
    race: &mut Race,
    cut: oag_render::viewport::Cut,
) {
    if let Some(lobby) = lobby.take() {
        let connected = controls.connected_pads();
        seat(race, &lobby, controls.assignment_mut(), &connected, cut);
    } else if let Some(humans) = controls.assignment().humans() {
        apply(race, humans, cut);
    }
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

#[cfg(test)]
mod tests;
