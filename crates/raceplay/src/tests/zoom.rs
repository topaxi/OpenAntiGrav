//! HD's zoom-streak ring on the race's side: the player's own speed pad fires the
//! boost pulse, nobody else's does, and a title with no ring has none.

use super::*;
use oag_gameplay::PlayerInputs;

/// [`race_with_a_grid`]'s pad fixture with HD's ring switched on or off.
fn grid_on_a_speed_pad(ring: bool) -> Race {
    grid_on_a_pad_with(ring, false)
}

/// The same fixture with the ring HD's own (`borrowed` false) or borrowed by a
/// title that draws none (`borrowed` true), as `load` sets `Setup::zoom_borrowed`.
fn grid_on_a_pad_with(ring: bool, borrowed: bool) -> Race {
    let mut handling = hulled_handling();
    handling.speedup_pads = oag_physics::params::SpeedupPads {
        amount: 37.0,
        time: 0.5,
    };
    let mut setup = setup(handling);
    setup.mode = Mode::SingleRace;
    setup.speedup_pads = enveloping_pad();
    setup.zoom_ring = ring.then_some(&oag_hd::race::ZOOM_RING);
    setup.zoom_borrowed = ring && borrowed;
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

/// `graphics.boost_zoom` off: a Pulse-shaped race (no ring of its own, nothing
/// borrowed) fires nothing on a speed pad and offers the scene's host pass nothing
/// to build.
#[test]
fn a_title_with_the_setting_off_draws_no_borrowed_ring() {
    let mut race = grid_on_a_pad_with(false, false);
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(race.hd_zoom_frame(), None);
    assert_eq!(race.borrowed_zoom_tuning(), None);
}

/// `graphics.boost_zoom` on: the same race borrows HD's numbers, the player's speed
/// pad raises `E`, and the scene's host pass is handed that tuning (HD's own ring,
/// drawn in its bloom chain, is not).
#[test]
fn a_borrowed_ring_fires_on_the_players_speed_pad() {
    let mut race = grid_on_a_pad_with(true, true);
    let mut peak = 0.0_f32;
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
        peak = peak.max(race.hd_zoom_frame().expect("a ring").boost);
    }
    assert!(peak > 0.9, "the pad raised E to {peak}");
    assert_eq!(
        race.borrowed_zoom_tuning(),
        Some(super::super::zoom::tuning(&oag_hd::race::ZOOM_RING)),
    );
    let own = grid_on_a_pad_with(true, false);
    assert_eq!(
        own.borrowed_zoom_tuning(),
        None,
        "HD's own ring is not borrowed"
    );
}

/// Which titles may borrow the ring is title data: Pulse and Pure, no one else.
#[test]
fn only_pulse_and_pure_offer_the_borrowed_ring() {
    assert!(oag_pulse::TITLE.race.borrows_zoom_ring);
    assert!(oag_pure::TITLE.race.borrows_zoom_ring);
    assert!(!oag_hd::TITLE.race.borrows_zoom_ring);
    assert!(
        oag_hd::TITLE.race.zoom_ring.is_some(),
        "HD has a ring of its own"
    );
}
