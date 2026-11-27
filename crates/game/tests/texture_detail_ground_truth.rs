//! TEXTURE DETAIL reaches the frame a player sees.
//!
//! **`#[ignore]`d and never run in CI.** It needs the Pulse PSP, Pulse PS2 and
//! Wipeout HD disc images under `data/images/` and a GPU adapter. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What only the whole game can say here
//!
//! `crates/mesh/tests/psp_slope_lod.rs` proves the shader shifts the level by
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
//!
//! HD takes the preset on the other path, as a shift of the sampler's own level,
//! and its three pictures differ pairwise too. Pulse PS2's textures author one
//! level (`docs/rendering/texture-level-per-title.md`, "Pulse PS2 in a race"),
//! so there all three are the same picture; a chain built under them would part them.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The far field of Pulse PSP's 16th circuit, past the 256 units a level step needs.
const PSP_LONG_STRAIGHT: [&str; 3] = [
    "--track",
    "Data\\Environments\\16_Track\\track.vex",
    "--pose=-297.96,-50.5,-172.83",
];

fn capture(image: &Path, scratch: &Path, at: &[&str], detail: Option<&str>) -> Vec<u8> {
    let name = image.file_stem().expect("an image name").to_string_lossy();
    let out = scratch.join(format!("{name}-{}.png", detail.unwrap_or("default")));
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
        .args([
            "--render-scale",
            "100",
            "--msaa",
            "off",
            "--motion-blur",
            "off",
        ])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--ticks", "1"])
        .args(at)
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
    oag_texture::png::picture_only(&std::fs::read(&out).expect("reading the screenshot"))
}

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("oag-texture-detail-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creating the scratch directory");
    dir
}

#[test]
#[ignore = "needs a Pulse disc image in data/images/ and a GPU adapter"]
fn each_texture_detail_preset_draws_its_own_picture_of_the_far_field() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let scratch = scratch("psp");
    let shot = |detail| capture(&image, &scratch, &PSP_LONG_STRAIGHT, detail);
    let original = shot(Some("original"));
    let high = shot(Some("high"));
    let maximum = shot(Some("maximum"));
    let default = shot(None);
    std::fs::remove_dir_all(&scratch).ok();

    assert_eq!(default, original, "original is the default preset");
    assert_ne!(original, high, "high must move a level step out");
    assert_ne!(high, maximum, "maximum must reach past high");
    assert_ne!(
        original, maximum,
        "maximum must differ from the recovered law"
    );
}

#[test]
#[ignore = "needs a Wipeout HD disc image in data/images/ and a GPU adapter"]
fn each_texture_detail_preset_draws_its_own_picture_on_hd() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let scratch = scratch("hd");
    let shot = |detail| capture(&image, &scratch, &[], detail);
    let original = shot(Some("original"));
    let high = shot(Some("high"));
    let maximum = shot(Some("maximum"));
    let default = shot(None);
    std::fs::remove_dir_all(&scratch).ok();

    assert!(default == original, "original is the default preset");
    assert!(original != high, "high must shift the sampler's level");
    assert!(high != maximum, "maximum must reach the base level");
    assert!(original != maximum, "maximum must differ from original");
}

#[test]
#[ignore = "needs a Pulse PS2 disc image in data/images/ and a GPU adapter"]
fn pulse_ps2_has_one_level_for_texture_detail_to_move() {
    let Some(image) = oag_testdata::image("pulse-ps2-eu.chd") else {
        return;
    };
    let scratch = scratch("ps2");
    let shot = |detail| capture(&image, &scratch, &[], Some(detail));
    let original = shot("original");
    let high = shot("high");
    let maximum = shot("maximum");
    std::fs::remove_dir_all(&scratch).ok();

    assert!(
        original == high,
        "a PS2 race reads the base at every distance: high moved it"
    );
    assert!(
        original == maximum,
        "a PS2 race reads the base at every distance: maximum moved it"
    );
}
