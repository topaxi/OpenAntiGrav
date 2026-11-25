use super::*;

fn context() -> Context {
    Context {
        title: "Pulse",
        track: Some("16_Track".into()),
        mode: "single_race",
        laps_target: Some(3),
        adapter: "{\"name\": \"gpu\", \"type\": \"X\", \"backend\": \"Vulkan\", \"driver\": \"d\", \"driver_info\": \"i\", \"gpu_timestamps\": false}".into(),
        adapter_name: "gpu".into(),
        driver: "d".into(),
        device: "linux x86_64".into(),
        settings: vec![("size".into(), "1280x720".into())],
        refresh: (60.0, "display"),
        preset: None,
        origin: "cli",
        command: vec!["--benchmark".into()],
        load: ("1 1 1".into(), "1 1 1".into()),
        git_hash: "abc",
        wall_seconds: 1.0,
        people: 1,
        tick_cap: 40_000,
    }
}

fn report(frame_us: impl Iterator<Item = f64>) -> Report {
    Report {
        context: context(),
        frames: frame_us
            .map(|frame_us| Frame {
                frame_us,
                lap: 1,
                ticks: 1,
            })
            .collect(),
        ending: Ending {
            tick: 100,
            state_hash: 0xabc,
            finished: true,
        },
    }
}

#[test]
fn figures_skip_the_warm_up_and_count_the_budget() {
    // 60 warm-up frames at 100 ms, then 1,000 at 16.7 ms and ten at 50 ms.
    let frames = std::iter::repeat_n(100_000.0, WARMUP_FRAMES)
        .chain(std::iter::repeat_n(16_700.0, 990))
        .chain(std::iter::repeat_n(50_000.0, 10));
    let figures = report(frames).figures().unwrap();
    assert_eq!(figures.counted, 1000);
    assert_eq!(figures.over_budget, 10);
    assert!((figures.median_ms - 16.7).abs() < 1e-9);
    assert!((figures.worst_ms - 50.0).abs() < 1e-9);
    assert!((figures.low_1 - 20.0).abs() < 1e-6, "{}", figures.low_1);
    assert!(figures.low_01 <= figures.low_1);
}

#[test]
fn a_run_shorter_than_the_warm_up_has_no_figures() {
    assert!(report(std::iter::repeat_n(16_700.0, 10)).figures().is_none());
}

#[test]
fn the_result_file_has_what_bench_compare_reads() {
    let frames = std::iter::repeat_n(16_700.0, WARMUP_FRAMES + 100);
    let json: serde_json::Value = serde_json::from_str(&report(frames).json()).unwrap();
    for key in [
        "title",
        "git_hash",
        "build",
        "adapter",
        "settings",
        "mode",
        "laps_target",
        "tick_cap",
        "final_tick",
        "final_state_hash",
        "frames",
        "load_average_start",
        "load_average_end",
        "metrics",
        "run",
        "per_lap",
    ] {
        assert!(json.get(key).is_some(), "{key}");
    }
    assert_eq!(json["adapter"]["driver_info"], "i");
    assert_eq!(json["metrics"]["frame_us"]["p50"], 16700.0);
    assert_eq!(json["frames"], WARMUP_FRAMES + 100);
    assert_eq!(json["run"]["kind"], "in_game");
    assert_eq!(json["final_state_hash"], "0000000000000abc");
}
