//! Pulse's own pause screen over a real race: the rows are the disc's for the race's mode, START
//! and ESCAPE open it, and each row does what it says.
//!
//! **`#[ignore]`d and never run in CI.** It needs a disc image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^pause_menu_ground_truth::/)'
//! ```
//!
//! The row lists against the disc's `InGame_Definition.xml` are
//! `oag-raceplay`'s `main_menu_ground_truth`; this drives the game (`--drive`) through the
//! `Session` a window runs and reads its report, so a pause that stopped opening the title's
//! screen, or a row that stopped working, fails here.

use serde_json::Value;

use super::photo_mode_ground_truth::{Run, drive_binary, pulse};

/// To a race of `mode` on the RACE page, 400 ticks in.
fn to_the_race(mode: &str) -> String {
    format!(
        "boot; select racebox; press cross; until page=race; set race.mode={mode}; \
         select launch_race; press cross; until screen=picker; press cross; \
         until screen.id=ship-select; press cross; until stage=race within 4000; ticks 400; "
    )
}

fn run(tag: &str, script: &str) -> Run {
    let Some(image) = pulse() else {
        panic!("OAG_REQUIRE_GAME_DATA unset and no Pulse image: the caller checks first");
    };
    let shots = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("pause-shots");
    let run = drive_binary(
        env!("CARGO_BIN_EXE_oag-game"),
        &image,
        &format!("pause-{tag}"),
        script,
        &shots,
        &[],
    );
    assert!(run.status.success(), "{tag}: {}", run.stderr);
    run
}

/// The menu state lines on the page `page`, last first.
fn on_page<'a>(run: &'a Run, page: &str) -> Vec<&'a Value> {
    run.states
        .iter()
        .rev()
        .filter(|state| state.get("stage").and_then(Value::as_str) == Some("menu"))
        .filter(|state| state["screen"]["id"].as_str() == Some(page))
        .collect()
}

/// `(label, action or kind)` of the rows the pause page showed last.
fn rows(run: &Run) -> Vec<(String, String)> {
    let state = on_page(run, "pause")
        .into_iter()
        .next()
        .expect("the pause page was on screen");
    state["screen"]["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| {
            (
                row["label"].as_str().unwrap_or_default().to_string(),
                row["action"]
                    .as_str()
                    .or_else(|| row["kind"].as_str())
                    .unwrap_or_default()
                    .to_string(),
            )
        })
        .collect()
}

fn pairs(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(label, what)| ((*label).to_string(), (*what).to_string()))
        .collect()
}

const OPEN: &str = "press start; until stage=menu; expect page=pause; ticks 5; ";

/// What the three screens this build has rows for show: Continue, Game options, Photo mode
/// (the disc's row, or ours where the screen has none), Restart race and Quit race (the disc's words). The rows
/// the disc authors and this build has nothing behind (GAME MANUAL, END SESSION) are not drawn.
fn the_rows() -> Vec<(String, String)> {
    pairs(&[
        ("Continue", "back"),
        ("Game options", "submenu"),
        ("Photo mode", "photo_mode"),
        ("Restart race", "restart_race"),
        ("Quit race", "quit_race"),
    ])
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_time_trial_pauses_into_the_time_trial_screen() {
    if pulse().is_none() {
        return;
    }
    let run = run(
        "time-trial",
        &format!("{} {OPEN}", to_the_race("time_trial")),
    );
    assert_eq!(rows(&run), the_rows());
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_speed_lap_pauses_into_the_speed_lap_screen() {
    if pulse().is_none() {
        return;
    }
    let run = run("speed-lap", &format!("{} {OPEN}", to_the_race("speed_lap")));
    assert_eq!(rows(&run), the_rows());
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_single_race_pauses_into_the_default_screen() {
    if pulse().is_none() {
        return;
    }
    let run = run(
        "single-race",
        &format!("{} {OPEN}", to_the_race("single_race")),
    );
    assert_eq!(rows(&run), the_rows());
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn escape_opens_the_same_screen_and_start_or_continue_resumes() {
    if pulse().is_none() {
        return;
    }
    let run = run(
        "resume",
        &format!(
            "{} escape; until stage=menu; expect page=pause; press start; \
             until stage=race within 200; ticks 30; \
             press start; until stage=menu; expect page=pause; press cross; \
             until stage=race within 200; ticks 30",
            to_the_race("time_trial")
        ),
    );
    assert_eq!(rows(&run), the_rows());
    // The race ran on after each resume: its tick is past where the pause stopped it.
    let ticks: Vec<u64> = run
        .states
        .iter()
        .filter_map(|state| state.get("sim_tick")?.as_u64())
        .collect();
    assert!(
        ticks.windows(2).all(|pair| pair[1] >= pair[0]),
        "one race, never reset"
    );
    assert!(
        ticks.last().copied().unwrap_or(0) > 430,
        "it ran on: {ticks:?}"
    );
}

/// The disc's pause screen authors `backward="none"`: CIRCLE on the root does nothing.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn circle_on_the_pause_screen_does_nothing() {
    if pulse().is_none() {
        return;
    }
    let run = run(
        "circle",
        &format!(
            "{} {OPEN} press circle; ticks 20; expect stage=menu page=pause",
            to_the_race("time_trial")
        ),
    );
    assert_eq!(rows(&run), the_rows());
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn restart_race_loads_the_race_again_from_the_grid() {
    if pulse().is_none() {
        return;
    }
    let run = run(
        "restart",
        &format!(
            "{} {OPEN} select pause_restart; press cross; \
             until stage=race within 4000; ticks 20",
            to_the_race("time_trial")
        ),
    );
    let ticks: Vec<u64> = run
        .states
        .iter()
        .filter_map(|state| state.get("sim_tick")?.as_u64())
        .collect();
    let before = ticks.iter().copied().max().unwrap_or(0);
    let last = ticks.last().copied().unwrap_or(u64::MAX);
    assert!(before >= 400, "the first race ran: {before}");
    assert!(
        last < 100,
        "the race that followed starts again from tick 0: {last}"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn quit_race_leaves_a_custom_race_for_the_main_page() {
    if pulse().is_none() {
        return;
    }
    let run = run(
        "quit",
        &format!(
            "{} {OPEN} select pause_quit; press cross; until stage=menu; expect page=main",
            to_the_race("time_trial")
        ),
    );
    let last = run.states.last().expect("a state");
    assert_eq!(last["stage"], "menu");
    assert_eq!(
        last["screen"]["id"], "main",
        "no parked race: the main page"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn photo_mode_and_game_options_open_from_the_pause_screen() {
    if pulse().is_none() {
        return;
    }
    let run = run(
        "photo",
        &format!(
            "{} {OPEN} select pause_options; press cross; expect page=options; \
             press circle; expect page=pause; \
             select photo_mode; press cross; until photo=internal within 200",
            to_the_race("time_trial")
        ),
    );
    assert!(
        run.states.iter().any(|state| state["photo"] == "internal"),
        "photo mode opened"
    );
}
