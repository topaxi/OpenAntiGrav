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

/// One OS window's share of a multi-window race: how many seats it holds and how it cuts
/// them. A window of one seat is the whole-screen view; two or more are cut up inside the
/// window exactly as a split screen cuts its views ([`oag_render::viewport::layout`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowSeats {
    /// How many people this window shows. `0` is a window with nobody yet, which draws a
    /// holding frame.
    pub count: usize,
    /// Which way two seats are cut; means nothing for any other count.
    pub cut: oag_render::viewport::Cut,
}

/// `people` dealt across `windows` windows as evenly as it divides, the first windows taking
/// the remainder: 3 over 2 is `[2, 1]`, 2 over 4 is `[1, 1, 0, 0]` (the last two have
/// nobody). `--windows N` is this with `people == N` unless `--players` says otherwise.
/// **Chosen, not measured.**
#[must_use]
pub fn deal(people: usize, windows: usize, cut: oag_render::viewport::Cut) -> Vec<WindowSeats> {
    let windows = windows.max(1);
    let people = people.min(oag_render::viewport::MAX_VIEWPORTS);
    (0..windows)
        .map(|window| WindowSeats {
            count: people / windows + usize::from(window < people % windows),
            cut,
        })
        .collect()
}

/// `--window-seats`: the seats per window as a comma-separated list, each a count of 1 to 8 and
/// optionally `h` or `v` for the way two seats are cut (`4,2v`: four seats in window 1, two side
/// by side in window 2); a count without a letter takes `default_cut`. At least two windows
/// (one window is `--players`), at most eight seats in all.
///
/// # Errors
///
/// Names the entry that is not a count, a count out of range, a cut letter that is neither, a
/// list of one window, or more seats than the grid has views.
pub fn parse_window_seats(
    text: &str,
    default_cut: oag_render::viewport::Cut,
) -> Result<Vec<WindowSeats>, String> {
    let mut windows = Vec::new();
    for entry in text.split(',') {
        let entry = entry.trim();
        let (digits, cut) = match entry.chars().last() {
            Some('h' | 'H') => (
                &entry[..entry.len() - 1],
                oag_render::viewport::Cut::Horizontal,
            ),
            Some('v' | 'V') => (
                &entry[..entry.len() - 1],
                oag_render::viewport::Cut::Vertical,
            ),
            _ => (entry, default_cut),
        };
        let count: usize = digits.parse().map_err(|_| {
            format!(
                "{entry:?} is not a seat count: write a number, then h or v for how two are cut"
            )
        })?;
        if !(1..=oag_render::viewport::MAX_VIEWPORTS).contains(&count) {
            return Err(format!(
                "{entry:?} asks for {count} seats in a window; a window holds 1 to {}",
                oag_render::viewport::MAX_VIEWPORTS
            ));
        }
        windows.push(WindowSeats { count, cut });
    }
    if windows.len() < 2 {
        return Err("one window is --players; --window-seats takes a list like 4,2".to_string());
    }
    let total: usize = windows.iter().map(|window| window.count).sum();
    if total > oag_render::viewport::MAX_VIEWPORTS {
        return Err(format!(
            "{total} seats in all; the grid shows at most {}",
            oag_render::viewport::MAX_VIEWPORTS
        ));
    }
    Ok(windows)
}

/// The layout the window layout screen chose, as the seats per window the race deals by: the
/// same list `--window-seats` parses ([`parse_window_seats`]), a two-seat window's cut included.
#[must_use]
pub fn layout_of(windows: &[oag_ui_screens::window_layout::Window]) -> Vec<WindowSeats> {
    windows
        .iter()
        .map(|window| WindowSeats {
            count: window.seats,
            cut: if window.vertical {
                oag_render::viewport::Cut::Vertical
            } else {
                oag_render::viewport::Cut::Horizontal
            },
        })
        .collect()
}

/// The spelling [`parse_window_seats`] reads back: the counts, comma-separated, a window of two
/// seats carrying `h` or `v` for its cut (`4,2v`). Empty for fewer than two windows, which is
/// not a layout `--window-seats` takes.
#[must_use]
pub fn spell_window_seats(layout: &[WindowSeats]) -> String {
    if layout.len() < 2 {
        return String::new();
    }
    layout
        .iter()
        .map(|window| match (window.count, window.cut) {
            (2, oag_render::viewport::Cut::Vertical) => "2v".to_string(),
            (2, oag_render::viewport::Cut::Horizontal) => "2h".to_string(),
            (count, _) => count.to_string(),
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// How a window of `seats` seats cuts them (`vertical`: two side by side), as rectangles in the
/// unit square in seat order: the race's own cut ([`oag_render::viewport::layout`]) on a 16:9
/// window, so the layout screen's preview is the picture a window draws.
#[must_use]
pub fn seat_cells(seats: usize, vertical: bool) -> Vec<[f32; 4]> {
    use oag_render::viewport::{Cut, layout};
    // The gap between views is in pixels of the target, so the unit square is laid out at a
    // window's size and divided back.
    const WIDTH: f32 = 640.0;
    const HEIGHT: f32 = WIDTH * 9.0 / 16.0;
    let cut = if vertical {
        Cut::Vertical
    } else {
        Cut::Horizontal
    };
    layout(seats, (0.0, 0.0, WIDTH, HEIGHT), cut)
        .into_iter()
        .map(|(x, y, w, h)| [x / WIDTH, y / HEIGHT, w / WIDTH, h / HEIGHT])
        .collect()
}

/// The view count the load is told for a layout ([`oag_raceplay::Options::views`]): it reads a
/// title's authored two-player split HUD and packs its textures when the race has two views, and
/// nothing else about the count. `2` when any window holds exactly two seats, so that window can
/// use the authored layout; otherwise the largest window, which is `1` for a window each.
#[must_use]
pub fn views_for(layout: &[WindowSeats]) -> usize {
    if layout.iter().any(|window| window.count == 2) {
        2
    } else {
        layout
            .iter()
            .map(|window| window.count)
            .max()
            .unwrap_or(1)
            .max(1)
    }
}

/// The layout a race of `people` takes across `windows` open windows: `wanted`, the layout the
/// command line spelled, when its first `windows` entries add up to exactly `people`; otherwise
/// `people` dealt evenly ([`deal`]) with `cut`. A join screen that seats a different head count
/// than `--window-seats` spelled is dealt, not refused.
#[must_use]
pub fn layout_for(
    wanted: Option<&[WindowSeats]>,
    people: usize,
    windows: usize,
    cut: oag_render::viewport::Cut,
) -> Vec<WindowSeats> {
    if let Some(wanted) = wanted {
        let held = &wanted[..windows.min(wanted.len())];
        if held.iter().map(|window| window.count).sum::<usize>() == people {
            return held.to_vec();
        }
    }
    deal(people, windows, cut)
}

/// Makes `race` a multi-window race of `layout`: the cameras and local slots of a split screen of
/// all the seats, which are then dealt across the windows in slot order, window 1 taking slots
/// from 0. Returns how many views the race settled on (clamped to the craft fielded), which is
/// `1` and leaves the race ordinary when the layout holds one person or none.
pub fn apply_windows(race: &mut Race, layout: &[WindowSeats]) -> usize {
    let total: usize = layout.iter().map(|window| window.count).sum();
    let cut = layout
        .first()
        .map_or_else(Default::default, |window| window.cut);
    let count = apply(race, total, cut);
    deal_windows(race, layout);
    count
}

/// Deals the views a race already has ([`apply`]) across the windows of `layout`, in slot order,
/// window 1 taking slots from 0: the presentation alone, which a race that was seated first (the
/// join screen's, a restart's kept seats) takes afterwards. A race of one view stays ordinary.
pub fn deal_windows(race: &mut Race, layout: &[WindowSeats]) {
    if race.viewport_count() <= 1 {
        return;
    }
    let mut next = 0;
    let views = layout
        .iter()
        .map(|window| {
            let seats = (next..next + window.count).collect();
            next += window.count;
            oag_raceplay::WindowViews {
                seats,
                cut: window.cut,
            }
        })
        .collect();
    race.set_windows(views);
    log::info!(
        "multi-window: {} window(s) holding {}",
        layout.len(),
        layout
            .iter()
            .map(|window| window.count.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
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
