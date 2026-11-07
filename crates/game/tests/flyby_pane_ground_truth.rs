//! Platinum Rush's opening shot draws one surface per window pane.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game flyby_pane_ground_truth:: --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! The circuit authors the start building's panes in two groups, exactly coplanar and cut in
//! different triangles, and the original's section mask draws one of them in this shot. Drawn
//! together, the depth test picks a winner per pixel from the last float bit, and a live camera
//! crawls. The shot holds its camera, so the same frame at two blends differs only where depth
//! ties: this is `0.00` with one group and `45.72` with both
//! (`docs/gameplay/race-intro.md`, "The opening shot's window panes stipple on Platinum Rush").

use std::path::{Path, PathBuf};

/// The left dome's panes in a 960x544 capture: `x0, y0, x1, y1`.
const PANES: (usize, usize, usize, usize) = (250, 290, 365, 370);

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("flyby-pane-{name}"));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// The held shot at `alpha`, RGBA.
fn held_shot(image: &Path, track: &str, name: &str, alpha: &str) -> Vec<u8> {
    let dir = scratch(name);
    let out = dir.join(format!("{alpha}.png"));
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args(["--race", "--no-audio", "--size", "960x544"])
        .args(["--track", track, "--ticks", "0", "--intro-ticks", "5"])
        .args(["--draw-alpha", alpha])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .output()
        .expect("oag-game ran");
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let file = std::fs::File::open(&out).unwrap_or_else(|e| panic!("{}: {e}", out.display()));
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .expect("a PNG");
    let mut pixels = vec![0; reader.output_buffer_size().expect("a buffer size")];
    let info = reader.next_frame(&mut pixels).expect("a frame");
    assert_eq!((info.width, info.height), (960, 544));
    assert_eq!(info.color_type, png::ColorType::Rgba);
    pixels
}

/// The mean over [`PANES`] of the summed absolute RGB difference.
fn pane_difference(a: &[u8], b: &[u8]) -> f32 {
    let (x0, y0, x1, y1) = PANES;
    let mut sum = 0u64;
    for y in y0..y1 {
        for x in x0..x1 {
            let at = (y * 960 + x) * 4;
            for c in 0..3 {
                sum += u64::from(a[at + c].abs_diff(b[at + c]));
            }
        }
    }
    sum as f32 / ((x1 - x0) * (y1 - y0)) as f32
}

fn the_held_shot_ties_nowhere(track: &str, name: &str) {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let difference = pane_difference(
        &held_shot(&image, track, name, "0.5"),
        &held_shot(&image, track, name, "1.0"),
    );
    assert!(
        difference < 0.01,
        "{track}: the panes moved by {difference}"
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn platinum_rush_white_draws_one_surface_per_pane() {
    the_held_shot_ties_nowhere(r"Data\Environments\13_Track\track.vex", "white");
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn platinum_rush_black_draws_one_surface_per_pane() {
    the_held_shot_ties_nowhere(r"Data\Environments\13_Track\track_reversed.vex", "black");
}
