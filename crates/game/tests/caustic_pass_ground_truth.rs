//! Pulse's caustic pass reaches the frame a player sees, on Vostok Reef's PS2 circuit.
//!
//! **`#[ignore]`d and never run in CI.** It needs the Pulse PS2 EU disc image under
//! `data/images/` and a GPU adapter.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What only the whole game can say here
//!
//! `crates/vex/tests/caustic_pass_ground_truth.rs` proves which batches carry the bit. This
//! proves the rest of the road: the rule on the title, the sixteen frames read off the disc by
//! their `.pct` name, the pass's drawable, its recording after the circuit's solid lists and its
//! frame following the world clock. The camera hangs over the flat floor of the circuit's
//! second hall, 80 units up and looking down at a flagged batch (`nodes track25Shape`...). Three
//! captures:
//!
//! - `graphics.water = off` and `original` at one clock differ: the pass draws.
//! - `original` at two clocks 1/15 s (two frames) apart differ: the frame follows the clock.
//! - `off` at one clock twice is the same picture: nothing else moved.
//!
//! Dropping any link (the title's rule, the loader's frames, the scene's drawable, the
//! recording) makes `original` the same bytes as `off`.

use std::path::{Path, PathBuf};
use std::process::Command;

const VOSTOK: &str = r"Data\Environments\08_Track\track.vex";
const OVER_THE_FLOOR: &str = "--camera-pose=-37,80,-640,0,-0.8,0.6,0,0.6,0.8";

/// A settings directory whose Pulse PS2 profile says `water`.
fn config(scratch: &Path, water: &str) -> PathBuf {
    let dir = scratch.join(format!("config-{water}")).join("oag");
    std::fs::create_dir_all(&dir).expect("creating the config directory");
    std::fs::write(
        dir.join("settings.toml"),
        format!("[render_profiles.\"Wipeout Pulse (PS2)\"]\nwater = \"{water}\"\n"),
    )
    .expect("writing the settings");
    dir.parent().expect("has a parent").to_path_buf()
}

fn capture(image: &Path, scratch: &Path, water: &str, clock: &str) -> Vec<u8> {
    let out = scratch.join(format!("vostok-{water}-{clock}.png"));
    let status = Command::new(env!("CARGO_BIN_EXE_oag-game"))
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
        .args(["--track", VOSTOK, "--ticks", "3", "--camera-fov", "70"])
        .args(["--anim-seconds", clock, OVER_THE_FLOOR])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", config(scratch, water))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game water={water} clock={clock}");
    oag_texture::png::picture_only(&std::fs::read(&out).expect("reading the screenshot"))
}

#[test]
#[ignore = "needs the Pulse PS2 EU disc image in data/images/ and a GPU adapter"]
fn vostok_reefs_floor_draws_its_caustics_and_they_follow_the_clock() {
    let Some(image) = oag_testdata::image("data/images/pulse-ps2-eu.chd") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-caustic-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let off = capture(&image, &scratch, "off", "1.01");
    let off_again = capture(&image, &scratch, "off", "1.01");
    let on = capture(&image, &scratch, "original", "1.01");
    let later = capture(&image, &scratch, "original", "1.0767");
    std::fs::remove_dir_all(&scratch).ok();

    assert_eq!(off, off_again, "nothing but the pass moves between runs");
    assert_ne!(
        off, on,
        "original draws the caustic pass where off does not"
    );
    assert_ne!(on, later, "the frame follows the world clock");
}
