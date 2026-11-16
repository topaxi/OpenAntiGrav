//! Pulse's `Ghost Ship Visible` reaches the picture: with it off, a race that is
//! racing a ghost reports no ghost to draw, and with it on the same race does.

use oag_core::math::{Quat, Vec3};
use oag_gameplay::PlayerInputs;
use oag_replay::{GhostLap, Pose};

/// A ghost lap standing still at the origin: the draw path asks `ghost_pose` for a pose, and
/// only whether one comes back matters here.
fn standing_ghost() -> oag_raceplay::Ghost {
    let lap_ticks = 6_000;
    oag_raceplay::Ghost {
        lap: GhostLap {
            lap_ticks,
            poses: vec![
                Pose {
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                };
                lap_ticks as usize + 1
            ],
        },
        team: "default".to_string(),
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn a_hidden_ghost_is_raced_but_reports_no_pose_to_draw() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        seed: Some(1),
        opponents: false,
        ..oag_raceplay::Options::default()
    })
    .expect("loading the race");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.set_ghost(standing_ghost());
    // Flown by the autopilot, so the lap clock starts at the line.
    race.set_autopilot(true);
    let input = PlayerInputs::none();
    let mut visible_ticks = 0;
    let mut hidden_ticks = 0;
    for tick in 0..2_400 {
        race.tick(&input);
        race.drain_cues();
        race.set_ghost_visible(true);
        visible_ticks += usize::from(race.ghost_pose().is_some());
        race.set_ghost_visible(false);
        hidden_ticks += usize::from(race.ghost_pose().is_some());
        assert!(race.ghost().is_some(), "tick {tick}: still raced");
    }
    assert!(
        visible_ticks > 0,
        "the ghost was on the circuit while shown"
    );
    assert_eq!(hidden_ticks, 0, "never reported while hidden");
}
