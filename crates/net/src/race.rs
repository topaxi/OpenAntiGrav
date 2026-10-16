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
//! - **Weapons off**: weapons are not server events yet (ADR-0069, condition 2).
//! - **Every person's slot is [`Controller::Remote`] on every peer**, the client's
//!   own included, because the controllers are hashed. Which slot is this machine's
//!   is view-side: `Race::set_seat`.

use anyhow::{Context, Result, anyhow};
use oag_gameplay::Controller;
use oag_raceplay::{Options, Race};

use crate::wire::RaceParams;

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
        no_driver_for_class: !oag_ai::coverage::project_driver_covers(&params.class),
        server_source: true,
        ..Options::default()
    })
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
    for slot in 0..usize::from(params.people).min(oag_gameplay::MAX_PLAYERS) {
        race.sim.world.controllers[slot] = Controller::Remote;
    }
    race.set_control_scheme(scheme(params)?);
    race.set_autopilot(false);
    race.set_pilot_assist(oag_physics::pilot_assist::Level::Off);
    Ok(())
}

/// The state hash a prepared race starts from: what the client must reproduce to
/// join.
#[must_use]
pub fn fingerprint(race: &Race) -> u64 {
    race.sim.state_hash()
}

/// Loads, starts and prepares the race `params` names, from `source`, and names
/// the title it opened. The server's path; the windowed client builds its race
/// through the game's own load (with [`options`]) and then calls [`prepare`].
///
/// # Errors
///
/// Whatever `oag_raceplay::load` refuses, and a title other than `params.title`
/// when that is not empty.
pub fn load(params: &RaceParams, source: &str) -> Result<(Box<Race>, &'static str)> {
    let options = options(params, source)?;
    let loaded = oag_raceplay::load(&options).with_context(|| format!("loading {source}"))?;
    check_title(params, loaded.title.name)?;
    let title = loaded.title.name;
    // Boxed where it is made: a `Race` is 141 KB, and moved by value up an
    // unoptimised call chain its copies overflowed a test thread's 2 MiB stack.
    let mut race = Box::new(Race::start(loaded.setup));
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

#[cfg(test)]
mod tests {
    use super::*;

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
