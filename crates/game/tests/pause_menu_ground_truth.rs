//! Pulse's and HD's own pause screens over a real race: the rows are the disc's for the race's
//! mode, START and ESCAPE open it, and each row does what it says.
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
    run_on(&image, tag, script)
}

fn hd() -> Option<std::path::PathBuf> {
    oag_testdata::image("hdfury-ps3-eu-dec.iso")
}

fn run_on(image: &std::path::Path, tag: &str, script: &str) -> Run {
    let shots = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("pause-shots");
    let run = drive_binary(
        env!("CARGO_BIN_EXE_oag-game"),
        image,
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

/// HD and Fury author a different pause screen: `Block` and `List` widgets, and per mode (Time
/// Trial, Speed Lap, Tournament, Zone and the default) its own list. The rows this build has
/// something behind are drawn; PILOT ASSIST, GHOST and VIEW INVITES are not.
mod hd_screens {
    use super::*;

    fn run_hd(tag: &str, script: &str) -> Run {
        let image = hd().expect("the caller checks first");
        run_on(&image, &format!("hd-{tag}"), script)
    }

    /// The rows each HD screen shows (label, action or kind), by the mode that opens it: the
    /// disc's widgets in its order, PILOT ASSIST and GHOST as lists and VIEW INVITES inert.
    fn hd_rows(mode: &str) -> Vec<(String, String)> {
        let continue_ = ("CONTINUE", "back");
        let pilot = ("PILOT ASSIST", "choice");
        let ghost = ("GHOST", "choice");
        let options = [
            ("GAME OPTIONS", "submenu"),
            ("AUDIO OPTIONS", "submenu"),
            ("PHOTO MODE", "photo_mode"),
            ("VIEW INVITES", "inert"),
        ];
        let restart = ("RESTART RACE", "restart_race");
        let quit = ("QUIT RACE", "quit_race");
        let mut rows = vec![continue_];
        match mode {
            "time_trial" => rows.extend([pilot, ghost]),
            "speed_lap" => rows.extend([pilot, ghost]),
            "zone" => {}
            _ => rows.push(pilot),
        }
        rows.extend(options);
        match mode {
            "speed_lap" => rows.push(("END SESSION", "quit_race")),
            _ => rows.extend([restart, quit]),
        }
        pairs(&rows)
    }

    #[test]
    #[ignore = "needs a disc image in data/images/"]
    fn a_single_race_and_a_time_trial_pause_into_their_own_screens() {
        if hd().is_none() {
            return;
        }
        for mode in ["single_race", "time_trial"] {
            let run = run_hd(mode, &format!("{} {OPEN}", to_the_race(mode)));
            assert_eq!(rows(&run), hd_rows(mode), "{mode}");
        }
    }

    /// The Speed Lap screen has no RESTART and no QUIT: END SESSION is its only way out.
    #[test]
    #[ignore = "needs a disc image in data/images/"]
    fn a_speed_lap_pauses_into_its_screen_and_end_session_leaves_the_race() {
        if hd().is_none() {
            return;
        }
        let run = run_hd(
            "speed-lap",
            &format!(
                "{} {OPEN} select pause_options; press circle; expect page=pause; \
                 select pause_quit; press cross; until stage=menu; expect page=main",
                to_the_race("speed_lap")
            ),
        );
        assert_eq!(rows(&run), hd_rows("speed_lap"));
        assert_eq!(run.states.last().expect("a state")["screen"]["id"], "main");
    }

    #[test]
    #[ignore = "needs a disc image in data/images/"]
    fn a_zone_race_pauses_into_the_zone_screen() {
        if hd().is_none() {
            return;
        }
        let run = run_hd("zone", &format!("{} {OPEN}", to_the_race("zone")));
        assert_eq!(rows(&run), hd_rows("zone"));
    }

    #[test]
    #[ignore = "needs a disc image in data/images/"]
    fn audio_options_opens_the_audio_page_and_back_returns() {
        if hd().is_none() {
            return;
        }
        let run = run_hd(
            "audio",
            &format!(
                "{} {OPEN} select pause_audio; press cross; expect page=audio; \
                 press circle; expect page=pause; \
                 select pause_restart; press cross; until stage=race within 4000; ticks 20",
                to_the_race("time_trial")
            ),
        );
        let sims: Vec<u64> = run
            .states
            .iter()
            .filter_map(|state| state.get("sim_tick")?.as_u64())
            .collect();
        assert!(
            sims.last().copied().unwrap_or(u64::MAX) < 100,
            "RESTART RACE loads the race again: {sims:?}"
        );
    }

    /// The PILOT ASSIST list reaches the parked race on resume (ON is Extreme, the level HD
    /// flies with), and the other settings page edits the same level: nothing is switched on
    /// that the player did not choose.
    #[test]
    #[ignore = "needs a disc image in data/images/"]
    fn the_pilot_assist_list_switches_the_race_it_resumes() {
        if hd().is_none() {
            return;
        }
        let level = |run: &Run| -> String {
            run.states
                .iter()
                .rev()
                .find_map(|state| state.get("pilot_assist")?.as_str().map(str::to_string))
                .expect("a race state")
        };
        let race = to_the_race("time_trial");
        let plain = run_hd(
            "assist-off",
            &format!("{race} {OPEN} press cross; until stage=race; ticks 30"),
        );
        assert_eq!(level(&plain), "off", "the list opens on OFF");
        let on = run_hd(
            "assist-on",
            &format!(
                "{race} {OPEN} select pause_pilot_assist; press right; \
                 select pause_resume; press cross; until stage=race; ticks 30"
            ),
        );
        assert_eq!(level(&on), "extreme", "ON is Extreme");
    }

    /// An Eliminator race pauses into the default screen, and a pause and resume leaves the race
    /// as it was (the state hash at one simulation tick is the unpaused race's) and the flip still
    /// works after it (a press after the resume changes the hash at that tick).
    #[test]
    #[ignore = "needs a disc image in data/images/"]
    fn an_eliminator_race_resumes_with_its_flip_intact() {
        if hd().is_none() {
            return;
        }
        let hash_at = |run: &Run, tick: u64| -> String {
            run.states
                .iter()
                .find(|state| state.get("sim_tick").and_then(Value::as_u64) == Some(tick))
                .and_then(|state| state["hash"].as_str())
                .unwrap_or_else(|| panic!("no state at sim tick {tick}"))
                .to_string()
        };
        let race = to_the_race("eliminator");
        let resume = format!("{OPEN} press cross; until stage=race within 200;");
        let plain = run_hd("elim-plain", &format!("{race} ticks 80"));
        let paused = run_hd("elim-paused", &format!("{race} {resume} ticks 80"));
        let flipped = run_hd(
            "elim-flipped",
            &format!("{race} {resume} press flip; ticks 80"),
        );
        assert_eq!(
            rows(&paused),
            hd_rows("eliminator"),
            "an Eliminator race takes the default screen"
        );
        let at = 460;
        assert_eq!(
            hash_at(&plain, at),
            hash_at(&paused, at),
            "a pause and resume leaves the race as it was"
        );
        assert_ne!(
            hash_at(&paused, at),
            hash_at(&flipped, at),
            "the flip works after the resume"
        );
    }
}

/// Omega's table is read but not wired: a pause there is the main page over the parked race, as
/// before. Needs the decrypted package pair, not an image.
mod omega_screens {
    use super::*;

    #[test]
    #[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
    fn a_pause_on_omega_opens_the_main_page_over_the_race() {
        let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
            return;
        };
        let run = run_on(
            &source,
            "omega-fall-through",
            &format!(
                "{} escape; until stage=menu; expect page=main",
                to_the_race("time_trial")
            ),
        );
        assert!(
            on_page(&run, "pause").is_empty(),
            "no pause page: Omega is not wired"
        );
        let last = run.states.last().expect("a state");
        assert_eq!(last["screen"]["id"], "main");
    }
}
