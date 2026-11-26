//! One race, built the same way on the server and on every client.
//!
//! The protocol sends inputs and a state hash, never state, so the server and the
//! client must start from the same `RaceSim` bit for bit: the same content, the
//! same options, and the same settings applied after `Race::start`. Both ends go
//! through [`options`] and [`prepare`] here and nowhere else, and the client proves
//! it arrived at the server's state by the tick-0 hash ([`fingerprint`]) before the
//! race begins.
//!
//! # Chosen, not measured
//!
//! Version 1 races hold these fixed, each a setting that is one per race in this
//! engine rather than one per person:
//!
//! - **This project's AI driver flies the opponents.** The original driver's
//!   handling of several humans is unread (`oag_game::ai_choice`'s own
//!   `MultipleHumans`), and one rule for every network race keeps the server and
//!   the client on one path.
//! - **Pilot assist off** for every person (a per-seat assist is open work).
//! - **Weapons are the room's**, and they are predicted simulation, not server events
//!   (ADR-0071): a client compares the discrete weapon state at every confirmed tick
//!   ([`crate::weapon_sync`]).
//! - **Every person's slot is [`oag_gameplay::Controller::Remote`] on every peer**, the client's
//!   own included, because the controllers are hashed. Which slot is this machine's
//!   is view-side: `Race::set_seat`.

use anyhow::{Context, Result, anyhow};
use oag_raceplay::{Options, Race};

use crate::wire::RaceParams;
use crate::wire::board::{Board, BoardRow};

/// The race options `params` names, opened from `source` (this machine's own copy
/// of the title).
///
/// # Errors
///
/// A mode, difficulty or control scheme this build does not know.
pub fn options(params: &RaceParams, source: &str) -> Result<Options> {
    let mode = oag_race::Mode::from_name(&params.mode)
        .ok_or_else(|| anyhow!("the server's race mode {:?} is unknown here", params.mode))?;
    let difficulty = oag_ai::Difficulty::from_name(&params.difficulty).ok_or_else(|| {
        anyhow!(
            "the server's difficulty {:?} is unknown here",
            params.difficulty
        )
    })?;
    scheme(params)?;
    Ok(Options {
        source: source.to_string(),
        track: Some(params.track.clone()).filter(|t| !t.is_empty()),
        team: seat_team(params, 0),
        seat_picks: (1..usize::from(params.people).min(oag_gameplay::MAX_PLAYERS))
            // A seat with no team ends the list: a pick is the next slot's, so none may be skipped.
            .map_while(|seat| seat_team(params, seat))
            .map(|team| oag_raceplay::SeatPick {
                team,
                hull_variant: None,
                skin: None,
            })
            .collect(),
        class: params.class.clone(),
        mode,
        difficulty,
        seed: Some(params.seed),
        people: params.people,
        opponents: params.opponents,
        weapons_override: Some(params.weapons),
        laps_override: lap_count(mode, params.laps),
        no_driver_for_class: !oag_ai::coverage::project_driver_covers(&params.class),
        server_source: true,
        ..Options::default()
    })
}

/// Whether `mode` ends on a lap count, so a room may set one. `Options::laps_override` must
/// not be set for the rest: Speed Lap and Zone never end on their own, and Eliminator ends on
/// kills.
#[must_use]
pub fn mode_has_laps(mode: oag_race::Mode) -> bool {
    mode.laps_target(oag_race::SpeedClass::ALL[0]).is_some()
}

/// The lap count a race runs: the room's, in a mode that ends on laps, else the mode's own.
fn lap_count(mode: oag_race::Mode, laps: u8) -> Option<u32> {
    (laps > 0 && mode_has_laps(mode)).then_some(u32::from(laps))
}

/// The team seat `seat` flies: its own entry in [`RaceParams::teams`], else the race's default.
fn seat_team(params: &RaceParams, seat: usize) -> Option<String> {
    let team = params
        .teams
        .get(seat)
        .filter(|team| !team.is_empty())
        .unwrap_or(&params.team);
    (!team.is_empty()).then(|| team.clone())
}

fn scheme(params: &RaceParams) -> Result<oag_gameplay::ControlScheme> {
    oag_gameplay::ControlScheme::from_name(&params.scheme).ok_or_else(|| {
        anyhow!(
            "the server's control scheme {:?} is unknown here",
            params.scheme
        )
    })
}

/// Applies what a network race holds fixed (the module doc) to a race just
/// started from [`options`]. Both ends call it before [`fingerprint`].
///
/// # Errors
///
/// A control scheme this build does not know.
pub fn prepare(race: &mut Race, params: &RaceParams) -> Result<()> {
    race.core.sim.set_control_scheme(scheme(params)?);
    race.core.sim.set_autopilot(false);
    // Off for everybody unless the room allowed it, in which case each seat flies the level
    // that person asked for: the same list on both ends, so the physics agrees.
    race.core
        .sim
        .set_pilot_assist(oag_physics::pilot_assist::Level::Off);
    race.core.sim.apply_room(&room(params));
    Ok(())
}

/// What `params` holds fixed beyond [`options`], for [`RaceSim::apply_room`](oag_raceplay::RaceSim::apply_room) and a recording's header.
#[must_use]
pub fn room(params: &RaceParams) -> oag_raceplay::replay::Room {
    let people = params
        .people
        .min(u8::try_from(oag_gameplay::MAX_PLAYERS).unwrap_or(u8::MAX));
    oag_raceplay::replay::Room {
        people,
        assists: (0..usize::from(people))
            .map(|seat| seat_assist(params, seat))
            .collect(),
        barrel_rolls: !params.no_barrel_rolls,
        zone_target: zone_target(params),
    }
}

/// The `options` table a recording of this room's race carries, so the file rebuilds as the
/// race: the launch settings `prepare` applies, and the room.
///
/// # Errors
///
/// A difficulty or control scheme this build does not know.
pub fn replay_options(params: &RaceParams) -> Result<std::collections::BTreeMap<String, String>> {
    let difficulty = oag_ai::Difficulty::from_name(&params.difficulty).ok_or_else(|| {
        anyhow!(
            "the server's difficulty {:?} is unknown here",
            params.difficulty
        )
    })?;
    let launch = oag_raceplay::replay::Launch {
        scheme: scheme(params)?,
        difficulty,
        autopilot: false,
        pilot_assist: oag_physics::pilot_assist::Level::Off,
        original_ai: None,
        campaign_skill: None,
        skill_level: None,
    };
    let mut table = launch.to_table();
    table.extend(room(params).to_table());
    Ok(table)
}

/// The Pilot Assist level seat `seat` flies: its entry in [`RaceParams::assists`], else off.
fn seat_assist(params: &RaceParams, seat: usize) -> oag_physics::pilot_assist::Level {
    params
        .assists
        .get(seat)
        .and_then(|&i| oag_physics::pilot_assist::Level::ALL.get(usize::from(i)))
        .copied()
        .unwrap_or_default()
}

/// The zone a Zone race ends at, or `None` for the original's own Zone (and in any other mode).
fn zone_target(params: &RaceParams) -> Option<u16> {
    let zone = oag_race::Mode::from_name(&params.mode) == Some(oag_race::Mode::Zone);
    (zone && params.zone_target > 0).then_some(u16::from(params.zone_target))
}

/// Hands the craft of every seat in `away` (a [`crate::wire::TickRecord::retired`] mask) to the
/// driver that flies a dropped person's seat: a person who left must not hold the race open
/// (`RaceSim::finished` counts only people), and the craft crawls round with its pickup kept
/// (`Race::set_away`). Both ends call it with the same mask at the same tick. **Chosen, not
/// measured**: the original has no network race.
pub fn retire(race: &mut Race, away: u8) {
    for slot in 0..oag_gameplay::MAX_PLAYERS.min(8) {
        if away >> slot & 1 == 1 {
            race.set_away(slot);
        }
    }
}

/// Hands every away seat missing from `away` back to its person: the countdown of
/// `Race::hand_back`, with the pickup held before the drop still held.
pub fn return_seats(race: &mut Race, away: u8) {
    for slot in 0..oag_gameplay::MAX_PLAYERS.min(8) {
        if away >> slot & 1 == 0 {
            race.hand_back(slot);
        }
    }
}

/// [`retire`] and [`return_seats`] for one tick's mask: the whole of a seat's presence, applied
/// before the tick runs on the host and on a client's confirmed state alike.
pub fn presence(race: &mut Race, away: u8) {
    retire(race, away);
    return_seats(race, away);
    #[cfg(feature = "test-cheats")]
    staged_wreck(race);
}

/// A wreck a test stages, on every peer alike, so that a race one person leaves by wreck is
/// reached on purpose: `OAG_TEST_WRECK=<slot>` destroys that craft at
/// [`crate::host::CHEAT_AT`]. Part of [`presence`] because the host, a client's confirmed state
/// and the referee all pass through it before a tick. Built only with `test-cheats`.
#[cfg(feature = "test-cheats")]
fn staged_wreck(race: &mut Race) {
    if race.core.sim.world.tick != u64::from(crate::host::CHEAT_AT) {
        return;
    }
    let slot = std::env::var("OAG_TEST_WRECK")
        .ok()
        .and_then(|slot| slot.parse::<usize>().ok());
    if let Some(ship) = slot.and_then(|slot| race.core.sim.world.ships.get_mut(slot)) {
        ship.physics.craft_state = oag_physics::CraftState::Destroyed;
        ship.physics.state_timer = oag_physics::damage::DESTROYED_DURATION;
    }
}

/// The state hash a prepared race starts from: what the client must reproduce to
/// join.
#[must_use]
pub fn fingerprint(race: &Race) -> u64 {
    race.core.sim.state_hash()
}

/// Loads, starts and prepares the race `params` names, from `source`, and names
/// the title it opened. The server's path; the windowed client builds its race
/// through the game's own load (with [`options`]) and then calls [`prepare`].
///
/// The race is the simulation half on a headless view that carries the launch request a
/// recording writes, whether or not this build links a renderer: nothing here draws, and the
/// state hash is the same either way (`oag_raceplay`'s `view_independence_ground_truth`).
///
/// # Errors
///
/// Whatever `oag_raceplay::load_sim` refuses, and a title other than `params.title`
/// when that is not empty.
pub fn load(params: &RaceParams, source: &str) -> Result<(Box<Race>, &'static str)> {
    let options = options(params, source)?;
    let loaded = oag_raceplay::load_sim(&options).with_context(|| format!("loading {source}"))?;
    check_title(params, loaded.title.name)?;
    let title = loaded.title.name;
    // Boxed where it is made: a `Race` is 141 KB, and moved by value up an
    // unoptimised call chain its copies overflowed a test thread's 2 MiB stack.
    let view = oag_raceplay::ViewSetup::headless_on(&options, &loaded.circuit);
    let mut race = Box::new(Race::start(loaded.setup, view, loaded.modules));
    prepare(&mut race, params)?;
    Ok((race, title))
}

/// Refuses a title other than the server's, by name.
///
/// # Errors
///
/// The two titles differ.
pub fn check_title(params: &RaceParams, opened: &str) -> Result<()> {
    if !params.title.is_empty() && params.title != opened {
        return Err(anyhow!(
            "the server races {:?} and this source is {opened:?}",
            params.title
        ));
    }
    Ok(())
}

/// The race's final board in the wire's shape: the rows of `race.results()` with each person's
/// name from `names` (by slot, `None` or empty for a craft the AI flew). Empty before the
/// race has a result. The server sends this; a client builds its own the same way from its
/// confirmed state, and [`Board::same_race_as`] compares the two.
#[must_use]
pub fn final_board(race: &Race, names: &[Option<String>]) -> Board {
    let Some(board) = race.core.results() else {
        return Board::default();
    };
    let ships = &race.core.sim.world.ships;
    Board {
        rows: board
            .rows
            .iter()
            .map(|row| {
                let slot = usize::from(row.slot);
                BoardRow {
                    place: row.place,
                    slot: row.slot,
                    name: if row.player {
                        names.get(slot).cloned().flatten().unwrap_or_default()
                    } else {
                        String::new()
                    },
                    laps: u16::try_from(row.laps_completed).unwrap_or(u16::MAX),
                    finish_ticks: row
                        .finish_tick
                        .map(|t| u32::try_from(t).unwrap_or(u32::MAX - 1)),
                    best_lap_ticks: row.best_lap_ticks,
                    eliminations: ships.get(slot).map_or(0, |ship| {
                        u16::try_from(ship.standing.kills).unwrap_or(u16::MAX)
                    }),
                }
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Laps reach the race only in a mode that ends on laps: Speed Lap, Zone and Eliminator
    /// would end a race the original never ends.
    #[test]
    fn laps_are_set_only_for_a_mode_that_ends_on_laps() {
        use oag_race::Mode;
        assert_eq!(lap_count(Mode::SingleRace, 1), Some(1));
        assert_eq!(lap_count(Mode::SingleRace, 0), None);
        for mode in [Mode::SpeedLap, Mode::Zone, Mode::Eliminator] {
            assert_eq!(lap_count(mode, 5), None, "{mode:?}");
        }
    }

    /// Every person flies their own team: seat 0 is the race's team, the rest are `seat_picks`
    /// in slot order, a seat that named none flies the default, and no seat is skipped.
    #[test]
    fn each_seat_flies_its_own_team() {
        let params = RaceParams {
            mode: "single_race".into(),
            difficulty: "elite".into(),
            scheme: "veteran".into(),
            team: "default".into(),
            teams: vec!["a".into(), String::new(), "c".into()],
            people: 3,
            ..RaceParams::default()
        };
        let options = options(&params, "x").expect("options");
        assert_eq!(options.team.as_deref(), Some("a"));
        let picks: Vec<&str> = options
            .seat_picks
            .iter()
            .map(|pick| pick.team.as_str())
            .collect();
        assert_eq!(picks, ["default", "c"]);
        // One person: no picks. No default and no choice: the title's own draw.
        let alone = RaceParams {
            people: 1,
            teams: Vec::new(),
            team: String::new(),
            ..params
        };
        let options = super::options(&alone, "x").expect("options");
        assert!(options.seat_picks.is_empty() && options.team.is_none());
    }
}
