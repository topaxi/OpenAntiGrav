//! TEXTURE DETAIL reaches the frame a player sees.
//!
//! **`#[ignore]`d and never run in CI.** It needs a Pulse disc image under
//! `data/images/` and a GPU adapter. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What only the whole game can say here
//!
//! `crates/render/tests/psp_slope_lod.rs` proves the shader shifts the level by
//! the scene uniform's `texlod_shift`. This proves the rest of the road: the
//! setting is parsed, handed to the race, written into the scene uniform each
//! frame, and read back by the same fragment stage. It runs the shipped binary
//! three times at one pose on Talon's Junction's long straight, where the far
//! field is past the 256 units a level step needs, and asserts the three
//! pictures differ pairwise. Dropping any link (the `--texture-detail` flag,
//! `Race::set_texture_detail`, `Fog::with_texture_detail`) makes two of them
//! byte-identical.
//!
//! `original` is also the default: a run with no flag is byte-identical to it.

use std::path::{Path, PathBuf};
use std::process::Command;

fn capture(image: &Path, scratch: &Path, detail: Option<&str>) -> Vec<u8> {
    let out = scratch.join(format!("far-{}.png", detail.unwrap_or("default")));
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
        .args(["--ticks", "1", "--pose=-297.96,-50.5,-172.83"])
        .arg("--screenshot")
        .arg(&out)
        // A settings file of the player's must not decide the preset.
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if let Some(detail) = detail {
        command.args(["--texture-detail", detail]);
    }
    let status = command.status().expect("running oag-game");
    assert!(status.success(), "oag-game --texture-detail {detail:?}");
    std::fs::read(&out).expect("reading the screenshot")
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-texture-detail-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creating the scratch directory");
    dir
}

#[test]
#[ignore = "needs a Pulse disc image in data/images/ and a GPU adapter"]
fn each_texture_detail_preset_draws_its_own_picture_of_the_far_field() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let scratch = scratch();
    let original = capture(&image, &scratch, Some("original"));
    let high = capture(&image, &scratch, Some("high"));
    let maximum = capture(&image, &scratch, Some("maximum"));
    let default = capture(&image, &scratch, None);
    std::fs::remove_dir_all(&scratch).ok();

    assert_eq!(default, original, "original is the default preset");
    assert_ne!(original, high, "high must move a level step out");
    assert_ne!(high, maximum, "maximum must reach past high");
    assert_ne!(
        original, maximum,
        "maximum must differ from the recovered law"
    );
}
