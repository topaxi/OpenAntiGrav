//! A split screen's views draw the same picture however they are recorded: one after another
//! (`OAG_SPLIT_RECORDING=interleaved`, prepare, record and submit per view), every view prepared
//! and then recorded last view first on one thread (`reversed`), and recorded at once on threads
//! of their own (`threaded`, the default). See `oag_raceplay::Recording`.
//!
//! **What fails it**: anything a view's recording reads that a later view's prepare changes, and
//! any write a later view's prepare makes that is not kept for its own submission. The first
//! shape of the second was a drawable with no slot on the shared matrix buffer writing its own
//! uniforms straight to the queue: Pure's magstrip effect drew view 0 from view 1's camera at
//! tick 1500 of an autopilot race, and only there of 34 stills and 52 weapon stills.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game split_recording_ground_truth:: --run-ignored all
//! ```

/// One `--screenshot` of `image` with `args`, recorded `recording`, as PNG bytes.
fn capture(
    dir: &std::path::Path,
    image: &std::path::Path,
    recording: &str,
    args: &[&str],
) -> Vec<u8> {
    std::fs::create_dir_all(dir).expect("a temp directory");
    let shot = dir.join(format!("{recording}.png"));
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args([
            "--no-audio",
            "--race",
            "--mode",
            "single_race",
            "--size",
            "640x400",
        ])
        .args(args)
        .arg("--screenshot")
        .arg(&shot)
        .env("OAG_SPLIT_RECORDING", recording)
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
    std::fs::read(&shot).expect("the screenshot was written")
}

/// The three recordings of one frame, byte for byte.
fn same_every_way(name: &str, image: &std::path::Path, args: &[&str]) {
    let dir = std::env::temp_dir()
        .join("oag-split-recording-ground-truth")
        .join(name);
    let interleaved = capture(&dir, image, "interleaved", args);
    for recording in ["reversed", "threaded"] {
        assert!(
            capture(&dir, image, recording, args) == interleaved,
            "{name}: the {recording} recording draws a different picture than the interleaved one \
             (both in {})",
            dir.display()
        );
    }
}

/// Pure, two views, an autopilot race at tick 1500: the frame the slot-less magstrip effect
/// leaked view 1's camera into view 0 on.
#[test]
#[ignore = "needs data/images/pure-psp-eu.chd and a GPU"]
fn pure_two_views_draw_the_same_however_they_are_recorded() {
    let Some(image) = oag_testdata::image("data/images/pure-psp-eu.chd") else {
        return;
    };
    same_every_way(
        "pure-2h",
        &image,
        &[
            "--players",
            "2",
            "--split-cut",
            "horizontal",
            "--autopilot",
            "--ticks",
            "1500",
        ],
    );
}

/// Pulse, four views, a Rocket in flight: the effect pipelines (smoke, blasts, exhaust) each
/// view uploads into one buffer with its own camera.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn pulse_four_views_with_a_rocket_draw_the_same_however_they_are_recorded() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    same_every_way(
        "pulse-4",
        &image,
        &[
            "--players",
            "4",
            "--give",
            "Rocket",
            "--hold",
            "cross",
            "--press",
            "square",
            "--ticks",
            "520",
        ],
    );
}
