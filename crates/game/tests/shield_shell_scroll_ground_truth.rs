//! A raised Shield's shell animates on the one animation clock, on every title that draws one.
//!
//! **`#[ignore]`d and never run in CI.** It needs the title's disc image or extract under
//! `data/` and a GPU adapter.
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
//! The scenery's own animations read the clock too, so each test takes a control
//! pair - the same scenario with no shield - and asks for a difference
//! *outside* the pixels where the control pair differs.
//!
//! HD's, Omega's and 2048's shells scroll in the shader, on the scene block's clock
//! (`docs/ghidra/functions/ps3-hdfury-eu/shield.md`, "The lattice scrolls"), and the
//! magnetic shell of a 2048 craft on a vertex scroll track. A shell whose drawable reads no
//! clock stands still, so the pixel count is zero without the wiring.

use std::path::Path;
use std::process::Command;

const WIDTH: usize = 480;
const HEIGHT: usize = 272;
/// 105 ticks after the fire: the shell has settled (colour and swell are within
/// 1e-4 of rest after about 40) and is well inside the pickup's lifetime.
const TICK: &str = "526";
/// Pixels that must change outside the control's own difference. Measured
/// 2026-10-01: 8,486 with the shell's `write_anims`, 276 without it.
const FLOOR: usize = 2_500;
/// A channel moving by more than this between the two clocks is the shell's.
const MOVED: u8 = 16;
/// Where the control pair's channels move by no more than this, the movement is not scenery's:
/// Omega's and HD's scenery animates nearly everywhere, so an exact match would drop the craft.
const STILL: u8 = 8;

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

/// The arguments of the Pulse scenario, past the ones every title shares.
const PULSE: &[&str] = &[
    "--team",
    "Assegai",
    "--class",
    "venom",
    "--track",
    r"Data\Environments\16_Track\track.vex",
];

fn frame(
    image: &Path,
    scratch: &Path,
    name: &str,
    give: Option<&str>,
    clock: &str,
    scenario: &[&str],
) -> Vec<u8> {
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
        .args([
            "--race",
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--size",
            "480x272",
        ])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off"])
        .args(["--mode", "time_trial"])
        .args(scenario)
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

fn scratch(title: &str) -> oag_testdata::scratch::TempDir {
    oag_testdata::scratch::dir(&format!("shield-scroll-{title}"))
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

/// How many pixels the shield moves between `a` and `b` beyond what the control pair moves.
fn moving_beyond(a: &[u8], b: &[u8], control_a: &[u8], control_b: &[u8]) -> usize {
    let most = |a: &[u8], b: &[u8]| a.iter().zip(b).map(|(&a, &b)| a.abs_diff(b)).max();
    (0..a.len() / 4)
        .filter(|&p| {
            let at = p * 4..p * 4 + 3;
            most(&a[at.clone()], &b[at.clone()]) > Some(MOVED)
                && most(&control_a[at.clone()], &control_b[at.clone()]) <= Some(STILL)
        })
        .count()
}

/// The four frames of one title's scenario: shield at two clocks, no shield at the same two.
fn four_frames(image: &Path, title: &str, scenario: &[&str]) -> [Vec<u8>; 4] {
    let scratch = scratch(title);
    // A quarter of Pulse's 59-frame loop apart, and not a multiple of the third of a second
    // HD's three-per-second scroll wraps in.
    let shot = |name, give, clock| frame(image, &scratch, name, give, clock, scenario);
    let frames = [
        shot("early", Some("shield"), "1000.0"),
        shot("later", Some("shield"), "1000.25"),
        shot("control-early", None, "1000.0"),
        shot("control-later", None, "1000.25"),
    ];
    std::fs::remove_dir_all(&scratch).ok();
    frames
}

#[test]
#[ignore = "needs a Pulse disc image in data/images/ and a GPU adapter"]
fn a_raised_shields_texture_follows_the_animation_clock() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let [early, later, control_early, control_later] = four_frames(&image, "pulse", PULSE);
    let shell = differing_outside(&early, &later, &control_early, &control_later);
    assert!(
        shell > FLOOR,
        "only {shell} pixels (floor {FLOOR}) of the shell changed between two clocks a quarter loop apart: \
         its texture transform is not being written"
    );
}

fn assert_the_lattice_moves(source: &Path, title: &str, scenario: &[&str], floor: usize) {
    let [early, later, control_early, control_later] = four_frames(source, title, scenario);
    let moving = moving_beyond(&early, &later, &control_early, &control_later);
    assert!(
        moving > floor,
        "{title}: only {moving} pixels (floor {floor}) of the shield moved between two clocks: \
         its scroll is not wired or its drawable reads no clock"
    );
}

#[test]
#[ignore = "needs the decrypted HD disc image in data/images/ and a GPU adapter"]
fn hds_raised_shield_lattice_moves() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    // Measured: 12,598 pixels; 0 with the shell off the scene clock.
    assert_the_lattice_moves(&image, "hd", &[], 3_000);
}

#[test]
#[ignore = "needs the Omega package in data/images/ and a GPU adapter"]
fn omegas_raised_shield_lattice_moves() {
    let Some(image) = oag_testdata::exact("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    // Measured: 9,249 pixels; 0 with the shell off the scene clock.
    assert_the_lattice_moves(&image, "omega", &[], 2_500);
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/ and a GPU adapter"]
fn the_2048_magnetic_shields_wires_move() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    // The default craft flies the magnetic shell. Measured: 1,300 pixels, 0 without the track.
    assert_the_lattice_moves(&source, "2048-magnetic", &[], 400);
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/ and a GPU adapter"]
fn the_2048_hd_content_shields_lightning_moves() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    assert_the_lattice_moves(&source, "2048-hd", &["--team", "Assegai"], 1_000);
}
