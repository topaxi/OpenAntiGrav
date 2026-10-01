//! A laid Bomb's own `Anim Transform`s play, on the one animation clock.
//!
//! **`#[ignore]`d and never run in CI.** It needs a Pulse disc image under
//! `data/images/` and a GPU adapter.
//!
//! ```sh
//! just test-data
//! ```
//!
//! `Pulse_Bomb.vex` authors two animated transforms: `orbit`, the ring, which
//! tumbles about its own `X` once in three seconds, and `bomb`, the canister's
//! own node. The original reads them off `g_ingame->0x40` like every other
//! animated node (`anim-transform.md`), and a launch seen from the chase camera
//! shows the ring edge-on as a thin vertical shaft or tilted, by the clock's
//! phase. Until 2026-10-01 this engine drew the bomb at its time-zero pose
//! only: a wide flat ring, whatever the clock said.
//!
//! The scenery's own animations also read the clock, so two frames at two
//! clocks differ everywhere. The test therefore takes a **control** pair - the
//! same scenario with no bomb - and asks for a difference *outside* the pixels
//! where the control pair differs. Dropping the bomb's node-animation write
//! leaves that set empty.

use std::path::{Path, PathBuf};
use std::process::Command;

const WIDTH: usize = 480;
const HEIGHT: usize = 272;
/// One tick after the fire, where the canister and its ring are in view.
const TICK: &str = "405";
/// Pixels that must change outside the control's own difference. Measured
/// 2026-10-01: 18,778 with the write, 2,201 without it - the bomb's glow
/// bloom-spills onto scenery pixels the control never touched, which is why the
/// control mask alone does not give zero.
const FLOOR: usize = 8_000;

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
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../verification/scenarios/weapon-after-go.inputs");
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
    let dir = std::env::temp_dir().join(format!("oag-bomb-orbit-{}", std::process::id()));
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
fn a_laid_bombs_ring_follows_the_animation_clock() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = scratch();
    // The ring is flat at clock 0 and edge-on and vertical at 0.75 s.
    let flat = frame(&image, &scratch, "flat", Some("bomb"), "0.0");
    let upright = frame(&image, &scratch, "upright", Some("bomb"), "0.75");
    let control_flat = frame(&image, &scratch, "control-flat", None, "0.0");
    let control_upright = frame(&image, &scratch, "control-upright", None, "0.75");
    std::fs::remove_dir_all(&scratch).ok();

    let bomb = differing_outside(&flat, &upright, &control_flat, &control_upright);
    assert!(
        bomb > FLOOR,
        "only {bomb} pixels (floor {FLOOR}) of the bomb changed between clock 0 and 0.75 s: its Anim Transforms \
         are not being written"
    );
}
