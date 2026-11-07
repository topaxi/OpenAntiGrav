//! Wipeout HD's pre-race flyby on a real circuit, played through a `Race`.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay hd_flyby_ground_truth:: --run-ignored all
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
const TRACK_REVERSED: &str = r"Data\Environments\Talons_Junction\track_reversed.vex";
const ZONE_TRACK: &str = r"Data\Environments\zone_1\track.vex";

fn started(track: &str) -> Option<(Race, Vec<String>)> {
    started_in(track, oag_race::Mode::SingleRace)
}

fn started_in(track: &str, mode: oag_race::Mode) -> Option<(Race, Vec<String>)> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode,
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

/// The file the loader reports flying for `mode` on `track`: the entry name after
/// `pre-race flyby: ` on the line that reads it (`None` when no line does).
fn flown(track: &str, mode: oag_race::Mode) -> Option<Option<String>> {
    let (_, report) = started_in(track, mode)?;
    Some(
        report
            .iter()
            .filter_map(|line| line.strip_prefix("pre-race flyby: "))
            .filter(|line| !line.contains("did not read"))
            .find_map(|line| line.split([',', ' ']).next().map(str::to_string))
            .map(|entry| entry.rsplit('\\').next().unwrap_or(&entry).to_string()),
    )
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_lone_time_trial_craft_flies_the_ta_file() {
    // `Track_Construct_q` (`0x000b2af0`): game mode 5 takes `%s\%sstart_grid_ta%s.vex`.
    let Some(file) = flown(TRACK, oag_race::Mode::TimeTrial) else {
        return;
    };
    assert_eq!(file.as_deref(), Some("start_grid_ta.vex"));
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_speed_lap_flies_the_ta_file() {
    // Game mode 10.
    let Some(file) = flown(TRACK, oag_race::Mode::SpeedLap) else {
        return;
    };
    assert_eq!(file.as_deref(), Some("start_grid_ta.vex"));
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_zone_race_on_a_circuit_flies_the_ta_file() {
    // Game mode 6 is in the `0x206040` set.
    let Some(file) = flown(TRACK, oag_race::Mode::Zone) else {
        return;
    };
    assert_eq!(file.as_deref(), Some("start_grid_ta.vex"));
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_modes_with_a_grid_fly_the_plain_file() {
    // Single Race (3), Eliminator (8), Head to Head (9) and Tournament (4) are in no `_ta` row.
    for mode in [
        oag_race::Mode::SingleRace,
        oag_race::Mode::Eliminator,
        oag_race::Mode::Head2Head,
        oag_race::Mode::Tournament,
    ] {
        let Some(file) = flown(TRACK, mode) else {
            return;
        };
        assert_eq!(file.as_deref(), Some("start_grid.vex"), "{mode:?}");
    }
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_reversed_circuit_keeps_its_modes_file_family() {
    let Some(ta) = flown(TRACK_REVERSED, oag_race::Mode::TimeTrial) else {
        return;
    };
    assert_eq!(ta.as_deref(), Some("start_grid_ta_reversed.vex"));
    let Some(plain) = flown(TRACK_REVERSED, oag_race::Mode::SingleRace) else {
        return;
    };
    assert_eq!(plain.as_deref(), Some("start_grid_reversed.vex"));
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn zone_s_own_circuits_have_no_ta_file_and_fall_back_to_a_plain_one_with_no_camera() {
    // `zone_1` ships `start_grid.vex` only, and that file has no `grid_camera1`: the original's
    // fallback `%s\start_grid.vex` finds it, and there is nothing to fly.
    let Some((_, report)) = started_in(ZONE_TRACK, oag_race::Mode::Zone) else {
        return;
    };
    let lines: Vec<&String> = report
        .iter()
        .filter(|line| line.starts_with("pre-race flyby: "))
        .collect();
    assert!(
        lines
            .iter()
            .any(|l| l.contains("start_grid_ta.vex did not read")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("start_grid.vex has no playable gridCamera")),
        "{lines:?}"
    );
}
