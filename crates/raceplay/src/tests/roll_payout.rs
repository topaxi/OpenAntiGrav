//! A landed barrel roll lights the flare and raises `TURBO`, once, on the tick
//! the payout is armed. See `race::roll_payout`.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_gameplay::cue::Cue;

/// A race whose player has completed a roll in the air above the grid, falling
/// onto it. `turbotime` is `<Special roll_turbotime>`.
fn rolled_and_falling(turbotime: f32) -> Race {
    let mut handling = hulled_handling();
    handling.roll_turbotime = turbotime;
    let mut race = Race::start(setup_with(
        handling,
        vec![upward_plane(0.0, oag_physics::Surface::Floor)],
    ));
    // Past the countdown: the grid holds a craft in place until the green light.
    for _ in 0..oag_race::COUNTDOWN_TICKS + 2 {
        race.tick(&PlayerInputs::none());
    }
    race.drain_cues();
    let ship = &mut race.sim.world.ships[0].physics;
    ship.body.position.y += 40.0;
    ship.body.linear_velocity.y = -40.0;
    ship.grounded = 0.0;
    ship.grounded_prev = 0.0;
    ship.roll_armed = true;
    ship.roll_phase = 1.0;
    ship.roll_target = 1.0;
    race
}

/// Ticks until the payout arms, returning the cues of that tick and the HUD's
/// boost phase, which the boost copies and the speed bar's glow draw from.
fn until_paid(race: &mut Race) -> Option<(Vec<Cue>, f32)> {
    for _ in 0..600 {
        race.tick(&PlayerInputs::none());
        let cues: Vec<Cue> = race.drain_cues().into_iter().map(|e| e.cue).collect();
        if race.sim.world.ships[0].physics.roll_payout_timer > 0.0 {
            return Some((cues, race.readout().boost_phase));
        }
    }
    None
}

#[test]
fn landing_a_completed_roll_arms_the_flare_and_raises_turbo_once() {
    let mut race = rolled_and_falling(0.5);
    let (cues, flare) = until_paid(&mut race).expect("the roll never paid out");
    assert!(
        flare > 0.7,
        "the flare was not armed on the payout tick: {flare}"
    );
    assert_eq!(
        cues.iter().filter(|&&c| c == Cue::Turbo).count(),
        1,
        "{cues:?}"
    );
    for _ in 0..90 {
        race.tick(&PlayerInputs::none());
        assert!(
            !race.drain_cues().iter().any(|e| e.cue == Cue::Turbo),
            "the payout re-fired while it ran"
        );
    }
}

#[test]
fn a_roll_that_pays_nothing_raises_no_turbo() {
    let mut race = rolled_and_falling(0.0);
    for _ in 0..300 {
        race.tick(&PlayerInputs::none());
        assert!(!race.drain_cues().iter().any(|e| e.cue == Cue::Turbo));
    }
}
