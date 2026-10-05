//! The perfect start's cue: `TURBO`, raised once on a perfect-window launch.
//! See `race::perfect_start`.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_sound::sfx::Cue;

/// A time trial with a launch window whose player first thrusts inside the
/// perfect window: what `ExhaustFlare_OnPerfectStart` (`0x08904fd4`) raises
/// `TURBO` on. The disc-backed half, with the disc's own window, is
/// `launch_boost_ground_truth::only_a_perfect_start_fires_the_flare_and_turbo`.
pub(super) fn perfect_start() -> Vec<Cue> {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::TimeTrial;
    setup.start_boost = Some(oag_physics::launch::StartBoost {
        window_start: 0.1,
        window_end: 0.35,
        stall_end: 0.45,
        overall_duration: 1.0,
        stall_mul: 1.2,
        normal_mul: 1.4,
        boost_mul: 1.6,
    });
    let mut race = Race::start(setup);
    let mut buttons = Buttons::new();
    let mut raised = Vec::new();
    for tick in 0..oag_race::COUNTDOWN_TICKS + 40 {
        let press = tick >= oag_race::COUNTDOWN_TICKS + 12;
        race.tick(&PlayerInputs::single(buttons.tick(if press {
            CROSS
        } else {
            0
        })));
        raised.extend(race.drain_cues().into_iter().map(|e| e.cue));
    }
    raised
}

#[test]
fn a_perfect_start_raises_turbo_once_and_a_held_one_does_not() {
    let raised = perfect_start();
    assert_eq!(
        raised.iter().filter(|&&c| c == Cue::Turbo).count(),
        1,
        "{raised:?}"
    );
}
