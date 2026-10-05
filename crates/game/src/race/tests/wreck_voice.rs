//! `cont_elim`, the one sound an opponent's destruction makes in the original:
//! raised once per wreck, 2.3 s after the craft is down, in every mode but the
//! Eliminator, for the player never. `Race::tick_wreck_voice`.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_physics::CraftState;
use oag_sound::sfx::Cue;

/// The ticks `cont_elim` was raised on, with `down` slots set `Eliminated`
/// from the first tick.
fn raised(mode: Mode, voiced: bool, down: &[usize]) -> Vec<(u64, usize)> {
    let mut setup = setup(hulled_handling());
    setup.mode = mode;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    let mut race = Race::start(setup);
    race.sim.countdown_voice = voiced;
    for &slot in down {
        race.sim.world.ships[slot].physics.craft_state = CraftState::Eliminated;
    }
    let mut out = Vec::new();
    for _ in 0..400 {
        race.tick(&PlayerInputs::none());
        let stepped = race.sim.world.tick - 1;
        for event in race.drain_cues() {
            if event.cue == Cue::ContElim {
                out.push((stepped, usize::from(event.slot)));
            }
        }
    }
    out
}

#[test]
fn a_wrecked_opponent_raises_cont_elim_once_after_two_point_three_seconds() {
    let hits = raised(Mode::SingleRace, true, &[1, 2]);
    // Both wrecks, one cue each, on the same tick: 138 ticks of 1/60 s.
    assert_eq!(hits.len(), 2, "{hits:?}");
    assert_eq!(hits[0].0, hits[1].0);
    assert!((137..=139).contains(&hits[0].0), "{hits:?}");
}

#[test]
fn the_eliminator_and_an_unvoiced_title_raise_nothing() {
    assert!(raised(Mode::Eliminator, true, &[1]).is_empty());
    assert!(raised(Mode::SingleRace, false, &[1]).is_empty());
}

#[test]
fn the_player_and_a_standing_opponent_raise_nothing() {
    assert!(raised(Mode::SingleRace, true, &[0]).is_empty());
    assert!(raised(Mode::SingleRace, true, &[]).is_empty());
}
