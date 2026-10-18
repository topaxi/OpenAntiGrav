//! The camera's lens - its impact shake and its field of view - is not motion
//! to blur.
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
//! `race::tests::camera::the_blur_is_given_the_boost_kicks_own_screen_motion`,
//! `post::motion_blur::shake_tests`); this proves the scene hands the blur the
//! map. The craft sits on the grid, still, so the blur has nothing to blur: a
//! frame drawn with the blur on must equal the same frame with it off. Each
//! lens change is forced at the end of tick 120 and the frame is the one it
//! lands in:
//!
//! - **the shake**, the first shaken frame, where the velocity buffer used to
//!   read the view's jump as the world flying past and smeared the whole
//!   picture;
//! - **the boost's field-of-view kick**, opened in one tick, where the buffer
//!   used to read the widening field as everything rushing toward the middle
//!   of the screen, and smeared the player's own hull back over the track on
//!   every speed pad.
//!
//! Dropping `Scene::render`'s call to `Race::lens_screen_motion` makes both
//! pairs differ; so does dropping the map from the blur's uniform, and leaving
//! the previous tick's projection out of the map makes the kick's pair differ.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A lens change forced at the end of tick 120: the flag and its `TICK:VALUE`.
type Forced<'a> = Option<(&'a str, &'a str)>;

fn capture(image: &Path, scratch: &Path, name: &str, blur: &str, forced: Forced<'_>) -> Vec<u8> {
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
        // A settings file of the player's must not decide the preset, and the
        // kick's default strength is what a fresh install widens by.
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if let Some((flag, value)) = forced {
        command.args([flag, value]);
    }
    let status = command.status().expect("running oag-game");
    assert!(status.success(), "oag-game {name}");
    std::fs::read(&out).expect("reading the screenshot")
}

/// The captures' directory, removed when the test ends, failing or not.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("oag-lens-blur-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("creating the scratch directory");
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// One lens change's pair against the unforced frame: it reached the picture,
/// and the blur left that picture as it was.
fn check_lens_change(image: &Path, scratch: &Path, level: &[u8], forced: (&str, &str)) {
    let (flag, _) = forced;
    let name = flag.trim_start_matches("--force-");
    let sharp = capture(
        image,
        scratch,
        &format!("{name}-sharp"),
        "off",
        Some(forced),
    );
    let blurred = capture(
        image,
        scratch,
        &format!("{name}-blurred"),
        "high",
        Some(forced),
    );
    assert_ne!(sharp, level, "{flag} did not reach the picture");
    assert!(
        blurred == sharp,
        "a still craft under {flag} was blurred: the lens change read as camera motion"
    );
}

#[test]
#[ignore = "needs a Pulse disc image in data/images/ and a GPU adapter"]
fn a_still_craft_under_a_changing_lens_is_not_blurred() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = Scratch::new();
    let level = capture(&image, &scratch.0, "level", "off", None);
    check_lens_change(&image, &scratch.0, &level, ("--force-shake", "120:1.0"));
    check_lens_change(
        &image,
        &scratch.0,
        &level,
        ("--force-boost-kick", "120:1.0"),
    );
}
