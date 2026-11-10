//! TRAIL ORDER and TRAIL LENGTH reach the frame a player sees.
//!
//! **`#[ignore]`d and never run in CI.** It needs a Pulse disc image under
//! `data/images/` and a GPU adapter.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What only the whole game can say here
//!
//! `oag_raceplay`'s `trail_order` unit tests prove `Race::ribbons_first` follows
//! the setting. This proves the rest of the road: `--trail-order` is parsed into the
//! render profile, handed to the race, and the frame records the ribbons on the
//! side of the circuit's blended list it names. The craft is posed over the
//! translucent floor patch under the player on Talon's Junction (the one the
//! original's ribbon is veiled by, `docs/ghidra/functions/psp-pulse-usa/exhaust.md`,
//! "The ribbon is queued first"), so the two orders draw different pictures.
//! Dropping any link (the flag, `Race::set_trail_order`, `Race::ribbons_first` in
//! the frame) makes them byte-identical.
//!
//! `original` is also the default: a run with no flag is byte-identical to it.

use std::path::{Path, PathBuf};
use std::process::Command;

fn capture(image: &Path, scratch: &Path, order: Option<&str>) -> Vec<u8> {
    capture_with(image, scratch, order.map(|o| ["--trail-order", o]))
}

/// A posed capture with one extra `--flag value` pair, or none.
fn capture_with(image: &Path, scratch: &Path, flag: Option<[&str; 2]>) -> Vec<u8> {
    let name = flag.map_or("default".to_string(), |[flag, value]| {
        format!("{}-{value}", flag.trim_start_matches('-'))
    });
    let out = scratch.join(format!("trail-{name}.png"));
    let mut command = Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args(["--race", "--no-audio", "--size", "480x272"])
        .args([
            "--render-scale",
            "100",
            "--msaa",
            "off",
            "--motion-blur",
            "off",
        ])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--track", "Data\\Environments\\16_Track\\track.vex"])
        .args(["--ticks", "0", "--pose=-118.39,-49.24,-702.05"])
        // A posed capture runs no tick, so the exhaust is put into a boost state
        // for the ribbon to have a ring to draw.
        .args(["--pose-boost", "0.5"])
        .arg("--screenshot")
        .arg(&out)
        // A settings file of the player's must not decide the order.
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .env("XDG_DATA_HOME", scratch.join("data"));
    if let Some(flag) = flag {
        command.args(flag);
    }
    let status = command.status().expect("running oag-game");
    assert!(status.success(), "oag-game {flag:?}");
    std::fs::read(&out).expect("reading the screenshot")
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-trail-order-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creating the scratch directory");
    dir
}

#[test]
#[ignore = "needs a Pulse disc image in data/images/ and a GPU adapter"]
fn each_trail_order_draws_its_own_picture_over_a_translucent_floor() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let scratch = scratch();
    let original = capture(&image, &scratch, Some("original"));
    let over_floors = capture(&image, &scratch, Some("over-floors"));
    let default = capture(&image, &scratch, None);
    std::fs::remove_dir_all(&scratch).ok();

    assert_eq!(default, original, "original is the default order");
    assert_ne!(
        original, over_floors,
        "over-floors must draw the ribbon after the floor patch"
    );
}

/// `--trail-length hd` reaches the ribbon the frame draws, and `original` is the default: a run
/// with no flag is byte-identical to it. The posed capture fills the ring at the length the
/// race holds, so a length that never reached the race draws the same picture as `original`.
#[test]
#[ignore = "needs a Pulse disc image in data/images/ and a GPU adapter"]
fn the_hd_trail_length_draws_a_longer_ribbon() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let scratch = scratch();
    let default = capture_with(&image, &scratch, None);
    let original = capture_with(&image, &scratch, Some(["--trail-length", "original"]));
    let hd = capture_with(&image, &scratch, Some(["--trail-length", "hd"]));
    std::fs::remove_dir_all(&scratch).ok();

    assert_eq!(default, original, "original is the default length");
    assert_ne!(original, hd, "hd must draw the longer ribbon");
}
