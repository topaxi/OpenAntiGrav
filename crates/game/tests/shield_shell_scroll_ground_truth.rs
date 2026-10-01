//! A raised Shield's shell scrolls its texture on the one animation clock.
//!
//! **`#[ignore]`d and never run in CI.** It needs a Pulse disc image under
//! `data/images/` and a GPU adapter.
//!
//! ```sh
//! just test-data
//! ```
//!
//! `shipshield.vex` authors a `u` offset track on its one material (frame 1 to
//! 59, `0` to `251/256`, a 59-frame loop), and the original runs it off
//! `g_ingame->0x40` like every other texture transform: the offset read out of
//! three GE dumps follows the track (`shield-pickup.md`, 2026-10-01). Until that
//! date the shell was never given a `write_anims`, so its texture stood still
//! under the breathing alpha.
//!
//! The scenery's own animations read the clock too, so the test takes a control
//! pair - the same scenario with no shield - and asks for a difference
//! *outside* the pixels where the control pair differs.

use std::path::{Path, PathBuf};
use std::process::Command;

const WIDTH: usize = 480;
const HEIGHT: usize = 272;
/// 105 ticks after the fire: the shell has settled (colour and swell are within
/// 1e-4 of rest after about 40) and is well inside the pickup's lifetime.
const TICK: &str = "526";
/// Pixels that must change outside the control's own difference. Measured
/// 2026-10-01: 8,486 with the shell's `write_anims`, 276 without it.
const FLOOR: usize = 2_500;

/// The RGBA bytes of a PNG this project's own writer made (stored deflate
/// blocks, no filtering - see `glow_stamp_ground_truth.rs`).
fn pixels_of(png: &[u8]) -> Vec<u8> {
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
    let stride = 1 + WIDTH * 4;
    assert_eq!(
        raw.len(),
        stride * HEIGHT,
        "an RGBA PNG of the frame's size"
    );
    raw.chunks_exact(stride)
        .flat_map(|row| row[1..].iter().copied())
        .collect()
}

fn frame(image: &Path, scratch: &Path, name: &str, give: Option<&str>, clock: &str) -> Vec<u8> {
    let out = scratch.join(format!("{name}.png"));
    let script = scratch.join("still.inputs");
    // The control presses nothing: with no `--give` a square press fires the
    // slot's own Turbo and moves the craft.
    let text = if give.is_some() {
        "420 none\n1 square\n600 none\n"
    } else {
        "1100 none\n"
    };
    std::fs::write(&script, text).expect("writing the script");
    let mut command = Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args(["--race", "--no-audio", "--size", "480x272"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off"])
        .args([
            "--mode",
            "time_trial",
            "--team",
            "Assegai",
            "--class",
            "venom",
        ])
        .args(["--track", r"Data\Environments\16_Track\track.vex"])
        .arg("--input-script")
        .arg(&script)
        .args(["--ticks", TICK, "--anim-seconds", clock])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if let Some(weapon) = give {
        command.args(["--give", weapon]);
    }
    let status = command.status().expect("running oag-game");
    assert!(status.success(), "oag-game {name}");
    pixels_of(&std::fs::read(&out).expect("reading the screenshot"))
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-shield-scroll-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creating the scratch directory");
    dir
}

/// How many pixels differ between `a` and `b` where `control_a` and `control_b`
/// agree.
fn differing_outside(a: &[u8], b: &[u8], control_a: &[u8], control_b: &[u8]) -> usize {
    (0..a.len() / 4)
        .filter(|&p| {
            let at = p * 4..p * 4 + 4;
            control_a[at.clone()] == control_b[at.clone()] && a[at.clone()] != b[at]
        })
        .count()
}

#[test]
#[ignore = "needs a Pulse disc image in data/images/ and a GPU adapter"]
fn a_raised_shields_texture_follows_the_animation_clock() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = scratch();
    // A quarter of the 59-frame loop apart: the offset moves by about a quarter
    // of the texture's width.
    let early = frame(&image, &scratch, "early", Some("shield"), "1000.0");
    let later = frame(&image, &scratch, "later", Some("shield"), "1000.25");
    let control_early = frame(&image, &scratch, "control-early", None, "1000.0");
    let control_later = frame(&image, &scratch, "control-later", None, "1000.25");
    std::fs::remove_dir_all(&scratch).ok();

    let shell = differing_outside(&early, &later, &control_early, &control_later);
    assert!(
        shell > FLOOR,
        "only {shell} pixels (floor {FLOOR}) of the shell changed between two clocks a quarter loop apart: \
         its texture transform is not being written"
    );
}
