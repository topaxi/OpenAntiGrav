//! A speed pad's boost on Pulse PSP's HUD, on a real disc: the speed bar glows and the widgets
//! smear, and at rest the HUD is the plain one.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game hud_boost_glow_ground_truth:: --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! A PPSSPP frame at a boost's middle (`n = 7` of the 0.8 s timer) has the speed bar's fill washed
//! to near white with a halo, and its text ghosted: the HUD's own copies, and a glow mask stamped
//! under the fill that the bloom, which the frame has anyway, then blooms
//! (`docs/ghidra/functions/psp-pulse-usa/hud-boost-smear.md`). Dropping either half of the
//! wiring - the copies in `oag_hud::boost_smear`, or `Scene::stamp_glow` from
//! `oag_game::hud_views::draw` - leaves the bar crisp and mint, which this reads.

use std::path::{Path, PathBuf};

/// The RGBA bytes of a PNG this project's own writer made (stored deflate blocks).
fn rgba_of(png: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut at = 8;
    let mut zlib = Vec::new();
    while at + 8 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        if &png[at + 4..at + 8] == b"IDAT" {
            zlib.extend_from_slice(&png[at + 8..at + 8 + len]);
        }
        at += 12 + len;
    }
    let mut raw = Vec::new();
    let mut at = 2;
    loop {
        let last = zlib[at] & 1 == 1;
        let len = usize::from(u16::from_le_bytes([zlib[at + 1], zlib[at + 2]]));
        raw.extend_from_slice(&zlib[at + 5..at + 5 + len]);
        at += 5 + len;
        if last {
            break;
        }
    }
    let stride = 1 + width * 4;
    assert_eq!(
        raw.len(),
        stride * height,
        "an RGBA PNG of the frame's size"
    );
    raw.chunks_exact(stride)
        .flat_map(|row| row[1..].iter().copied())
        .collect()
}

fn frame(image: &Path, scratch: &Path, name: &str, boost: Option<&str>) -> Vec<u8> {
    let out = scratch.join(format!("{name}.png"));
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args(["--race", "--no-audio", "--size", "480x272"])
        .args([
            "--render-scale",
            "100",
            "--msaa",
            "off",
            "--screen-filter",
            "off",
        ])
        .args([
            "--anisotropy",
            "off",
            "--motion-blur",
            "off",
            "--ticks",
            "0",
        ])
        .args(["--mode", "time_trial", "--class", "venom"])
        .args(["--track", r"Data\Environments\16_Track\track.vex"])
        .arg("--pose=-118.4,-49.2,-702.0,2.8")
        .args(["--hud-state", "5,2,29.4,39.3,562.9,99.4,56.34"])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if let Some(phase) = boost {
        command.args(["--hud-boost-phase", phase]);
    }
    assert!(command.status().expect("running oag-game").success());
    rgba_of(&std::fs::read(&out).expect("the frame"), 480, 272)
}

/// The mean of channel `channel` over the rectangle `[x0, x1) x [y0, y1)`.
fn mean(rgba: &[u8], channel: usize, x: (usize, usize), y: (usize, usize)) -> f32 {
    let mut sum = 0u64;
    for row in y.0..y.1 {
        for col in x.0..x.1 {
            sum += u64::from(rgba[(row * 480 + col) * 4 + channel]);
        }
    }
    sum as f32 / ((x.1 - x.0) * (y.1 - y.0)) as f32
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-boost-glow-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creating the scratch directory");
    dir
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU adapter"]
fn a_boost_washes_the_speed_bar_and_rest_leaves_the_hud_plain() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = scratch();
    let plain = frame(&image, &scratch, "plain", None);
    let rest = frame(&image, &scratch, "rest", Some("0"));
    let boost = frame(&image, &scratch, "boost", Some("0.45"));
    std::fs::remove_dir_all(&scratch).ok();

    assert_eq!(
        plain, rest,
        "a boost timer of zero is the plain HUD, byte for byte"
    );
    // The speed bar's fill (x 306..430, y 222..244): mint at rest, washed toward white (the red
    // channel climbs from about 125) with the stamp's glow and the copies' own composite.
    let bar = |frame: &[u8]| mean(frame, 0, (310, 420), (224, 242));
    assert!(
        bar(&boost) > bar(&rest) + 40.0,
        "the bar's red went {} -> {}",
        bar(&rest),
        bar(&boost)
    );
    // The lap counter's corner is the same widgets with the same copies and no stamp: ghosted,
    // never brightened past what the copies' alpha leaves of the glyph.
    let corner = |frame: &[u8]| mean(frame, 1, (0, 60), (0, 40));
    assert!(
        (corner(&boost) - corner(&rest)).abs() < 25.0,
        "{} -> {}",
        corner(&rest),
        corner(&boost)
    );
}
