//! Wipeout HD's pre-race flyby on a real circuit, played through a `Race`.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test hd_flyby_ground_truth --run-ignored all
//! ```
//!
//! HD opens a race on a fly-over with a `START RACE` prompt and one tap of cross skips it
//! (`docs/gameplay/race-intro.md`). The camera is the circuit's `start_grid.vex`; its loop
//! length is the file's own (173.33 s on Vineta K). This proves the loader reads it through the
//! title's `PreRace` data, that the flyby does not end by itself, that a *held* cross does not
//! skip it but a *press* does, and that the world is not stepped meanwhile.

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input, InputSnapshot};
use oag_raceplay as race;
use oag_raceplay::Race;

const TRACK: &str = r"Data\Environments\Talons_Junction\track.vex";

fn started(track: &str) -> Option<(Race, Vec<String>)> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let report = loaded.report.clone();
    Some((Race::start(loaded.setup), report))
}

fn cross(down: bool) -> PlayerInputs {
    let mut buttons = Input::EMPTY;
    buttons.begin_frame(if down { Button::Cross.bit() } else { 0 });
    PlayerInputs::single(InputSnapshot {
        buttons,
        ..InputSnapshot::EMPTY
    })
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_loader_reads_the_circuits_own_flyby() {
    let Some((_, report)) = started(TRACK) else {
        return;
    };
    assert!(
        report
            .iter()
            .any(|line| line.starts_with("pre-race flyby: ")
                && line.contains("start_grid.vex")
                && line.contains("188.33 s")),
        "{report:?}"
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn it_loops_until_a_press_and_a_held_cross_alone_does_not_skip_it() {
    let Some((mut race, _)) = started(TRACK) else {
        return;
    };
    let before = race.sim.state_hash();
    assert!(race.begin_intro());
    // Past the 188 s of animation: a title that ended at `AnimEnd` would be over by 11,300.
    for _ in 0..12_000 {
        race.tick_intro(&cross(true));
        assert!(race.in_intro(), "a held cross must not skip HD's flyby");
    }
    race.tick_intro(&cross(false));
    race.tick_intro(&cross(true));
    assert!(!race.in_intro(), "a press ends it");
    assert_eq!(race.sim.state_hash(), before, "the flyby stepped the world");
    assert_eq!(race.sim.world.tick, 0);
}
