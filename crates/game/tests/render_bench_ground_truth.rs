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
        .args(["--no-audio", "--no-intro", "--benchmark", "--ticks"])
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
    let json = std::fs::read_to_string(runs[0].path().join("pulse.json"))
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
    // not. Losing focus and the system Back must neither end the run nor give the settings back
    // while it flies.
    let (low, _) = in_game(
        &low_dir,
        &image,
        &["--preset", "low"],
        "press select; away; ticks 5; back; ticks 5; expect benchmark=running",
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
