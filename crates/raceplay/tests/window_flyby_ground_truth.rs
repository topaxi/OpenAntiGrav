//! A multi-window race plays the pre-race flyby in every window that holds a seat, and each
//! window goes back to its own seats when it ends. Disc-backed: the flyby is the circuit's
//! own camera animation, which no fixture has.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay window_flyby_ground_truth:: --run-ignored all
//! ```

use oag_gameplay::PlayerInputs;
use oag_raceplay::{self as race, WindowViews};
use oag_render::viewport::Cut;

fn seats(slots: &[usize]) -> WindowViews {
    WindowViews {
        seats: slots.to_vec(),
        cut: Cut::Horizontal,
    }
}

/// Window 1 holds the player and one more, window 2 holds two others, and a fourth window
/// holds nobody. The flyby is the player's camera in the first two, whole; the last has no
/// one to show it to. The split is each window's own when the flyby ends. Fails if the other
/// windows draw their chase cameras through the flyby (the build before this one) or if a
/// window with no seat starts drawing one.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_window_plays_the_flyby_and_then_its_own_seats() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        views: 4,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    assert_eq!(race.set_viewport_count(4), 4);
    race.set_windows(vec![seats(&[0, 1]), seats(&[2, 3]), seats(&[])]);
    assert_eq!(race.window_drawn(0), vec![0, 1], "no flyby yet");
    assert_eq!(race.window_drawn(1), vec![2, 3], "no flyby yet");

    assert!(race.begin_intro(), "a Pulse single race opens on its flyby");
    let player = race.player_slot();
    let area = (0.0, 0.0, 1280.0, 720.0);
    let mut ticks = 0;
    while race.in_intro() {
        for window in 0..2 {
            assert_eq!(
                race.window_drawn(window),
                vec![player],
                "window {} plays the player's flyby",
                window + 1
            );
            assert_eq!(
                race.window_rects(window, area),
                vec![area],
                "the flyby fills window {}",
                window + 1
            );
        }
        assert!(race.window_drawn(2).is_empty(), "a window with no seat");
        assert!(race.window_drawn(3).is_empty(), "no such window");
        race.tick_intro(&PlayerInputs::none());
        ticks += 1;
        assert!(ticks < 60 * 60, "the flyby ends by itself");
    }
    assert!(ticks > 0);
    assert_eq!(race.window_drawn(0), vec![0, 1], "window 1 splits again");
    assert_eq!(race.window_drawn(1), vec![2, 3], "window 2 gets its seats");
    assert_eq!(race.window_rects(1, area).len(), 2);
}

/// A skip ends the flyby for every window together, because it is one race state and not a
/// window's own.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_skip_ends_the_flyby_in_every_window_at_once() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        views: 2,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    assert_eq!(race.set_viewport_count(2), 2);
    race.set_windows(vec![seats(&[0]), seats(&[1])]);
    assert!(race.begin_intro());
    assert_eq!(race.window_drawn(1), vec![race.player_slot()]);
    race.skip_intro();
    let mut ticks = 0;
    while race.in_intro() {
        race.tick_intro(&PlayerInputs::none());
        ticks += 1;
        assert!(ticks < 60 * 60, "a skip ends the flyby");
    }
    assert_eq!(race.window_drawn(0), vec![0]);
    assert_eq!(race.window_drawn(1), vec![1], "window 2 shows its own seat");
}
