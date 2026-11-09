//! Pulse's, HD's, Omega's and Pure's own pause screens over a real race: the rows are the disc's for the race's
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
    pub(super) fn hd_rows(mode: &str) -> Vec<(String, String)> {
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

        // The change is recorded as an event and the recording goes on, so the run still
        // reproduces (the replay ground truth proves the file); dropping the call that records
        // it leaves the count at zero.
        let events = |run: &Run| -> Option<u64> {
            run.states
                .iter()
                .rev()
                .find_map(|state| state.get("recorded_events"))
                .and_then(serde_json::Value::as_u64)
        };
        assert_eq!(events(&plain), Some(0), "a recorded race with no change");
        assert!(
            events(&on).is_some_and(|n| n >= 1),
            "the switch is an event in a recording that went on: {:?}",
            events(&on)
        );
    }

    /// The GHOST list switches the ghost ship's drawing in the race it resumes
    /// (`display.ghost_ship_visible`, which HD's own GAME OPTIONS page lists too).
    #[test]
    #[ignore = "needs a disc image in data/images/"]
    fn the_ghost_list_hides_the_ghost_ship_of_the_race_it_resumes() {
        if hd().is_none() {
            return;
        }
        let shown = |run: &Run| -> bool {
            run.states
                .iter()
                .rev()
                .find_map(|state| state.get("ghost_visible")?.as_bool())
                .expect("a race state")
        };
        let race = to_the_race("time_trial");
        let plain = run_hd(
            "ghost-on",
            &format!("{race} {OPEN} press cross; until stage=race; ticks 30"),
        );
        assert!(shown(&plain), "the list opens on ON");
        let off = run_hd(
            "ghost-off",
            &format!(
                "{race} {OPEN} select pause_ghost; press left; \
                 select pause_resume; press cross; until stage=race; ticks 30"
            ),
        );
        assert!(!shown(&off), "OFF hides it");
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

/// Omega's pause screens: HD's file less the invites row, over a real Omega race. Needs the
/// decrypted package pair, not an image.
mod omega_screens {
    use super::*;

    fn omega() -> Option<std::path::PathBuf> {
        oag_testdata::exact("data/extracted/ps4")
    }

    fn run_omega(tag: &str, script: &str) -> Run {
        let source = omega().expect("the caller checks first");
        run_on(&source, &format!("omega-{tag}"), script)
    }

    /// The rows an Omega screen shows: HD's, without PSN's invites.
    fn omega_rows(mode: &str) -> Vec<(String, String)> {
        super::hd_screens::hd_rows(mode)
            .into_iter()
            .filter(|(label, _)| label != "VIEW INVITES")
            .collect()
    }

    fn rows_of(mode: &str) {
        if omega().is_none() {
            return;
        }
        let run = run_omega(mode, &format!("{} {OPEN}", to_the_race(mode)));
        assert_eq!(rows(&run), omega_rows(mode), "{mode}");
    }

    #[test]
    #[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
    fn a_single_race_pauses_into_the_default_screen() {
        rows_of("single_race");
    }

    #[test]
    #[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
    fn a_time_trial_pauses_into_the_time_trial_screen() {
        rows_of("time_trial");
    }

    #[test]
    #[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
    fn a_zone_race_pauses_into_the_zone_screen() {
        rows_of("zone");
    }

    /// The Speed Lap screen has no RESTART and no QUIT: END SESSION is its only way out.
    #[test]
    #[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
    fn a_speed_lap_pauses_into_its_screen_and_end_session_leaves_the_race() {
        if omega().is_none() {
            return;
        }
        let run = run_omega(
            "speed-lap",
            &format!(
                "{} {OPEN} select pause_options; press circle; expect page=pause; \
                 select pause_quit; press cross; until stage=menu; expect page=main",
                to_the_race("speed_lap")
            ),
        );
        assert_eq!(rows(&run), omega_rows("speed_lap"));
        assert_eq!(run.states.last().expect("a state")["screen"]["id"], "main");
    }

    /// START resumes the race with the lists on their opening values (PILOT ASSIST off, GHOST on),
    /// and what the lists were set to reaches the race it resumes: ON is Extreme, the level HD
    /// flies with, and GHOST OFF hides the ghost ship (`display.ghost_ship_visible`).
    #[test]
    #[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
    fn start_resumes_and_the_lists_reach_the_race_they_resume() {
        if omega().is_none() {
            return;
        }
        let last = |run: &Run, key: &str| -> Value {
            run.states
                .iter()
                .rev()
                .find_map(|state| state.get(key).cloned())
                .expect("a race state")
        };
        let race = to_the_race("time_trial");
        let plain = run_omega(
            "lists-opening",
            &format!("{race} {OPEN} press start; until stage=race; ticks 30"),
        );
        assert_eq!(last(&plain, "pilot_assist"), "off", "the list opens on OFF");
        assert_eq!(last(&plain, "ghost_visible"), true, "the list opens on ON");
        let set = run_omega(
            "lists-set",
            &format!(
                "{race} {OPEN} select pause_pilot_assist; press right; \
                 select pause_ghost; press left; \
                 select pause_resume; press cross; until stage=race; ticks 30"
            ),
        );
        assert_eq!(last(&set, "pilot_assist"), "extreme", "ON is Extreme");
        assert_eq!(
            last(&set, "ghost_visible"),
            false,
            "OFF hides the ghost ship"
        );
    }

    /// RESTART RACE loads the race again.
    #[test]
    #[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
    fn restart_race_loads_the_race_again_from_the_grid() {
        if omega().is_none() {
            return;
        }
        let run = run_omega(
            "restart",
            &format!(
                "{} {OPEN} select pause_restart; press cross; until stage=race within 4000; \
                 ticks 20",
                to_the_race("single_race")
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
}

mod pure_screens {
    use super::*;

    fn pure() -> Option<std::path::PathBuf> {
        oag_testdata::image("pure-psp-eu.chd")
    }

    fn run_pure(tag: &str, script: &str) -> Run {
        let image = pure().expect("the caller checks first");
        run_on(&image, &format!("pure-{tag}"), script)
    }

    /// To a Tournament race, 400 ticks in: SINGLE PLAYER, TOURNAMENT, the class, the first
    /// tournament, the team.
    const TO_THE_TOURNAMENT: &str = "boot; select campaign; press cross; ticks 40; press down; \
        ticks 10; press cross; ticks 40; press cross; ticks 40; press cross; ticks 40; \
        press cross; ticks 40; until stage=race within 4000; ticks 400; ";

    /// Pure's own words for the rows (`Continue`, `Restart`, `Quit`), with this project's PHOTO
    /// MODE row after the first (the screen authors none).
    fn rows_with_restart() -> Vec<(String, String)> {
        pairs(&[
            ("Continue", "back"),
            ("PHOTO MODE", "photo_mode"),
            ("Restart", "restart_race"),
            ("Quit", "quit_race"),
        ])
    }

    /// Single Race, Time Trial and Zone take the default screen, `InGame Main Menu`.
    #[test]
    #[ignore = "needs a disc image in data/images/"]
    fn a_single_race_a_time_trial_and_a_zone_race_pause_into_the_default_screen() {
        if pure().is_none() {
            return;
        }
        for mode in ["single_race", "time_trial"] {
            let run = run_pure(mode, &format!("{} {OPEN}", to_the_race(mode)));
            assert_eq!(rows(&run), rows_with_restart(), "{mode}");
        }
        // A Zone race picks its circuit and takes no ship selection after it.
        let zone = run_pure(
            "zone",
            &format!(
                "boot; select racebox; press cross; until page=race; set race.mode=zone; \
                 select launch_race; press cross; until screen=picker; press cross; \
                 until stage=race within 4000; ticks 400; {OPEN}"
            ),
        );
        assert_eq!(rows(&zone), rows_with_restart(), "zone");
    }

    /// A Tournament race takes `InGame NoRestart Main Menu`: no RESTART.
    #[test]
    #[ignore = "needs a disc image in data/images/"]
    fn a_tournament_race_pauses_into_the_screen_without_a_restart() {
        if pure().is_none() {
            return;
        }
        let run = run_pure("tournament", &format!("{TO_THE_TOURNAMENT} {OPEN}"));
        assert_eq!(
            rows(&run),
            pairs(&[
                ("Continue", "back"),
                ("PHOTO MODE", "photo_mode"),
                ("Quit", "quit_race")
            ])
        );
    }

    /// A Free Play race takes `InGame Free Play Menu`: END SESSION in place of QUIT, which leaves
    /// the race for the main page here.
    #[test]
    #[ignore = "needs a disc image in data/images/"]
    fn a_free_play_race_pauses_into_the_screen_with_end_session() {
        if pure().is_none() {
            return;
        }
        let to_free_play = "boot; select campaign; press cross; ticks 40; press down; ticks 10; \
            press down; ticks 10; press down; ticks 10; press down; ticks 10; press cross; \
            ticks 40; press cross; ticks 40; press cross; ticks 40; press cross; ticks 40; \
            press cross; ticks 40; until stage=race within 4000; ticks 400; ";
        let run = run_pure("free-play", &format!("{to_free_play} {OPEN}"));
        assert_eq!(
            rows(&run),
            pairs(&[
                ("Continue", "back"),
                ("PHOTO MODE", "photo_mode"),
                ("Restart", "restart_race"),
                ("End Session", "quit_race")
            ])
        );
        let ended = run_pure(
            "free-play-end",
            &format!(
                "{to_free_play} {OPEN} select pause_quit; press cross; until stage=menu; \
                 expect page=main"
            ),
        );
        assert_eq!(ended.states.last().expect("a state")["stage"], "menu");
    }

    /// START and CONTINUE resume, CIRCLE on the root does nothing, RESTART loads the race again,
    /// QUIT leaves for the main page and PHOTO MODE opens from the screen.
    #[test]
    #[ignore = "needs a disc image in data/images/"]
    fn the_rows_work_by_pad() {
        if pure().is_none() {
            return;
        }
        let race = to_the_race("single_race");
        let resumed = run_pure(
            "start",
            &format!("{race} {OPEN} press start; until stage=race; ticks 30"),
        );
        assert_eq!(
            resumed.states.last().expect("a state")["stage"],
            "race",
            "START resumes"
        );
        let circle = run_pure(
            "circle",
            &format!("{race} {OPEN} press circle; ticks 20; expect stage=menu page=pause"),
        );
        assert_eq!(rows(&circle), rows_with_restart(), "CIRCLE does nothing");
        let restart = run_pure(
            "restart",
            &format!(
                "{race} {OPEN} select pause_restart; press cross; until stage=race within 4000; \
                 ticks 20"
            ),
        );
        let sims: Vec<u64> = restart
            .states
            .iter()
            .filter_map(|state| state.get("sim_tick")?.as_u64())
            .collect();
        assert!(
            sims.last().copied().unwrap_or(u64::MAX) < 100,
            "RESTART: {sims:?}"
        );
        let quit = run_pure(
            "quit",
            &format!(
                "{race} {OPEN} select pause_quit; press cross; until stage=menu; expect page=main"
            ),
        );
        assert_eq!(quit.states.last().expect("a state")["screen"]["id"], "main");
        let photo = run_pure(
            "photo",
            &format!(
                "{race} {OPEN} select photo_mode; press cross; until photo=internal within 200"
            ),
        );
        assert!(
            photo
                .states
                .iter()
                .any(|state| state["photo"] == "internal"),
            "PHOTO MODE opens the photo lens from the pause screen"
        );
    }
}
