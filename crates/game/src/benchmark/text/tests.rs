use super::*;
use crate::benchmark::report::tests::{COUNTDOWN_FRAMES, context, report};
use crate::benchmark::report::{MOSTLY_SMOOTH_SHARE, SMOOTH_SHARE};

fn english(id: &str, fallback: &str) -> String {
    let _ = id;
    fallback.to_string()
}

/// `slow` of 1,000 counted frames at 50 ms (three periods of the 60 FPS budget), the rest at
/// one.
fn run(slow: usize) -> Report {
    let frames = std::iter::repeat_n(100_000.0, COUNTDOWN_FRAMES)
        .chain(std::iter::repeat_n(16_600.0, 1000 - slow))
        .chain(std::iter::repeat_n(50_000.0, slow));
    report(frames)
}

#[test]
fn the_verdict_follows_the_share_of_frames_that_missed_the_budget() {
    for (slow, verdict) in [
        (0, Verdict::Smooth),
        (10, Verdict::Smooth),
        (11, Verdict::MostlySmooth),
        (50, Verdict::MostlySmooth),
        (51, Verdict::Choppy),
        (1000, Verdict::Choppy),
    ] {
        let figures = run(slow).figures().unwrap();
        assert_eq!(figures.verdict(), verdict, "{slow} slow frames of 1000");
    }
    assert!((SMOOTH_SHARE - 0.01).abs() < f64::EPSILON);
    assert!((MOSTLY_SMOOTH_SHARE - 0.05).abs() < f64::EPSILON);
}

#[test]
fn the_summary_leads_with_the_verdict_and_speaks_plainly() {
    let lines = summary(&run(30), &english);
    assert_eq!(
        lines,
        [
            "MOSTLY SMOOTH: 57 FPS ON AVERAGE, JUDGED AGAINST 60 FPS",
            "Slowest moments: 20 FPS (worst 1 in 100 frames)",
            "Stutters: 30 of 1000 frames missed the 16.7 ms frame budget",
            "Mods: none",
        ]
    );
}

#[test]
fn the_lines_and_the_row_name_the_target_while_the_refresh_keeps_its_row() {
    let mut run = run(30);
    run.context.refresh = (120.0, "display");
    run.context.target_fps_hz = Some(60.0);
    let text = copy_text(&run, &english);
    for line in [
        "MOSTLY SMOOTH: 57 FPS ON AVERAGE, JUDGED AGAINST 60 FPS",
        "Stutters: 30 of 1000 frames missed the 16.7 ms frame budget",
        "FRAMES OVER 16.7 MS: 30 OF 1000",
        "SCREEN REFRESH: 120 HZ",
    ] {
        assert!(text.contains(line), "{line} not in:\n{text}");
    }
}

#[test]
fn the_frame_limit_line_is_there_only_when_the_limit_binds() {
    let mut capped = run(0);
    capped.context.frame_limit_hz = Some(60.0);
    let with = summary(&capped, &english).join("\n");
    assert!(
        with.contains("Your frame limit (60 FPS) capped this result."),
        "{with}"
    );
    assert!(with.contains("BENCHMARK OPTIONS"), "{with}");
    let mut roomy = run(0);
    roomy.context.frame_limit_hz = Some(240.0);
    assert!(!summary(&roomy, &english).join("\n").contains("frame limit"));
    assert!(
        !summary(&run(0), &english)
            .join("\n")
            .contains("frame limit")
    );
}

#[test]
fn no_internal_name_reaches_what_a_player_reads() {
    let mut run = run(0);
    run.context.track = Some("Wipeout Pulse: Data\\Environments\\16_Track\\track.vex".into());
    run.context.track_name = None;
    let text = [
        head(&run, &english),
        summary(&run, &english).join("\n"),
        details(&run, &english)
            .all()
            .map(|(k, v)| format!("{k} {v}"))
            .collect::<Vec<_>>()
            .join("\n"),
        copy_text(&run, &english),
    ]
    .join("\n");
    for internal in ["_Track", "track.vex", "16_", "Environments"] {
        assert!(!text.contains(internal), "{internal} in:\n{text}");
    }
    run.context.track_name = Some("Vineta K".into());
    assert!(head(&run, &english).contains("VINETA K"));
}

#[test]
fn the_copied_text_is_pinned_line_for_line() {
    let mut run = run(30);
    run.context.frame_limit_hz = None;
    assert_eq!(
        copy_text(&run, &english),
        "\
Pulse, Vineta K, YOUR SETTINGS
MOSTLY SMOOTH: 57 FPS ON AVERAGE, JUDGED AGAINST 60 FPS
Slowest moments: 20 FPS (worst 1 in 100 frames)
Stutters: 30 of 1000 frames missed the 16.7 ms frame budget
Mods: none

DETAILS
AVERAGE FPS: 56.8
1% LOW FPS: 20.0
0.1% LOW FPS: 20.0
FRAME TIME MEDIAN: 16.60 MS
FRAME TIME P95: 16.60 MS
FRAME TIME P99: 50.00 MS
FRAME TIME WORST: 50.00 MS
FRAMES OVER 16.7 MS: 30 OF 1000
SCREEN REFRESH: 60 HZ
RENDER SCALE: 100
MODS: OFF
DEVICE: linux x86_64
GPU: gpu (d)
BUILD: test
"
    );
}

#[test]
fn a_run_with_mods_names_them_in_the_details() {
    let mut run = run(0);
    run.context.mods = Some(vec![crate::benchmark::ModLoaded {
        id: "oag.energy-turbo".into(),
        version: "1.0.0".into(),
        hash: String::new(),
    }]);
    assert_eq!(summary(&run, &english).last().unwrap(), "Mods: 1 loaded");
    assert!(
        details(&run, &english)
            .machine
            .contains(&("MODS".to_string(), "oag.energy-turbo 1.0.0".to_string()))
    );
}

/// A browser names neither its adapter nor its driver, and a Mali's driver is its adapter's
/// name again: the line reads as what is known, never as empty brackets.
#[test]
fn the_gpu_line_says_only_what_the_platform_named() {
    for (adapter, driver, line) in [
        ("", "", None),
        ("Mali-G78", "Mali-G78", Some("Mali-G78")),
        ("Mali-G78", "", Some("Mali-G78")),
        ("", "Mesa 25", Some("Mesa 25")),
        ("RX 7800", "Mesa 25", Some("RX 7800 (Mesa 25)")),
    ] {
        let mut run = run(0);
        run.context.adapter_name = adapter.into();
        run.context.driver = driver.into();
        let machine = details(&run, &english).machine;
        let shown = machine.iter().find(|(name, _)| name == "GPU");
        assert_eq!(
            shown.map(|(_, value)| value.as_str()),
            line,
            "adapter {adapter:?}, driver {driver:?}"
        );
    }
}

#[test]
fn a_run_too_short_for_figures_says_so() {
    let run = report(std::iter::repeat_n(16_600.0, 5));
    assert_eq!(
        summary(&run, &english),
        ["The race ended before enough frames were drawn"]
    );
    let _ = context();
}

/// The words in the code and the words in `english.toml` are one text: a string id edited in
/// one place only would draw the other's.
#[test]
fn every_word_is_in_english_toml_with_the_same_text() {
    let table: toml::Table = include_str!("../../../../../assets/ui/strings/english.toml")
        .parse()
        .expect("english.toml");
    let table = table["strings"].as_table().expect("a [strings] table");
    for (id, english) in words::ALL {
        let text = table
            .get(*id)
            .unwrap_or_else(|| panic!("{id} is not in english.toml"))
            .get("text")
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| panic!("{id} has no text"));
        assert_eq!(text, *english, "{id}");
    }
}
