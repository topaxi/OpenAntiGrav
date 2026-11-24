//! The airbrake flaps on the grid, on Pulse's own hulls: both fully out on every craft until the
//! green light, then folded back at the team's authored `down_speed` (`docs/physics/grid-state.md`,
//! "The flaps on the grid"). The original's flap words, read live on Assegai, run `100` to `0`
//! linearly over 59 frames after GO; Assegai authors `down_speed` `100`, which is 60 ticks.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

fn single_race(team: &str) -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        team: Some(team.to_string()),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(race::Race::start(loaded.setup, loaded.view, loaded.modules))
}

fn active_slots(race: &race::Race) -> Vec<usize> {
    (0..usize::from(race.core.sim.ship_count()))
        .filter(|&slot| race.core.sim.ship_active(slot))
        .collect()
}

/// The player's flaps, drawn, from the load through the countdown and the fold: the radians at
/// the load, then one value per tick from the last gated tick until they are back at rest.
fn player_fold(team: &str) -> Option<(f32, Vec<f32>)> {
    let mut race = single_race(team)?;
    let full = race.airbrake_flaps_of(0)[0];
    assert!(full > 0.0, "{team}: drawn out at the load");
    let mut fold = Vec::new();
    for tick in 0..oag_race::COUNTDOWN_TICKS + 300 {
        race.tick(&PlayerInputs::none());
        let drawn = race.airbrake_flaps_of(0);
        assert_eq!(drawn[0], drawn[1], "{team}: both flaps together");
        if tick < oag_race::COUNTDOWN_TICKS - 1 {
            assert_eq!(
                drawn[0], full,
                "{team}: out through the countdown, tick {tick}"
            );
        } else {
            fold.push(drawn[0]);
            if drawn[0] < full * 1e-4 {
                break;
            }
        }
    }
    Some((full, fold))
}

#[test]
fn assegai_folds_linearly_in_the_original_s_59_frames() {
    let Some((full, fold)) = player_fold("ASSEGAI") else {
        return;
    };
    let step = full / 60.0;
    for (tick, drawn) in fold.iter().enumerate() {
        let want = (full - step * (tick + 1) as f32).max(0.0);
        assert!(
            (drawn - want).abs() < full * 1e-4,
            "tick {tick} after the last gated one: {drawn} against {want}"
        );
    }
    assert!(
        (59..=60).contains(&fold.len()),
        "100 to 0 in the original's 59 frames: {} ticks",
        fold.len()
    );
}

#[test]
fn every_craft_of_a_field_is_out_on_the_grid_and_folded_by_the_start() {
    let Some(mut race) = single_race("ASSEGAI") else {
        return;
    };
    let slots = active_slots(&race);
    assert!(slots.len() > 1, "a single race fields opponents");
    for &slot in &slots {
        assert!(
            race.airbrake_flaps_of(slot)[0] > 0.0,
            "slot {slot} at the load"
        );
    }
    for _ in 0..oag_race::COUNTDOWN_TICKS - 1 {
        race.tick(&PlayerInputs::none());
    }
    let out: Vec<f32> = slots
        .iter()
        .map(|&s| race.airbrake_flaps_of(s)[0])
        .collect();
    assert!(out.iter().all(|&flap| flap > 0.0), "{out:?}");
    // Past the slowest team's fold: every hull authors a `down_speed` of at least 80.
    for _ in 0..90 {
        race.tick(&PlayerInputs::none());
    }
    for &slot in &slots {
        assert_eq!(race.airbrake_flaps_of(slot), [0.0; 2], "slot {slot} folded");
    }
}
