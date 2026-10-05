//! A magstrip effect's streaks move on the one animation clock.
//!
//! **`#[ignore]`d and never run in CI.** It needs a Pulse disc image under
//! `data/images/` and a GPU adapter.
//!
//! ```sh
//! just test-data
//! ```
//!
//! `MagEffect1.vex` authors a `v` offset track on its lightning material (key 1
//! to 298, `0` to `1525/256`, a 5 s loop) and `MagEffect2.vex` one on its halo,
//! plus the halo's own spinning `Anim Transform`. The original flickers because
//! of them. Until 2026-10-05 the draw path wrote the node table and never the
//! texture table, so the streaks held one texture phase.
//!
//! The scenery reads the clock too, so the test takes a control pair - the same
//! pose with the effect off (`OAG_MAGFX=off`) - and asks for a difference
//! *outside* the pixels where the control pair differs.

use std::path::{Path, PathBuf};
use std::process::Command;

const WIDTH: usize = 480;
const HEIGHT: usize = 272;
/// The summed absolute RGB change that must show outside the control's own
/// difference. Measured 2026-10-05: 663,606 with the texture table written,
/// 291,268 without it (the halo's spin alone, which moves a lot of pixels by a
/// little).
const FLOOR: u64 = 450_000;

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

/// Talon's Junction's start line, which has no strip: the effect is forced.
fn frame(image: &Path, scratch: &Path, name: &str, effect: &str, clock: &str) -> Vec<u8> {
    let out = scratch.join(format!("{name}.png"));
    let status = Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args(["--race", "--no-audio", "--size", "480x272"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off"])
        .args(["--team", "Assegai"])
        .args(["--track", r"Data\Environments\16_Track\track.vex"])
        .args(["--pose", "6.07,-50.07,-196.05"])
        .args(["--ticks", "0", "--anim-seconds", clock])
        .arg("--screenshot")
        .arg(&out)
        .env("OAG_MAGFX", effect)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game {name}");
    pixels_of(&std::fs::read(&out).expect("reading the screenshot"))
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-mag-floor-scroll-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creating the scratch directory");
    dir
}

/// The summed absolute RGB difference between `a` and `b` where `control_a`
/// and `control_b` agree.
fn difference_outside(a: &[u8], b: &[u8], control_a: &[u8], control_b: &[u8]) -> u64 {
    (0..a.len() / 4)
        .filter(|&p| control_a[p * 4..p * 4 + 4] == control_b[p * 4..p * 4 + 4])
        .map(|p| {
            (0..3)
                .map(|c| u64::from(a[p * 4 + c].abs_diff(b[p * 4 + c])))
                .sum::<u64>()
        })
        .sum()
}

#[test]
#[ignore = "needs a Pulse disc image in data/images/ and a GPU adapter"]
fn the_magstrip_streaks_follow_the_animation_clock() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = scratch();
    // 0.1 s apart: the lightning's `v` offset moves by about 0.12 of the texture.
    let early = frame(&image, &scratch, "early", "on", "1000.0");
    let later = frame(&image, &scratch, "later", "on", "1000.1");
    let control_early = frame(&image, &scratch, "control-early", "off", "1000.0");
    let control_later = frame(&image, &scratch, "control-later", "off", "1000.1");
    std::fs::remove_dir_all(&scratch).ok();

    let moved = difference_outside(&early, &later, &control_early, &control_later);
    assert!(
        moved > FLOOR,
        "the effect's pixels moved by only {moved} (floor {FLOOR}) between two clocks 0.1 s apart: \
         its texture transform is not being written"
    );
}
