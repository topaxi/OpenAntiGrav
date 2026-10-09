//! HD's zoom-streak ring on the race's side: the player's own speed pad fires the
//! boost pulse, nobody else's does, and a title with no ring has none.

use super::*;
use oag_gameplay::PlayerInputs;

/// [`race_with_a_grid`]'s pad fixture with HD's ring switched on or off.
fn grid_on_a_speed_pad(ring: bool) -> Race {
    let mut handling = hulled_handling();
    handling.speedup_pads = oag_physics::params::SpeedupPads {
        amount: 37.0,
        time: 0.5,
    };
    let mut setup = setup(handling);
    setup.mode = Mode::SingleRace;
    setup.speedup_pads = enveloping_pad();
    setup.zoom_ring = ring.then_some(&oag_hd::race::ZOOM_RING);
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    Race::start(setup)
}

/// The pad every craft crosses fires the *player's* pulse, which then runs the
/// original's law: a ramp to 1 inside a fifth of a second, a hold, and over by
/// 1.3 s - stepped by ticks, so the frame carries a tick count that moves.
#[test]
fn the_players_speed_pad_runs_the_boost_pulse() {
    let mut race = grid_on_a_speed_pad(true);
    assert_eq!(
        race.hd_zoom_frame().map(|f| f.boost),
        Some(0.0),
        "quiet at the start"
    );
    let mut peak = 0.0_f32;
    let mut at_peak = 0;
    for tick in 0..90 {
        race.tick(&PlayerInputs::none());
        let boost = race.hd_zoom_frame().expect("a ring").boost;
        if boost > peak {
            peak = boost;
            at_peak = tick;
        }
    }
    assert!((peak - 1.0).abs() < 1e-6, "the pulse peaked at {peak}");
    assert!(at_peak < 20, "the ramp took {at_peak} ticks");
    let frame = race.hd_zoom_frame().expect("a ring");
    assert_eq!(frame.boost, 0.0, "1.5 s later the pulse is over");
    assert!(
        frame.tick >= 90,
        "the frame names the tick it was stepped for"
    );
}

/// A title with no ring has no pulse to read and nothing to fire.
#[test]
fn a_title_without_the_ring_has_no_pulse() {
    let mut race = grid_on_a_speed_pad(false);
    for _ in 0..10 {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(race.hd_zoom_frame(), None);
}
