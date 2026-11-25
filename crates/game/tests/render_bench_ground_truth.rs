//! `OAG_RENDER_BENCH_RACE` plays a race tick by tick, draws every frame and writes a per-frame
//! CSV and a JSON summary. Disc-backed, and needs a GPU like every `--screenshot` run.
//!
//! The whole-race matrix is `just bench-render`; this is the cheap guard that the wiring in
//! `race_capture` still reaches the loop (a capture that dropped it would write a screenshot and
//! no result files) and that two runs of one race agree on the tick count and final state hash.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game render_bench_ground_truth:: --run-ignored all
//! ```

/// Long enough to clear the 272-tick start-line countdown, whose frames are in no figure.
const TICKS: usize = 420;

/// The string or number after `"key": ` in the summary.
fn field(json: &str, key: &str) -> String {
    let marker = format!("\"{key}\": ");
    let rest = json
        .split(&marker)
        .nth(1)
        .unwrap_or_else(|| panic!("no {key} in:\n{json}"));
    rest.split([',', '\n'])
        .next()
        .unwrap()
        .trim()
        .trim_matches('"')
        .to_string()
}

/// Runs a fresh `TICKS`-tick bench into `dir` and returns its summary and CSV.
fn bench(dir: &std::path::Path, image: &std::path::Path) -> (String, String) {
    bench_with(dir, image, &[])
}

/// [`bench`] with `extra` flags after the fixed ones (`--players 2 --split-cut vertical`).
fn bench_with(dir: &std::path::Path, image: &std::path::Path, extra: &[&str]) -> (String, String) {
    std::fs::create_dir_all(dir).expect("a temp directory");
    let stem = dir.join("race");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args([
            "--no-audio",
            "--race",
            "--mode",
            "single_race",
            "--seed",
            "1",
            "--autopilot",
        ])
        .args([
            "--size",
            "320x180",
            "--log-every",
            "0",
            "--ticks",
            &TICKS.to_string(),
        ])
        .args(extra)
        .arg("--screenshot")
        .arg(dir.join("unused.png"))
        .env("OAG_RENDER_BENCH_RACE", &stem)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .output()
        .expect("running oag-game");
    assert!(
        out.status.success(),
        "oag-game: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json = std::fs::read_to_string(dir.join("race.json")).expect("the summary was written");
    let csv = std::fs::read_to_string(dir.join("race.csv")).expect("the per-frame CSV was written");
    (json, csv)
}

/// A race cut off by the tick cap reports `finished: false`, one CSV row per drawn frame, a
/// measured CPU time on every frame, and the same final tick and state hash on a second run.
/// Fails on a build where the hook in `race_capture::capture` is gone: no files are written.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn a_capped_race_writes_its_frames_and_replays_to_the_same_hash() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let base = std::env::temp_dir().join("oag-render-bench-ground-truth");
    let (json, csv) = bench(&base.join("a"), &image);
    let (again, _) = bench(&base.join("b"), &image);

    assert_eq!(
        field(&json, "finished"),
        "false",
        "240 ticks do not finish three laps"
    );
    assert_eq!(field(&json, "final_tick"), TICKS.to_string());
    let frames: usize = field(&json, "frames").parse().unwrap();
    assert_eq!(
        csv.lines().count(),
        frames + 1,
        "a header and a row per frame"
    );
    assert!(frames >= TICKS - 1, "{frames} frame(s) of {TICKS} ticks");
    assert!(
        csv.lines()
            .skip(1)
            .all(|row| row.split(',').nth(6).is_some_and(|cpu| !cpu.is_empty())),
        "every frame has a CPU time"
    );
    assert!(
        json.contains("\"cpu_us\": {\"count\":"),
        "the summary carries the CPU metric"
    );
    for key in ["final_tick", "final_state_hash", "frames"] {
        assert_eq!(
            field(&json, key),
            field(&again, key),
            "{key} differs between two runs"
        );
    }
}

/// A two-view race is benched as a split: the summary says two views and the cut, the CSV has a
/// timestamped scene pass per view, every frame has a CPU time (the whole `render_viewports`
/// call), and the cut changes the picture and not the race, so both cuts end on one state hash.
/// Fails on a build that refuses a split again (no files written) or drops the per-view columns.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn a_split_race_is_benched_per_view_and_the_cut_does_not_change_the_race() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let base = std::env::temp_dir().join("oag-render-bench-ground-truth-split");
    let two = ["--players", "2"];
    let (horizontal, csv) = bench_with(
        &base.join("h"),
        &image,
        &[&two[..], &["--split-cut", "horizontal"]].concat(),
    );
    let (vertical, _) = bench_with(
        &base.join("v"),
        &image,
        &[&two[..], &["--split-cut", "vertical"]].concat(),
    );

    assert!(horizontal.contains("\"views\": \"2\""), "{horizontal}");
    assert!(horizontal.contains("\"split_cut\": \"horizontal\""));
    assert!(vertical.contains("\"split_cut\": \"vertical\""));
    let header = csv.lines().next().unwrap();
    assert!(header.ends_with(",gpu_view0_us,gpu_view1_us"), "{header}");
    assert!(
        csv.lines()
            .skip(1)
            .all(|row| row.split(',').nth(6).is_some_and(|cpu| !cpu.is_empty())),
        "every frame has a CPU time"
    );
    assert_eq!(field(&horizontal, "final_tick"), TICKS.to_string());
    assert_eq!(
        field(&horizontal, "final_state_hash"),
        field(&vertical, "final_state_hash"),
        "the cut is presentation"
    );
}

/// Flies the in-game benchmark (`--benchmark`, driven with no window through the same `Session` a
/// window runs) for `TICKS` ticks in `dir` with `extra` flags, and returns its result file and the
/// path of the stats screen it drew. `during` is drive steps run once the race is flying.
fn in_game(
    dir: &std::path::Path,
    image: &std::path::Path,
    extra: &[&str],
    during: &str,
) -> (String, std::path::PathBuf) {
    in_game_of(dir, image, "pulse", &["--no-intro"], extra, during)
}

/// [`in_game`] on the title whose result file is `<key>.json`, with the launch flags `launch`
/// (`--no-intro` or none: a benchmark that plays its flyby).
fn in_game_of(
    dir: &std::path::Path,
    image: &std::path::Path,
    key: &str,
    launch: &[&str],
    extra: &[&str],
    during: &str,
) -> (String, std::path::PathBuf) {
    // A fresh run directory: an earlier run's result would be a second one.
    let _ = std::fs::remove_dir_all(dir.join("data"));
    std::fs::create_dir_all(dir).expect("a temp directory");
    let shot = dir.join("stats.png");
    let script = format!(
        "until benchmark=running within 3000; {during}; until benchmark=stats within 6000; \
         screenshot {}; quit",
        shot.display()
    );
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .arg("--no-audio")
        .args(launch)
        .args(["--benchmark", "--ticks"])
        .arg(TICKS.to_string())
        .args(extra)
        .arg("--drive")
        .arg(&script)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .output()
        .expect("running oag-game");
    assert!(
        out.status.success(),
        "oag-game: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let runs: Vec<_> = std::fs::read_dir(dir.join("data/oag/bench"))
        .expect("the benchmark wrote a run directory")
        .flatten()
        .collect();
    assert_eq!(runs.len(), 1, "one run");
    let json = std::fs::read_to_string(runs[0].path().join(format!("{key}.json")))
        .expect("the result file is named for the title");
    (json, shot)
}

/// The in-game benchmark ends its race on the state the offscreen bench ends the same race on,
/// writes a result file `bench-render.py compare` reads, and raises its stats screen. Fails on a
/// build that drops the benchmark's launch, its pinned race, its end-of-race hook or its screen:
/// no file, a different hash, or no `stats` phase to wait for.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn the_in_game_benchmark_flies_the_race_the_offscreen_bench_does() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let base = std::env::temp_dir().join("oag-in-game-benchmark-ground-truth");
    let (offscreen, _) = bench(&base.join("offscreen"), &image);
    let (json, shot) = in_game(&base.join("in-game"), &image, &[], "ticks 1");

    assert_eq!(field(&json, "final_tick"), TICKS.to_string());
    assert_eq!(field(&json, "finished"), "false");
    assert_eq!(
        field(&json, "final_state_hash"),
        field(&offscreen, "final_state_hash"),
        "the window flies the race the offscreen bench flies"
    );
    assert!(json.contains("\"kind\": \"in_game\""), "{json}");
    assert!(
        json.contains("\"frame_us\": {\"count\":"),
        "the frame metric"
    );
    assert!(
        std::fs::metadata(&shot).is_ok_and(|meta| meta.len() > 1000),
        "the stats screen was drawn"
    );
}

/// Pulse's EXTRAS row opens a page of ours holding BENCHMARK, which flies the same race the
/// offscreen bench does and returns to that page. Fails on a build that leaves `FE_EXTRAS`
/// unwired, drops the EXTRAS page, or sends the stats screen's BACK anywhere else.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn the_benchmark_row_is_on_pulses_extras_page_and_returns_to_it() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let base = std::env::temp_dir().join("oag-in-game-benchmark-extras-ground-truth");
    let (offscreen, _) = bench(&base.join("offscreen"), &image);
    let dir = base.join("menu");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temp directory");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(&image)
        .args(["--no-audio", "--no-intro", "--ticks"])
        .arg(TICKS.to_string())
        .arg("--drive")
        .arg(
            "boot; select extras; press cross; until page=extras within 300; \
             select benchmark; press cross; until benchmark=stats within 3000; press cross; \
             until stage=menu within 600; expect page=extras; quit",
        )
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .output()
        .expect("running oag-game");
    assert!(
        out.status.success(),
        "oag-game: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let runs: Vec<_> = std::fs::read_dir(dir.join("data/oag/bench"))
        .expect("the benchmark wrote a run directory")
        .flatten()
        .collect();
    assert_eq!(runs.len(), 1, "one run");
    let json = std::fs::read_to_string(runs[0].path().join("pulse.json")).expect("a result file");
    assert_eq!(
        field(&json, "final_state_hash"),
        field(&offscreen, "final_state_hash")
    );
}

/// A preset changes what a frame costs and never the race, and never the player's own settings
/// file: the run with `--preset low` says it drew at half resolution and ends on the plain run's
/// state hash, and no `settings.toml` of the player's holds the preset's values. Fails on a build
/// that drops the preset (the settings are the player's), applies it to the race (another hash)
/// or saves it (a file with the preset's shadows).
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn a_preset_changes_the_picture_and_not_the_race_or_the_players_settings() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let base = std::env::temp_dir().join("oag-in-game-benchmark-preset-ground-truth");
    let (plain, _) = in_game(&base.join("plain"), &image, &[], "ticks 1");
    let low_dir = base.join("low");
    // The player's own settings file, written before the run: a render scale the preset's is
    // not, so a preset that reached the file would change it.
    let _ = std::fs::remove_dir_all(&low_dir);
    std::fs::create_dir_all(&low_dir).expect("a temp directory");
    let seeded = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(&image)
        .args([
            "--no-audio",
            "--no-intro",
            "--drive",
            "boot; select options; press cross; until page=options; select GRAPHICS; press cross; \
             until page=graphics; set graphics.render_scale=75; quit",
        ])
        .env("XDG_CONFIG_HOME", low_dir.join("config"))
        .env("XDG_STATE_HOME", low_dir.join("state"))
        .env("XDG_DATA_HOME", low_dir.join("data"))
        .output()
        .expect("running oag-game");
    assert!(
        seeded.status.success(),
        "{}",
        String::from_utf8_lossy(&seeded.stderr)
    );
    let settings = low_dir.join("config/oag/settings.toml");
    let before = std::fs::read_to_string(&settings)
        .expect("the seeding run wrote settings.toml where the benchmark run reads it");
    assert!(before.contains("render_scale = 75"), "{before}");
    // The camera button saves the settings file on every press; with a preset in force it must
    // not (and in a benchmark it does nothing). Losing focus must neither end the run nor give
    // the settings back while it flies.
    let (low, _) = in_game(
        &low_dir,
        &image,
        &["--preset", "low"],
        "press select; away; ticks 5; expect benchmark=running",
    );

    assert_eq!(
        field(&low, "final_state_hash"),
        field(&plain, "final_state_hash"),
        "a preset is not a different race"
    );
    assert!(low.contains("\"render_scale\": \"50%\""), "{low}");
    assert!(low.contains("\"shadows\": \"Blob\""), "{low}");
    assert!(low.contains("\"preset\": \"low\""), "{low}");
    assert!(plain.contains("\"preset\": null"), "{plain}");
    assert_eq!(
        std::fs::read_to_string(&settings).expect("settings.toml is still there"),
        before,
        "the preset's values (or the camera cycle) reached settings.toml"
    );
}

/// RUN BENCHMARK would replace a race parked behind the menus, so the row is not on the graphics
/// page then, and is there when no race is parked. Fails on a build that offers it over a parked
/// race.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn run_benchmark_is_not_offered_over_a_parked_race() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let dir = std::env::temp_dir().join("oag-in-game-benchmark-parked-ground-truth");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temp directory");
    let log = dir.join("state.jsonl");
    let rows = |middle: &str| -> Vec<String> {
        let _ = std::fs::remove_file(&log);
        let script = format!("boot; {middle}; draw; quit");
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
            .arg(&image)
            .args(["--no-audio", "--no-intro", "--state-log"])
            .arg(&log)
            .arg("--drive")
            .arg(script)
            .env("XDG_CONFIG_HOME", dir.join("config"))
            .env("XDG_STATE_HOME", dir.join("state"))
            .env("XDG_DATA_HOME", dir.join("data"))
            .output()
            .expect("running oag-game");
        assert!(
            out.status.success(),
            "oag-game: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let last = std::fs::read_to_string(&log)
            .expect("a state log")
            .lines()
            .last()
            .expect("a state line")
            .to_string();
        let state: serde_json::Value = serde_json::from_str(&last).unwrap();
        state["screen"]["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .filter_map(|row| row["label"].as_str().map(str::to_string))
            .collect()
    };
    // Control: with no race, the row is there.
    assert!(
        rows(
            "select options; press cross; until page=options; select GRAPHICS; press cross; \
              until page=graphics"
        )
        .iter()
        .any(|label| label == "RUN BENCHMARK"),
        "the control run has no RUN BENCHMARK row"
    );
    // A race flown a moment and left with Escape is parked; its pause menu reaches the same
    // OPTIONS page through GAME OPTIONS.
    let parked = rows(
        "select racebox; press cross; until page=race; select launch_race; press cross; \
         until screen=picker; press cross; until screen.id=ship-select within 600; press cross; \
         until stage=race within 4000; ticks 30; escape; until stage=menu within 600; \
         select pause_options; press cross; until page=options; select GRAPHICS; press cross; \
         until page=graphics",
    );
    assert!(parked.iter().any(|label| label == "ADVANCED"), "{parked:?}");
    assert!(
        !parked.iter().any(|label| label == "RUN BENCHMARK"),
        "{parked:?}"
    );
}

/// Every input a player has, spent over the flyby and the race, with the pad's Back left out
/// (Back abandons the run). Pulse's flyby ends by itself; the race is flown past the countdown
/// so a held stick or button would reach a craft that is released.
const EVERY_INPUT: &str = "hold cross 90; click; press select; press start; press cross; away; \
    key; wheel 2; drag 30 30; ticks 5; until sim_tick=300 within 6000; \
    hold cross start select 15; lstick 1 0 20; hold left 10; press select; press start; away; \
    click; ticks 3";

/// What a benchmark's result must repeat run after run: the length of the flyby, the race's end,
/// the frames drawn (a paused frame is a frame) and the camera the settings name.
fn run_identity(json: &str) -> Vec<String> {
    ["flyby_ticks", "final_tick", "final_state_hash", "frames"]
        .into_iter()
        .map(|key| format!("{key}={}", field(json, key)))
        .chain(
            json.lines()
                .filter(|line| line.contains("\"camera_view\""))
                .map(|line| line.trim().to_string()),
        )
        .collect()
}

/// During a benchmark no input changes what runs: a held Cross, a click, the camera button, Start,
/// a stick, focus loss and a key leave the flyby the same length, the race on the same hash and
/// tick count and the same number of frames, and `settings.toml` as the quiet run left it. Fails on a build that
/// lets a press or a click skip the flyby, lets Start pause, or lets the camera button cycle (the
/// flyby ticks differ, the frames differ, or the settings file appears).
fn input_changes_nothing(image: &std::path::Path, key: &str, base: &std::path::Path) {
    let (quiet, _) = in_game_of(&base.join("quiet"), image, key, &[], &[], "ticks 1");
    let (busy, _) = in_game_of(&base.join("busy"), image, key, &[], &[], EVERY_INPUT);
    assert_eq!(
        run_identity(&busy),
        run_identity(&quiet),
        "input changed the run"
    );
    // A flyby that plays: a skipped one is a few dozen ticks.
    let flyby: u32 = field(&quiet, "flyby_ticks").parse().unwrap();
    assert!(flyby >= 60 * 17, "the flyby ran {flyby} ticks");
    let settings =
        |run: &str| std::fs::read_to_string(base.join(run).join("config/oag/settings.toml"));
    assert_eq!(
        settings("busy").expect("settings.toml"),
        settings("quiet").expect("settings.toml"),
        "the camera button changed the player's settings"
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn no_input_changes_a_pulse_benchmark() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let base = std::env::temp_dir().join("oag-in-game-benchmark-input-pulse");
    input_changes_nothing(&image, "pulse", &base);
}

/// HD's flyby loops until somebody presses: the run presses for it at a fixed tick, so it plays the
/// same length every time whatever the player does.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU"]
fn no_input_changes_an_hd_benchmark() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let base = std::env::temp_dir().join("oag-in-game-benchmark-input-hd");
    input_changes_nothing(&image, "hd", &base);
    let (json, _) = in_game_of(&base.join("again"), &image, "hd", &[], &[], "ticks 1");
    assert_eq!(
        field(&json, "flyby_ticks"),
        (27 * 60 + 1).to_string(),
        "the press lands on the same tick every run"
    );
}

/// The one input a benchmark takes: Back (Escape, the pad's back button, the system Back) abandons
/// it. Started from the EXTRAS page it returns to that page, writes no result, and the `--preset`
/// run of the same abandon gives the player's settings back and writes none either. Fails on a
/// build that ignores the pad's back button or the system Back during the run (the run flies on
/// and writes a result) or leaves the run going after Back.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn back_abandons_a_benchmark_and_writes_nothing() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let base = std::env::temp_dir().join("oag-in-game-benchmark-abort-ground-truth");
    let run = |dir: &std::path::Path, args: &[&str], script: &str| {
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir).expect("a temp directory");
        std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
            .arg(&image)
            .args(["--no-audio"])
            .args(args)
            .arg("--drive")
            .arg(script)
            .env("XDG_CONFIG_HOME", dir.join("config"))
            .env("XDG_STATE_HOME", dir.join("state"))
            .env("XDG_DATA_HOME", dir.join("data"))
            .output()
            .expect("running oag-game")
    };
    for (name, abandon) in [
        ("pad", "press circle"),
        ("system-back", "back"),
        ("escape", "escape"),
    ] {
        let dir = base.join(name);
        let script = format!(
            "boot; select extras; press cross; until page=extras within 300; select benchmark; \
             press cross; until benchmark=running within 3000; until sim_tick=300 within 6000; \
             {abandon}; until stage=menu within 600; expect page=extras; quit"
        );
        let out = run(&dir, &[], &script);
        assert!(
            out.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !dir.join("data/oag/bench").exists(),
            "{name}: an abandoned run wrote a result"
        );
    }
    // From the command line there is nothing behind the run: abandoning it ends the game, with
    // the preset never written to the player's file.
    let dir = base.join("command-line");
    let out = run(
        &dir,
        &["--benchmark", "--preset", "low", "--no-intro"],
        "until benchmark=running within 3000; until sim_tick=300 within 6000; back; ticks 5; quit",
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("the game quit"),
        "the system Back left the run going: {}",
        String::from_utf8_lossy(&out.stderr)
            .lines()
            .rev()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(!dir.join("data/oag/bench").exists(), "a result was written");
    let settings =
        std::fs::read_to_string(dir.join("config/oag/settings.toml")).unwrap_or_default();
    assert!(!settings.contains("render_scale = 50"), "{settings}");
}
