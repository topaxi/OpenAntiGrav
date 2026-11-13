//! The attract demo's race on a real disc: nobody in it, every craft flown by the AI, and one
//! demo ending into the next.
//!
//! **`#[ignore]`d and never run in CI.** It needs a disc image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^demo_ground_truth::/)'
//! ```

use std::path::Path;

use oag_game::demo::{Leave, chain};

/// Demos of this many ticks: past the countdown (272), so every craft has left the grid.
const LENGTH: u32 = 700;

/// What a title's demo is expected to show.
struct Expect {
    /// The screen over the race reads: `Some(logo)` says whether its logo is on the disc.
    screen: Option<bool>,
    /// The spectator director is the camera, and cuts. Only where the circuits author its stations.
    spectator: bool,
}

fn demos_on(image: &Path, demos: u32) -> Vec<chain::Report> {
    let source = image.display().to_string();
    let catalogue = oag_game::remix::catalogue(&source).expect("the title's roster");
    let options = oag_raceplay::Options {
        source,
        ..oag_raceplay::Options::default()
    };
    chain::run(
        &options,
        &catalogue,
        None,
        |track| track.unlock_grid.is_none(),
        7,
        demos,
        LENGTH,
    )
    .expect("the demos fly")
}

/// Two demos in a row: each has no slot a person flies, every craft on its grid travelled, each
/// ran its length and ended `Over`, and the second is another circuit. Fails when the race still
/// seats a person (`humans` is 1) or the camera craft is not flown (`flew` is one short).
fn a_chain_of_demos_has_nobody_in_it(image: &Path, expect: Expect) {
    let Expect { screen, spectator } = expect;
    let rules = oag_source::title::identify(&image.display().to_string())
        .expect("a title")
        .demo
        .rules;
    let reports = demos_on(image, 2);
    assert_eq!(reports.len(), 2, "{reports:#?}");
    for report in &reports {
        println!("{report}");
        assert_eq!(report.humans, 0, "{report}");
        assert!(report.craft > 1, "a demo has a field: {report}");
        assert_eq!(report.moved, report.craft, "every craft flew: {report}");
        assert_eq!(report.ticks, LENGTH, "{report}");
        assert_eq!(report.leave, Leave::Over, "{report}");
        // The screen over the demo reads off the disc: Pulse's `Demo InGame` is two rules, the
        // logo and the pulsing text. Fails when the XML, its logo or its string is not read.
        if let Some(logo) = screen {
            let drawn = report
                .screen
                .unwrap_or_else(|| panic!("no demo screen: {report:?}"));
            assert!(drawn.fills >= 2, "the two rules: {drawn:?}");
            // Pure's disc holds `demo_logo.mip` as an empty entry: nothing to draw, nothing in its place.
            assert_eq!(
                drawn.sprites >= 1,
                logo,
                "the logo: {drawn:?} {:?}",
                report.screen_notes
            );
            assert!(drawn.texts >= 1, "Press any button: {drawn:?}");
        } else {
            assert_eq!(report.screen, None, "{report}");
        }
        println!(
            "{report}: {} cut(s), spectator {}",
            report.cuts, report.spectated
        );
        if spectator {
            assert!(
                report.spectated,
                "the spectator director is the camera: {report:?}"
            );
            assert!(report.cuts >= 2, "it takes over and cuts: {report:?}");
        }
        // The field is the rules' (`Rules::craft`): fails when the data or the grid moves.
        assert_eq!(report.craft, rules.craft.value as usize, "{report}");
    }
    assert_ne!(
        reports[0].track, reports[1].track,
        "the next demo is another circuit"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_demos_have_nobody_in_them() {
    if let Some(image) = oag_testdata::image("pulse-psp-eu.chd") {
        a_chain_of_demos_has_nobody_in_it(
            &image,
            Expect {
                screen: Some(true),
                spectator: true,
            },
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_demos_have_nobody_in_them() {
    if let Some(image) = oag_testdata::image("pure-psp-eu.chd") {
        a_chain_of_demos_has_nobody_in_it(
            &image,
            Expect {
                screen: Some(false),
                spectator: false,
            },
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_demos_have_nobody_in_them() {
    if let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") {
        a_chain_of_demos_has_nobody_in_it(
            &image,
            Expect {
                screen: None,
                spectator: false,
            },
        );
    }
}

/// `oag-game --demo --dry-run`, the headless run: it flies demos back to back, one line each, and
/// the run's own report says nobody is in them.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_headless_demo_run_chains_demos_with_nobody_in_them() {
    let Some(image) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("demo-dry-run");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(&image)
        .args(["--no-audio", "--demo", "--dry-run", "--ticks", "700"])
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY")
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .expect("running oag-game");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{:?}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("demo "))
        .collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    for line in lines {
        assert!(line.contains("0 human slots"), "{line}");
        assert!(
            line.contains("8 craft") && line.contains("8 flew"),
            "{line}"
        );
        assert!(line.ends_with("700 ticks (over)"), "{line}");
    }
}

/// `oag-game --wallpaper --windows 3 --screenshot`: a wallpaper on three outputs is three cameras,
/// each on a craft of its own, from the one simulation (`Race::watch_in_windows`). Fails if a
/// window is not written, or two windows show the same picture (a seat dealt as a person, or
/// every window on the player's camera).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_wallpaper_on_three_outputs_shows_three_cameras() {
    let Some(image) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("demo-wallpaper-windows");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let shot = scratch.join("wallpaper.png");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(&image)
        .args([
            "--no-audio",
            "--wallpaper",
            "--windows",
            "3",
            "--ticks",
            "600",
            "--screenshot",
        ])
        .arg(&shot)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY")
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .expect("running oag-game");
    assert!(
        output.status.success(),
        "{:?}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let pixels: Vec<Vec<u8>> = (1..=3)
        .map(|window| {
            let path = scratch.join(format!("wallpaper-window{window}.png"));
            let file =
                std::fs::File::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let mut reader = png::Decoder::new(std::io::BufReader::new(file))
                .read_info()
                .expect("a PNG");
            let mut buffer = vec![0; reader.output_buffer_size().expect("a size")];
            let info = reader.next_frame(&mut buffer).expect("a frame");
            buffer.truncate(info.buffer_size());
            buffer
        })
        .collect();
    for (a, b) in [(0, 1), (1, 2), (0, 2)] {
        assert_eq!(pixels[a].len(), pixels[b].len());
        let differing = pixels[a]
            .iter()
            .zip(&pixels[b])
            .filter(|(x, y)| x.abs_diff(**y) > 24)
            .count();
        assert!(
            differing * 4 > pixels[a].len(),
            "windows {} and {} are much the same picture ({differing} of {} bytes differ)",
            a + 1,
            b + 1,
            pixels[a].len()
        );
    }
}

/// 2048 and Omega author their demo in `DemoConfig.xml` (byte-identical on both): the menu waits
/// 30 s, a demo runs 120 s and flies the circuit `tower`. The boot reads it, the timing takes its
/// seconds over the title's own, and a demo is on that circuit. Fails when the file is not read or
/// not laid over the title's values.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007 and data/extracted/ps4"]
fn the_authored_demo_config_of_2048_and_omega_sets_their_timing_and_circuit() {
    let extracted = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
    for source in ["vita/PCSF00007", "ps4"] {
        let path = extracted.join(source);
        if !path.exists() {
            eprintln!("skipped: {source} is not extracted");
            continue;
        }
        let source = path.to_string_lossy().into_owned();
        let mut opened = oag_source::title::open_source(&source, Vec::new(), Vec::new())
            .unwrap_or_else(|e| panic!("{source}: {e:#}"));
        let config =
            oag_game::boot::demo_config::load(&mut opened.archives, opened.title, &mut Vec::new())
                .unwrap_or_else(|| panic!("{source}: no DemoConfig.xml read"));
        assert_eq!(config.start_seconds, Some(30.0), "{source}");
        assert_eq!(config.end_seconds, Some(120.0), "{source}");
        assert_eq!(config.tracks, ["tower"], "{source}");
        let timing = oag_game::demo::Timing::of(&opened.title.demo, Some(&config));
        assert_eq!(
            timing.idle,
            30 * 60,
            "{source}: the authored 30 s, not the inherited 60"
        );
        let catalogue = oag_game::remix::catalogue(&source).expect("the roster");
        let options = oag_raceplay::Options {
            source,
            ..oag_raceplay::Options::default()
        };
        let reports =
            chain::run(&options, &catalogue, Some(&config), |_| true, 3, 2, 700).expect("demos");
        assert_eq!(reports.len(), 2);
        for report in &reports {
            assert_eq!(report.track, "tower", "the authored cycle: {report}");
            assert_eq!(report.humans, 0, "{report}");
        }
    }
}
