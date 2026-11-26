use super::*;

pub(crate) fn context() -> Context {
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
        details: vec![("RENDER SCALE".into(), "100".into())],
        track_name: Some("Vineta K".into()),
        build_version: "test",
        refresh: (60.0, "display"),
        frame_limit_hz: None,
        target_fps_hz: None,
        preset: None,
        mods: None,
        vsync: "off".into(),
        present_mode: "Immediate".into(),
        frame_limit: "240".into(),
        origin: "cli",
        command: vec!["--benchmark".into()],
        load: ("1 1 1".into(), "1 1 1".into()),
        git_hash: "abc",
        wall_seconds: 1.0,
        people: 1,
        tick_cap: 40_000,
    }
}

pub(crate) fn report(frame_us: impl Iterator<Item = f64>) -> Report {
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
pub(crate) const COUNTDOWN_FRAMES: usize = COUNTDOWN_TICKS as usize - 1;

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

/// A Galaxy S24's HD run as it reported itself (`docs/tools/in-game-benchmark.md`, "First phone
/// result"): 9165 counted frames, median 11.82 ms, p95 16.90, p99 18.27, worst 25.54, 8476 over
/// the 8.3 ms refresh period. `late` is what the frames between p95 and p99 took, which the
/// report did not say.
fn s24(late: f64) -> Report {
    let ms = [
        (689, 8.0),
        (3893, 10.5),
        (1, 11.82),
        (4123, 14.05),
        (1, 16.9),
        (366, late),
        (1, 18.27),
        (90, 19.794),
        (1, 25.54),
    ];
    let raced = ms
        .into_iter()
        .flat_map(|(count, ms)| std::iter::repeat_n(ms * 1000.0, count));
    let mut run = report(std::iter::repeat_n(100_000.0, COUNTDOWN_FRAMES).chain(raced));
    run.context.refresh = (120.0, "display");
    run
}

#[test]
fn a_run_that_asked_for_60_is_not_judged_against_a_120_hz_screen() {
    // The fixture is the run: with nothing asked for it reads as the phone reported it.
    let judged_at_the_screen = s24(17.0).figures().unwrap();
    assert_eq!(judged_at_the_screen.counted, 9165);
    assert_eq!(judged_at_the_screen.over_budget, 8476);
    for (got, reported) in [
        (judged_at_the_screen.median_ms, 11.82),
        (judged_at_the_screen.p95_ms, 16.90),
        (judged_at_the_screen.p99_ms, 18.27),
        (judged_at_the_screen.worst_ms, 25.54),
    ] {
        assert!((got - reported).abs() < 1e-9, "{got} for {reported}");
    }
    assert!((judged_at_the_screen.average_fps - 81.5).abs() < 0.2);
    assert!((judged_at_the_screen.low_1 - 50.4).abs() < 0.1);
    assert_eq!(judged_at_the_screen.verdict(), Verdict::Choppy);

    // The percentiles bound the late frames whatever lies between p95 and p99: 17.0 and 17.6
    // are either side of the 17.5 ms the 60 FPS budget allows.
    for (late, over) in [(17.0, 92), (17.6, 458)] {
        let mut run = s24(late);
        run.context.target_fps_hz = Some(60.0);
        let figures = run.figures().unwrap();
        assert!((figures.budget_ms - 1000.0 / 60.0).abs() < 1e-9);
        assert_eq!(figures.over_budget, over, "late {late}");
        assert_eq!(figures.verdict(), Verdict::MostlySmooth, "late {late}");
        assert!(run.json().contains("\"budget_from\": \"target_fps\""));
    }
}

/// `slow` of 1000 raced frames at `slow_ms`, the rest at `ms`, on a screen of `hz`.
fn on_screen(hz: f64, ms: f64, slow: usize, slow_ms: f64) -> Report {
    let mut run = report(
        std::iter::repeat_n(100_000.0, COUNTDOWN_FRAMES)
            .chain(std::iter::repeat_n(ms * 1000.0, 1000 - slow))
            .chain(std::iter::repeat_n(slow_ms * 1000.0, slow)),
    );
    run.context.refresh = (hz, "display");
    run
}

#[test]
fn with_no_target_and_no_limit_a_120_hz_screen_is_the_budget() {
    let figures = on_screen(120.0, 8.3, 100, 16.6).figures().unwrap();
    assert!((figures.budget_ms - 1000.0 / 120.0).abs() < 1e-9);
    assert_eq!(figures.over_budget, 100);
    assert_eq!(figures.verdict(), Verdict::Choppy);
}

#[test]
fn a_vsynced_60_hz_run_allows_jitter_and_counts_the_missed_refreshes() {
    // Vsync paces the loop itself, so no frame limit is in force: frames land around 16.67 ms
    // and a missed refresh takes two.
    let run = on_screen(60.0, 16.9, 8, 33.3);
    let figures = run.figures().unwrap();
    assert_eq!(run.context.budget().from, BudgetFrom::Refresh);
    assert_eq!(figures.over_budget, 8);
    assert_eq!(figures.verdict(), Verdict::Smooth);
}

#[test]
fn the_budget_is_the_longest_period_the_player_or_the_screen_sets() {
    use BudgetFrom::{FrameLimit, Refresh, TargetFps};
    for (target, limit, refresh, hz, from) in [
        (None, None, (120.0, "display"), 120.0, Refresh),
        (Some(60.0), None, (120.0, "display"), 60.0, TargetFps),
        (Some(60.0), Some(30.0), (120.0, "display"), 30.0, FrameLimit),
        (Some(144.0), None, (60.0, "display"), 60.0, Refresh),
        (None, Some(240.0), (120.0, "display"), 120.0, Refresh),
        (Some(60.0), Some(60.0), (60.0, "display"), 60.0, TargetFps),
        (None, Some(60.0), (60.0, "display"), 60.0, FrameLimit),
        (Some(120.0), None, (60.0, "assumed"), 60.0, Refresh),
        (Some(30.0), None, (60.0, "assumed"), 30.0, TargetFps),
    ] {
        let mut context = context();
        context.target_fps_hz = target;
        context.frame_limit_hz = limit;
        context.refresh = refresh;
        assert_eq!(
            context.budget(),
            Budget { hz, from },
            "target {target:?}, limit {limit:?}, refresh {refresh:?}"
        );
    }
}
