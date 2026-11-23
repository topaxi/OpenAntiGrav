//! `oag_ui::font::ink_width` is the original's `Font_MeasureString` on HD's own faces.
//!
//! The widths are what the footer legend's flow law needs
//! (`docs/ghidra/functions/ps3-hdfury-eu/navigation-legend.md`, "What measures a word"):
//! `helv.fnt` at the legend's scale 0.8, unsnapped and with each glyph's pen rounded up. They
//! were emulated from the font's records in the RE lane, and this reads them back through the
//! atlas the front end draws with.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-ui --run-ignored all \
//!     -E 'test(/^ink_width_ground_truth::/)'
//! ```

use oag_ui::font::{Atlas, ink_width};

fn helv() -> Option<Atlas> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let mut archives = oag_assets::Archives::open(&image.display().to_string(), oag_hd::TITLE)
        .expect("open the disc");
    let font = archives
        .read_font(r"Data\FE\Fonts\helv.fnt")
        .expect("helv reads");
    Some(Atlas::from_font(&font))
}

/// (word, unsnapped, snapped) at scale 0.8, in the authored 1920x1080 units.
const WORDS: &[(&str, f32, f32)] = &[
    ("NAVIGATION", 156.0, 159.8),
    ("CONFIRM", 120.0, 123.4),
    ("BACK", 72.8, 73.6),
];

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_legend_words_measure_as_the_original_measures_them() {
    let Some(atlas) = helv() else {
        return;
    };
    for &(word, plain, snapped) in WORDS {
        let got = ink_width(&atlas, word, 0.8, false);
        assert!(
            (got - plain).abs() < 0.05,
            "{word} unsnapped {got} vs {plain}"
        );
        let got = ink_width(&atlas, word, 0.8, true);
        assert!(
            (got - snapped).abs() < 0.05,
            "{word} snapped {got} vs {snapped}"
        );
    }
}
