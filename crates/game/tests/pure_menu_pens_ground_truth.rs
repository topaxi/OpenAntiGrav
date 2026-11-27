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
//!
//! The same frame's description line (46 glyphs and the cursor, typed at the 5.873 s the grab
//! was taken) ends its last glyph at x 425 on one line; at the scale the line drew with before
//! (0.7) it ended 20 % short, and measured against the wrong face it wrapped its last word onto a
//! second line.
//!
//! The title screen's `HOLD ON!` (`font="Stats"`, scale 0.7, right-aligned at 416) is that face too,
//! its last glyph's ink ending on 416: the original's seven glyphs start at 366, 373, 380, 388,
//! 400, 407 and 415 (the menu face drew them from 362; a pen-aligned right edge from 362 as well).

use std::path::Path;

/// The ink-left column of each glyph of the label in the original's frame (the pair's
/// `theirs.png`, read as `docs/ui/menus-original.md` says).
/// The ink-left column of the description line's last glyph, the cursor, in the same frame.
const DESCRIPTION_LAST_ORIGINAL: u32 = 425;

const RACE_RECORD_ORIGINAL: [u32; 10] = [250, 258, 266, 274, 289, 297, 305, 313, 321, 329];

/// The strongest ink of any row of `rows` in column `x`, on a 0 to 255 scale (white is 0).
fn ink_at(image: &oag_texture::replacer::Image, rows: std::ops::Range<u32>, x: u32) -> i32 {
    rows.map(|y| {
        let at = ((y * image.width + x) * 4) as usize;
        let rgb = &image.rgba[at..at + 3];
        (255 * 3 - rgb.iter().map(|&c| i32::from(c)).sum::<i32>()) / 3
    })
    .max()
    .unwrap_or(0)
}

/// The first column of each run of inked columns in `rows`, between `columns`.
fn glyph_starts(
    image: &oag_texture::replacer::Image,
    rows: std::ops::Range<u32>,
    columns: std::ops::Range<u32>,
) -> Vec<u32> {
    let inked = |x: u32| {
        rows.clone().any(|y| {
            let at = ((y * image.width + x) * 4) as usize;
            let rgb = &image.rgba[at..at + 3];
            (255 * 3 - rgb.iter().map(|&c| i32::from(c)).sum::<i32>()) / 3 > 60
        })
    };
    columns.filter(|&x| inked(x) && !inked(x - 1)).collect()
}

fn decode(png: &[u8]) -> oag_texture::replacer::Image {
    let image = oag_texture::replacer::decode_png(png).expect("the screenshot decodes");
    assert_eq!((image.width, image.height), (480, 272));
    image
}

/// The first column of each run of columns that hold a cyan-ish ink (red drawn down) in `rows`,
/// between `columns`.
fn cyan_starts(
    image: &oag_texture::replacer::Image,
    rows: std::ops::Range<u32>,
    columns: std::ops::Range<u32>,
) -> Vec<u32> {
    let inked = |x: u32| {
        rows.clone()
            .map(|y| 255 - i32::from(image.rgba[((y * image.width + x) * 4) as usize]))
            .sum::<i32>()
            > 60
    };
    columns.filter(|&x| inked(x) && !inked(x - 1)).collect()
}

fn title_still(image: &Path, scratch: &Path) -> Vec<u8> {
    std::fs::create_dir_all(scratch).expect("a scratch directory");
    let out = scratch.join("title-screen.png");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args([
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--size",
            "480x272",
            "--screen",
            "Intro Screen->Title Screen",
            "--ticks",
            "200",
            "--screenshot",
        ])
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("OAG_DLC", scratch.join("no-dlc"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game --screen on {}", image.display());
    std::fs::read(&out).expect("the screenshot")
}

/// The ink-left column of each glyph of the title screen's `HOLD ON!` in the original's frame.
const HOLD_ON_ORIGINAL: [u32; 7] = [366, 373, 380, 388, 400, 407, 415];

#[test]
#[ignore = "needs a Pure disc image in data/images/"]
fn the_title_screens_hold_on_is_the_stats_face_ending_on_its_anchor() {
    let images: Vec<(&str, std::path::PathBuf)> = ["pure-psp-eu.chd", "pure-psp-usa.chd"]
        .into_iter()
        .filter_map(|name| oag_testdata::image(name).map(|path| (name, path)))
        .collect();
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none() || !images.is_empty(),
        "OAG_REQUIRE_GAME_DATA is set but neither Pure PSP image is present"
    );
    for (name, image) in images {
        let scratch = oag_testdata::scratch::dir(&format!("pure-title-{name}"));
        let frame = decode(&title_still(&image, &scratch));
        let starts = cyan_starts(&frame, 242..248, 350..418);
        assert_eq!(
            starts.len(),
            HOLD_ON_ORIGINAL.len(),
            "{name}: HOLD ON!'s seven glyphs, got {starts:?}"
        );
        for (got, want) in starts.iter().zip(HOLD_ON_ORIGINAL) {
            assert!(
                got.abs_diff(want) <= 1,
                "{name}: glyph at {got}, the original's at {want}: {starts:?}"
            );
        }
    }
}

fn still(image: &Path, scratch: &Path) -> Vec<u8> {
    std::fs::create_dir_all(scratch).expect("a scratch directory");
    let out = scratch.join("track-selection.png");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args([
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
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
    let images: Vec<(&str, std::path::PathBuf)> = ["pure-psp-eu.chd", "pure-psp-usa.chd"]
        .into_iter()
        .filter_map(|name| oag_testdata::image(name).map(|path| (name, path)))
        .collect();
    // A missing `data/images/` skips honestly, `OAG_REQUIRE_GAME_DATA=1` makes it a failure: the
    // loop below asserts nothing for an image that is not there.
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none() || !images.is_empty(),
        "OAG_REQUIRE_GAME_DATA is set but neither Pure PSP image is present"
    );
    for (name, image) in images {
        let scratch = oag_testdata::scratch::dir(&format!("pure-pens-{name}"));
        let frame = decode(&still(&image, &scratch));
        let starts = glyph_starts(&frame, 171..180, 244..340);
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
        // The glyph is a whole-pixel quad from the pen (`oag_ui::font::snapped_quad`): the
        // first `R` of `RACE RECORD` (pen 249, a 5 px quad) takes a faint left column at 249 and
        // stops at 253; the original's frame has ink 44 % strong at 249 and none at 255, where
        // a fractional quad from the ink edge (250, 5.6 wide) leaves 249 blank and reaches 255.
        assert!(
            ink_at(&frame, 171..180, 249) > 40,
            "{name}: no ink in column 249, where the whole-pixel quad starts"
        );
        assert!(
            ink_at(&frame, 171..180, 255) < 10,
            "{name}: ink in column 255, past the whole-pixel quad"
        );
        // The description line: one line, its last glyph (the cursor) where the original's is.
        let line = glyph_starts(&frame, 219..231, 20..480);
        let last = *line.last().expect("the description line is drawn");
        assert!(
            last.abs_diff(DESCRIPTION_LAST_ORIGINAL) <= 3,
            "{name}: the description ends at {last}, the original's at {DESCRIPTION_LAST_ORIGINAL}"
        );
        assert!(
            glyph_starts(&frame, 231..245, 20..300).is_empty(),
            "{name}: the description wrapped a word onto a second line"
        );
    }
}
