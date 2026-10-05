//! A laid mine's hull texture scrolls on the one animation clock.
//!
//! **`#[ignore]`d and never run in CI.** It needs a Pulse disc image under
//! `data/images/` and a GPU adapter.
//!
//! ```sh
//! just test-data
//! ```
//!
//! `Pulse_Mine.vex` authors one texture-offset track (`v`, `0` to `-1` over key
//! 1 to 60, a 1 s loop) and one `Anim Transform` (a 119-frame loop). Until
//! 2026-10-05 `write_one_kind` uploaded the node table and never the texture
//! table, so the mine's texture held phase 0.
//!
//! The node animation moves the same pixels, so two clocks are chosen to cancel
//! it: 59.5 s apart is thirty node loops (`30 * 119/60`), which leaves the node
//! pose identical, and half a texture loop, which moves the texture by half its
//! height. Every pixel that differs between the two frames is then the texture
//! scroll, and a frame with no mine at all is the control for the scenery.

use std::path::{Path, PathBuf};
use std::process::Command;

const WIDTH: usize = 480;
const HEIGHT: usize = 272;
/// The summed absolute RGB change over the mine cluster's rows that must show.
/// Measured 2026-10-05: 791,563 with the texture table written, 497 without.
const FLOOR: u64 = 200_000;
/// What the no-mine control may change: the scenery between the two clocks.
/// Measured 2026-10-05: 896.
const CONTROL_CEILING: u64 = 20_000;
/// The region of the frame the five laid mines occupy.
const REGION: (usize, usize, usize, usize) = (80, 90, 400, 200);

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

/// A top-down camera over the mines laid at the start line, five of them
/// within 4 units of each other a tick after the drop.
fn frame(image: &Path, scratch: &Path, name: &str, mine: bool, clock: &str) -> Vec<u8> {
    let out = scratch.join(format!("{name}.png"));
    let script = scratch.join(format!("{name}.inputs"));
    // The control presses nothing: with no `--give` a square press would fire
    // the slot's own Turbo and move the craft.
    let text = if mine {
        "420 none\n1 square\n100 cross\n"
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
        .args(["--camera-pose", "8,-46,-195.9,0,-1,0,0,0,-1"])
        .arg("--input-script")
        .arg(&script)
        .args(["--ticks", "470", "--anim-seconds", clock])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if mine {
        command.args(["--give", "mine"]);
    }
    let status = command.status().expect("running oag-game");
    assert!(status.success(), "oag-game {name}");
    pixels_of(&std::fs::read(&out).expect("reading the screenshot"))
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-mine-scroll-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creating the scratch directory");
    dir
}

/// The summed absolute RGB difference between `a` and `b` inside [`REGION`].
fn difference(a: &[u8], b: &[u8]) -> u64 {
    let (x0, y0, x1, y1) = REGION;
    (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (y * WIDTH + x) * 4))
        .map(|p| {
            (0..3)
                .map(|c| u64::from(a[p + c].abs_diff(b[p + c])))
                .sum::<u64>()
        })
        .sum()
}

#[test]
#[ignore = "needs a Pulse disc image in data/images/ and a GPU adapter"]
fn a_laid_mines_texture_follows_the_animation_clock() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = scratch();
    let early = frame(&image, &scratch, "early", true, "1000.0");
    let later = frame(&image, &scratch, "later", true, "1059.5");
    let control_early = frame(&image, &scratch, "control-early", false, "1000.0");
    let control_later = frame(&image, &scratch, "control-later", false, "1059.5");
    std::fs::remove_dir_all(&scratch).ok();

    let scenery = difference(&control_early, &control_later);
    assert!(
        scenery < CONTROL_CEILING,
        "the scenery alone moved {scenery} between the two clocks (ceiling {CONTROL_CEILING}), \
         so a difference with the mine in view proves nothing"
    );
    let moved = difference(&early, &later);
    assert!(
        moved > FLOOR,
        "the mines moved by only {moved} (floor {FLOOR}) between two clocks whose node poses \
         match: their texture transform is not being written"
    );
}
