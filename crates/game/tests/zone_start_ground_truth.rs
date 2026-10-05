//! A Zone craft's start: it stands on the grid through the countdown.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

fn image(name: &str) -> Option<std::path::PathBuf> {
    oag_testdata::image(name)
}

/// A Zone craft sits on the grid through the countdown and moves from GO.
///
/// Read live on a Zone engine, 2026-10-02: `craft+0x1c0` is `0x3` through state 0
/// and Zone's auto-speed is gated on `!(flags & 2)`, so the craft stood at
/// `0.02` units/s until the first state-1 frame and then accelerated.
/// This port's Zone craft used to drift through the countdown at about 98 km/h
/// before GO.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_zone_craft_stands_still_through_the_countdown_and_moves_after_go() {
    let Some(path) = image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Zone,
        ..race::Options::default()
    })
    .expect("loading the zone race");

    let mut zone = race::Race::start(loaded.setup);
    let start = zone.ship().physics.body.position;
    let mut furthest_before_go = 0.0_f32;
    let go = oag_race::COUNTDOWN_TICKS;
    for _ in 0..go {
        zone.tick(&PlayerInputs::none());
        furthest_before_go =
            furthest_before_go.max((zone.ship().physics.body.position - start).length());
    }
    assert!(
        furthest_before_go < 0.5,
        "a Zone craft moved {furthest_before_go} units through the countdown"
    );
    for _ in 0..120 {
        zone.tick(&PlayerInputs::none());
    }
    let moved = (zone.ship().physics.body.position - start).length();
    assert!(
        moved > 20.0,
        "a Zone craft only moved {moved} units in 2 s after GO"
    );
}
