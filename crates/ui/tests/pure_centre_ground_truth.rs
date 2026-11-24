//! Pure's centred `<Menu>` rows start where the original's text layout puts them.
//!
//! Seven rows measured in PPSSPP (software renderer, USA disc, two boots) on the Free Play,
//! Tournament and Time Trial pause screens, in `FX300ANG.fnt` centred on x 240: the first glyph's
//! quad column of each (`docs/ghidra/functions/psp-pure-usa/menu-text-centring.md`).
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-ui --run-ignored all \
//!     -E 'test(/^pure_centre_ground_truth::/)'
//! ```

use oag_ui::font::{Atlas, snapped_centre_pen};

/// `(text, scale, first quad column)`.
const ROWS: &[(&str, f32, f32)] = &[
    ("Continue", 1.2, 187.0),
    ("Restart", 1.2, 189.0),
    ("Quit", 1.2, 215.0),
    ("Continue", 1.0, 197.0),
    ("Restart", 1.0, 198.0),
    ("End Session", 1.0, 181.0),
    ("Quit", 1.0, 220.0),
];

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn the_seven_measured_rows_start_where_the_original_starts_them() {
    let Some(image) = oag_testdata::image("pure-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&image.display().to_string(), oag_pure::TITLE)
        .expect("open the disc");
    let font = archives
        .read_font(r"Data\FE\Fonts\FX300ANG.fnt")
        .expect("the menu face reads");
    let atlas = Atlas::from_font(&font);
    for &(text, scale, column) in ROWS {
        let pen = snapped_centre_pen(&atlas, text, scale, 240.0);
        assert_eq!(pen.ceil(), column, "{text:?} at {scale}: pen {pen}");
    }
}
