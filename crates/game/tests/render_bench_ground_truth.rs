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

const TICKS: usize = 240;

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
