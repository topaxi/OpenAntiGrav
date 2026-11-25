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
        frame_limit_hz: None,
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
            .enumerate()
            .map(|(index, frame_us)| Frame {
                frame_us,
                lap: 1,
                ticks: 1,
                tick: index as u64 + 1,
            })
            .collect(),
        ending: Ending {
            tick: 100,
            state_hash: 0xabc,
            finished: true,
            flyby_ticks: 0,
        },
    }
}

/// Frames drawn before the craft is released: one tick each, so tick `COUNTDOWN_TICKS` is the
/// first raced frame.
const COUNTDOWN_FRAMES: usize = COUNTDOWN_TICKS as usize - 1;

#[test]
fn figures_skip_the_countdown_and_count_the_budget() {
    // The countdown's frames at 100 ms, then 990 at 16.7 ms and ten at 50 ms.
    let frames = std::iter::repeat_n(100_000.0, COUNTDOWN_FRAMES)
        .chain(std::iter::repeat_n(16_700.0, 990))
        .chain(std::iter::repeat_n(50_000.0, 10));
    let figures = report(frames).figures().unwrap();
    assert_eq!(figures.counted, 1000);
    assert_eq!(figures.over_budget, 10);
    assert!((figures.median_ms - 16.7).abs() < 1e-9);
    assert!((figures.worst_ms - 50.0).abs() < 1e-9);
    assert!((figures.low_1 - 20.0).abs() < 1e-6, "{}", figures.low_1);
    assert!(figures.low_01.unwrap() <= figures.low_1);
}

#[test]
fn the_point_one_percent_low_is_not_given_for_a_short_run() {
    let frames = |raced: usize| {
        report(
            std::iter::repeat_n(100_000.0, COUNTDOWN_FRAMES)
                .chain(std::iter::repeat_n(16_700.0, raced)),
        )
    };
    assert_eq!(frames(999).figures().unwrap().low_01, None);
    assert!(frames(1000).figures().unwrap().low_01.is_some());
    let json: serde_json::Value = serde_json::from_str(&frames(100).json()).unwrap();
    assert!(json["in_game"]["low_0_1pct_fps"].is_null());
}

#[test]
fn a_frame_limit_the_average_sits_on_is_named() {
    let raced = |frame_us: f64, limit: Option<f64>| {
        let mut run = report(
            std::iter::repeat_n(100_000.0, COUNTDOWN_FRAMES)
                .chain(std::iter::repeat_n(frame_us, 100)),
        );
        run.context.frame_limit_hz = limit;
        run
    };
    // 240 fps on a 240 fps limit is the limiter's; 100 fps is the machine's.
    let held = raced(1e6 / 240.0, Some(240.0));
    assert_eq!(held.figures().unwrap().limited_by, Some(240.0));
    assert!(held.json().contains("\"limiter_binds\": true"));
    assert_eq!(
        raced(10_000.0, Some(240.0)).figures().unwrap().limited_by,
        None
    );
    assert_eq!(raced(1e6 / 240.0, None).figures().unwrap().limited_by, None);
}

#[test]
fn a_run_that_ends_in_the_countdown_has_no_figures() {
    assert!(
        report(std::iter::repeat_n(16_700.0, 10))
            .figures()
            .is_none()
    );
}

#[test]
fn the_result_file_has_what_bench_compare_reads() {
    let frames = std::iter::repeat_n(16_700.0, COUNTDOWN_FRAMES + 100);
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
    assert_eq!(json["frames"], COUNTDOWN_FRAMES + 100);
    assert_eq!(json["warmup_frames_excluded"], COUNTDOWN_FRAMES);
    assert_eq!(json["countdown_ticks_excluded"], COUNTDOWN_TICKS);
    assert_eq!(json["run"]["kind"], "in_game");
    assert_eq!(json["final_state_hash"], "0000000000000abc");
}
