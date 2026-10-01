//! The camera's impact shake is not motion to blur.
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
//! The matrices and the shader are pinned on their own
//! (`race::tests::camera::the_blur_is_given_the_shakes_own_screen_motion`,
//! `post::motion_blur::shake_tests`); this proves the scene hands the blur the
//! map. The craft sits on the grid, still, so the blur has nothing to blur: a
//! frame drawn with the blur on must equal the same frame with it off. The
//! shake is forced at the end of tick 120 and the frame is the one it lands in,
//! the first shaken one, where the velocity buffer used to read the view's
//! jump as the world flying past and smeared the whole picture.
//! Dropping `Scene::render`'s call to `Race::shake_screen_motion` makes the
//! two differ; so does dropping the map from the blur's uniform.

use std::path::{Path, PathBuf};
use std::process::Command;

fn capture(image: &Path, scratch: &Path, name: &str, blur: &str, shake: Option<&str>) -> Vec<u8> {
    let out = scratch.join(format!("{name}.png"));
    let mut command = Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args(["--race", "--no-audio", "--size", "480x272"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", blur, "--ticks", "121"])
        .arg("--screenshot")
        .arg(&out)
        // A settings file of the player's must not decide the preset.
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if let Some(shake) = shake {
        command.args(["--force-shake", shake]);
    }
    let status = command.status().expect("running oag-game");
    assert!(status.success(), "oag-game {name}");
    std::fs::read(&out).expect("reading the screenshot")
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-shake-blur-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creating the scratch directory");
    dir
}

#[test]
#[ignore = "needs a Pulse disc image in data/images/ and a GPU adapter"]
fn the_first_shaken_frame_is_not_blurred_by_the_shake() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = scratch();
    let sharp = capture(&image, &scratch, "sharp", "off", Some("120:1.0"));
    let blurred = capture(&image, &scratch, "blurred", "high", Some("120:1.0"));
    let level = capture(&image, &scratch, "level", "off", None);
    std::fs::remove_dir_all(&scratch).ok();

    assert_ne!(sharp, level, "the forced shake did not reach the picture");
    assert_eq!(
        blurred, sharp,
        "a still craft under a shake was blurred: the shake read as camera motion"
    );
}
