//! Pure's front-end text steps each glyph's ink edge up to a whole pixel, on a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game pure_menu_pens_ground_truth:: --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! A PPSSPP frame of Track Selection (software renderer, 480x272) puts the first ten glyphs of
//! the circuit panel's `RACE RECORD` label, a `Stats` face widget at scale 0.8 with an advance of
//! 9 texels (7.2 px), 8 px apart in a row (250, 258, 266, 274, then 289 after the space, 297 ...
//! 329): the pen is stepped up to a whole pixel before each glyph, so the label spans 79 px where
//! the unsnapped pen spans 73 (`docs/ui/menus-original.md`, "Pure's text pens step up to whole
//! pixels"). Dropping `MenuSkin::snapped_pens` from the front end's renderer draws the 73.

use std::path::Path;

/// The ink-left column of each glyph of the label in the original's frame (the pair's
/// `theirs.png`, read as `docs/ui/menus-original.md` says).
const RACE_RECORD_ORIGINAL: [u32; 10] = [250, 258, 266, 274, 289, 297, 305, 313, 321, 329];

/// The first column of each run of inked columns in rows 171 to 179, between x 244 and 340.
fn glyph_starts(png: &[u8]) -> Vec<u32> {
    let image = oag_texture::replacer::decode_png(png).expect("the screenshot decodes");
    assert_eq!((image.width, image.height), (480, 272));
    let inked = |x: u32| {
        (171..180).any(|y| {
            let at = ((y * image.width + x) * 4) as usize;
            let rgb = &image.rgba[at..at + 3];
            (255 * 3 - rgb.iter().map(|&c| i32::from(c)).sum::<i32>()) / 3 > 60
        })
    };
    let mut starts = Vec::new();
    for x in 244..340 {
        if inked(x) && !inked(x - 1) {
            starts.push(x);
        }
    }
    starts
}

fn still(image: &Path, scratch: &Path) -> Vec<u8> {
    std::fs::create_dir_all(scratch).expect("a scratch directory");
    let out = scratch.join("track-selection.png");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args([
            "--no-audio",
            "--size",
            "480x272",
            "--menu-page",
            "pure-chain:arcade:0:0",
            "--menu-picker-seconds",
            "5.873",
            "--screenshot",
        ])
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("OAG_DLC", scratch.join("no-dlc"))
        .status()
        .expect("running oag-game");
    assert!(
        status.success(),
        "oag-game --menu-page on {}",
        image.display()
    );
    std::fs::read(&out).expect("the screenshot")
}

#[test]
#[ignore = "needs a Pure disc image in data/images/"]
fn the_circuit_panels_label_steps_eight_pixels_a_glyph_as_the_originals_does() {
    for name in ["pure-psp-eu.chd", "pure-psp-usa.chd"] {
        let Some(image) = oag_testdata::image(name) else {
            continue;
        };
        let scratch = std::env::temp_dir().join(format!("oag-pure-pens-{name}"));
        let starts = glyph_starts(&still(&image, &scratch));
        assert_eq!(
            starts.len(),
            10,
            "{name}: the label's ten glyphs, got {starts:?}"
        );
        for (got, want) in starts.iter().zip(RACE_RECORD_ORIGINAL) {
            assert!(
                got.abs_diff(want) <= 1,
                "{name}: glyph at {got}, the original's at {want}: {starts:?}"
            );
        }
        assert!(
            (starts[9] - starts[0]).abs_diff(79) <= 1,
            "{name}: the label spans {} px, the original's 79: {starts:?}",
            starts[9] - starts[0]
        );
    }
}
