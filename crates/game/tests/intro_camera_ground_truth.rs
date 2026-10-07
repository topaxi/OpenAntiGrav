//! The pre-race flyby on a real circuit, played through a `Race`.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test intro_camera_ground_truth --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! The original plays `start_grid.vex`'s camera animation before the countdown, for `AnimEnd`
//! seconds or until a held Cross (`docs/gameplay/race-intro.md`). Talon's Junction's is `1500`
//! keys, 25.0 s. This proves the loader reads it, that the world is not stepped meanwhile, that
//! the run ends on the tick the measured constants say, that the HUD comes back 30 ticks after,
//! and that a held Cross ends it when the lock lifts and not before.

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input, InputSnapshot};
use oag_raceplay as race;
use oag_raceplay::Race;

const TRACK: &str = r"Data\Environments\16_Track\track.vex";

fn started() -> Option<Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(TRACK.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.starts_with("pre-race flyby: ") && line.contains("25.00 s")),
        "the loader reports a 25 s flyby: {:?}",
        loaded.report
    );
    Some(Race::start(loaded.setup))
}

fn cross() -> PlayerInputs {
    let mut buttons = Input::EMPTY;
    buttons.begin_frame(Button::Cross.bit());
    PlayerInputs::single(InputSnapshot {
        buttons,
        ..InputSnapshot::EMPTY
    })
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_flyby_runs_its_course_and_leaves_the_world_alone() {
    let Some(mut race) = started() else { return };
    assert!(
        race.hud_shown(),
        "before it begins the race draws as always"
    );
    assert!(race.begin_intro());
    assert!(!race.hud_shown());
    let before = race.sim.state_hash();
    let mut ticks = 0u32;
    while race.in_intro() {
        let key = race.motion_tick();
        race.tick_intro(&PlayerInputs::none());
        ticks += 1;
        assert_eq!(
            race.motion_tick(),
            key + 1,
            "the blur's snapshot must promote on every flyby tick while the world is held"
        );
        assert!(ticks < 3000, "the flyby never ended");
    }
    // The tick that ends it is the one that already shows the chase camera.
    assert_eq!(
        ticks,
        oag_pulse::pre_race::PRE_RACE.hold_ticks.value + 1500 + 1
    );
    assert_eq!(race.sim.state_hash(), before, "the flyby stepped the world");
    assert_eq!(race.sim.world.tick, 0);
    assert!(!race.hud_shown(), "the HUD waits out the fade");
    for _ in 0..u64::from(oag_pulse::pre_race::PRE_RACE.hud_delay_ticks.value) {
        assert!(!race.hud_shown());
        race.tick(&PlayerInputs::none());
    }
    assert!(race.hud_shown());
    assert!(
        race.camera_cuts() >= 3,
        "the cuts at 360, 720 and 1080 are counted"
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_held_cross_ends_it_when_the_lock_lifts_and_not_before() {
    let Some(mut race) = started() else { return };
    assert!(race.begin_intro());
    let mut ticks = 0u32;
    while race.in_intro() {
        race.tick_intro(&cross());
        ticks += 1;
        assert!(ticks < 3000, "a held cross did not end the flyby");
    }
    assert_eq!(ticks, oag_pulse::pre_race::PRE_RACE.lock_ticks.value + 1);
}
