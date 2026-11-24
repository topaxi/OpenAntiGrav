//! The airbrake flaps on the grid: both fully out on every craft from the race's load until the
//! green light, then folding back at the hull's own `down_speed`
//! (`docs/physics/grid-state.md`, "The flaps on the grid").

use super::*;
use oag_gameplay::PlayerInputs;

/// Every active slot's two drawn flap levels, `0..=100`.
fn levels(race: &Race) -> Vec<(usize, [f32; 2])> {
    (0..race.core.sim.world.ship_count as usize)
        .filter(|&slot| race.core.sim.world.ships[slot].active)
        .map(|slot| (slot, race.view.flaps[slot]))
        .collect()
}

/// One craft at the grid and no more would not show the AI's: the fixture fields eight.
fn a_field() -> Race {
    let race = race_with_a_grid();
    assert!(levels(&race).len() > 1, "the fixture fields opponents");
    race
}

#[test]
fn every_craft_holds_its_flaps_full_from_the_load_until_the_green_light() {
    let mut race = a_field();
    for (slot, flaps) in levels(&race) {
        assert_eq!(flaps, [100.0; 2], "slot {slot} at the load");
        assert!(
            race.airbrake_flaps_of(slot)[0] > 0.0,
            "slot {slot} draws them out at the load"
        );
    }
    for tick in 0..oag_race::COUNTDOWN_TICKS - 1 {
        race.tick(&PlayerInputs::none());
        for (slot, flaps) in levels(&race) {
            assert_eq!(flaps, [100.0; 2], "slot {slot} after tick {tick}");
        }
    }
}

#[test]
fn after_the_green_light_every_craft_folds_at_the_hulls_down_speed() {
    let mut race = a_field();
    for _ in 0..oag_race::COUNTDOWN_TICKS - 1 {
        race.tick(&PlayerInputs::none());
    }
    let step = race.view.flap_graphics.down_speed * race.core.sim.dt;
    assert!(step > 0.0, "the hull authors a down_speed");
    let mut expected = 100.0_f32;
    let mut ticks = 0;
    while expected > 0.0 {
        race.tick(&PlayerInputs::none());
        expected = (expected - step).max(0.0);
        ticks += 1;
        for (slot, flaps) in levels(&race) {
            assert_eq!(
                flaps, [expected; 2],
                "slot {slot}, {ticks} tick(s) after GO"
            );
        }
    }
    assert_eq!(
        ticks,
        (100.0 / step).ceil() as i32,
        "folded in the time the rate gives"
    );
}

#[test]
fn a_posed_capture_stows_the_grid_flaps_and_they_stay_stowed() {
    let mut race = a_field();
    race.stow_grid_flaps();
    for (slot, flaps) in levels(&race) {
        assert_eq!(flaps, [0.0; 2], "slot {slot} stowed at once");
    }
    for tick in 0..oag_race::COUNTDOWN_TICKS {
        race.tick(&PlayerInputs::none());
        for (slot, flaps) in levels(&race) {
            assert_eq!(flaps, [0.0; 2], "slot {slot} after tick {tick}");
        }
    }
}

/// The flaps are drawing state: neither forcing them nor stowing them reaches the state hash.
#[test]
fn the_grid_flaps_reach_no_state_hash() {
    let run = |stowed: bool| {
        let mut race = a_field();
        if stowed {
            race.stow_grid_flaps();
        }
        for _ in 0..oag_race::COUNTDOWN_TICKS + 30 {
            race.tick(&PlayerInputs::none());
        }
        race.core.sim.state_hash()
    };
    assert_eq!(run(false), run(true));
}

/// What the flaps of a craft touching nothing draw after `tick` ticks of a race: out until GO,
/// then folding at the hull's `down_speed`.
fn flap_level_at(race: &Race, tick: u64) -> f32 {
    let step = race.view.flap_graphics.down_speed * race.core.sim.dt;
    if tick < oag_race::COUNTDOWN_TICKS {
        100.0
    } else {
        (100.0 - step * (tick - (oag_race::COUNTDOWN_TICKS - 1)) as f32).max(0.0)
    }
}

/// Every active slot's flaps are `level`.
fn assert_flaps(race: &Race, level: f32, what: &str) {
    for (slot, flaps) in levels(race) {
        assert!(
            (flaps[0] - level).abs() < 1e-3 && flaps[0] == flaps[1],
            "{what}: slot {slot} draws {flaps:?}, not {level}"
        );
    }
}

/// A race run to `ticks` and a snapshot of its simulation at each of `at`.
fn run_with_snapshots(ticks: u64, at: &[u64]) -> (Race, Vec<(u64, crate::RaceSim)>) {
    let mut race = a_field();
    let mut kept = Vec::new();
    for tick in 0..=ticks {
        if at.contains(&tick) {
            kept.push((tick, race.snapshot_sim()));
        }
        if tick < ticks {
            race.tick(&PlayerInputs::none());
        }
    }
    (race, kept)
}

const PAST_THE_FOLD: u64 = oag_race::COUNTDOWN_TICKS + 260;

/// A viewer's seek puts the simulation at another tick without running the ticks between: the
/// flaps follow it, back into the grid and forward through the fold.
#[test]
fn a_seek_across_go_draws_the_flaps_of_the_tick_it_lands_on() {
    let (mut race, kept) =
        run_with_snapshots(PAST_THE_FOLD, &[0, 150, 300, 330, 400, PAST_THE_FOLD]);
    assert_flaps(&race, 0.0, "after the fold");
    for &(tick, ref snapshot) in kept.iter().rev().chain(&kept) {
        race.restore_for_viewing(snapshot);
        assert_flaps(
            &race,
            flap_level_at(&race, tick),
            &format!("seek to {tick}"),
        );
    }
    // And forward from the grid, the other way round.
    let (mut race, kept) = run_with_snapshots(100, &[]);
    assert_flaps(&race, 100.0, "on the grid");
    let (_, past) = run_with_snapshots(PAST_THE_FOLD, &[PAST_THE_FOLD]);
    race.restore_for_viewing(&past[0].1);
    assert_flaps(&race, 0.0, "seek forward past the fold");
    drop(kept);
}

/// A posed view of a replay whose build stopped on the grid draws the flaps of the tick it
/// shows, not the grid's for ever.
#[test]
fn a_pose_view_draws_the_flaps_of_the_time_it_shows() {
    let mut recorder = oag_replay::PoseRecorder::new(0);
    assert!(recorder.push(&[]));
    let track = recorder.track().expect("a track");
    let mut race = a_field();
    assert_flaps(&race, 100.0, "on the grid");
    race.show_poses(&track, PAST_THE_FOLD as f64);
    assert_flaps(&race, 0.0, "a pose view past the fold");
    race.show_poses(&track, 100.0);
    assert_flaps(&race, 100.0, "a pose view on the grid");
}

/// A client catching up on confirmed ticks advances no view, and a snap back to the server's
/// state restores the simulation alone: neither may leave the flaps as the grid had them.
#[test]
fn confirmed_ticks_and_a_snap_mid_race_do_not_leave_the_grid_flaps_out() {
    let (reference, kept) = run_with_snapshots(PAST_THE_FOLD, &[PAST_THE_FOLD]);
    drop(reference);
    let mut client = a_field();
    for _ in 0..PAST_THE_FOLD {
        client.tick_sim_only(&PlayerInputs::none());
    }
    assert_flaps(&client, 0.0, "after confirmed ticks past the fold");

    let mut client = a_field();
    client.restore_sim(&kept[0].1);
    assert_flaps(&client, 0.0, "a snap to the server's state mid-race");
    // A correction that rewinds a few ticks mid-race moves nothing.
    let mid_flap = client.view.flaps;
    client.restore_sim(&kept[0].1);
    assert_eq!(client.view.flaps, mid_flap);
}
