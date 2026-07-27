//! Validates the `.fnt` decoder against every font on the PSP disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The test skips with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.
//!
//! # What this is for
//!
//! The atlas layout resisted several passes of blind structural search, and the
//! thing that settles it is not a block geometry - it is where the pixels start.
//! Three independent checks say the same thing, and each of them fails on the
//! reading this project used before:
//!
//! - The atlas block closes **exactly** at `64 + clut_size + width * height / 2`
//!   with no padding, which pins the header at 64 bytes.
//! - The palette then reads as a **full 16-level alpha ramp**. Read 48 bytes
//!   early it has 12 fully transparent entries, which no font can be drawn with.
//! - Every glyph's `u0`/`u1` is a **tight** horizontal bound, so the leftmost and
//!   rightmost column of each box has to contain ink. It does, on essentially
//!   every glyph; on the old reading it does on well under half.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_disc::DiscImage;
use oag_formats::fnt::{self, Font};
use oag_formats::wad::{self, Compression, Directory};

/// The PSP archive that holds the fonts.
const FE_WAD: &str = "PSP_GAME/USRDIR/FE.wad";

/// Fonts the disc is known to carry.
const EXPECTED_FONTS: usize = 5;

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// Fraction of glyphs whose leftmost and rightmost box column both hold ink.
///
/// `u1 - u0` is the glyph's declared width, so the box is tight horizontally and
/// both edge columns must be inked. Vertically it is not: `v0`/`v1` are shared
/// by every glyph on an atlas row, so a lowercase letter legitimately leaves the
/// top rows empty and this deliberately does not test them.
fn tight_horizontal_bounds(font: &Font) -> (usize, f64) {
    let mut checked = 0;
    let mut touching = 0;
    for glyph in &font.glyphs {
        if glyph.width < 3 || glyph.height < 3 {
            continue;
        }
        checked += 1;
        let inked = |x: u16| {
            (glyph.v0..glyph.v1).any(|y| font.alpha_at(usize::from(x), usize::from(y)) != 0)
        };
        touching += usize::from(inked(glyph.u0));
        touching += usize::from(inked(glyph.u1 - 1));
    }
    #[expect(clippy::cast_precision_loss, reason = "a few hundred glyphs")]
    let rate = if checked == 0 {
        0.0
    } else {
        touching as f64 / (2 * checked) as f64
    };
    (checked, rate)
}

#[test]
#[ignore = "needs a PSP disc image under data/images"]
fn every_font_decodes_and_its_glyph_boxes_bound_real_ink() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == FE_WAD)
        .expect("FE.wad present")
        .clone();

    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, Directory::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    let mut fonts = 0;
    let mut glyphs = 0;
    let mut worst_bounds = 1.0f64;
    let mut alpha_levels: BTreeMap<usize, usize> = BTreeMap::new();

    for (index, entry) in dir.entries.iter().enumerate() {
        if entry.size == 0 {
            continue;
        }
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let blob = match entry.compression {
            Compression::None => raw,
            Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            Compression::Zlib => panic!("entry {index}: unexpected zlib entry"),
        };
        if !fnt::looks_like_font(&blob) {
            continue;
        }

        let font = Font::parse(&blob).unwrap_or_else(|e| panic!("entry {index}: {e}"));
        fonts += 1;
        glyphs += font.glyphs.len();

        assert_eq!(
            font.indices.len(),
            usize::from(font.width) * usize::from(font.height),
            "entry {index}: atlas pixel count"
        );
        assert_eq!(font.palette.len(), 16, "entry {index}: palette length");

        // A quantised antialiasing ramp uses its whole palette. Read 48 bytes
        // early the palette has four or five distinct alphas and a dozen dead
        // entries, so this is the check that the palette is where it belongs.
        let levels = font
            .palette
            .iter()
            .map(|c| c[3])
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        *alpha_levels.entry(levels).or_default() += 1;
        assert!(
            levels >= 14,
            "entry {index}: only {levels} distinct palette alphas, expected a full ramp"
        );

        let (checked, rate) = tight_horizontal_bounds(&font);
        println!(
            "  entry {index:2} {}x{} lh={:2} glyphs={:3} checked={checked:3} edge-ink={rate:.3}",
            font.width,
            font.height,
            font.line_height,
            font.glyphs.len()
        );
        worst_bounds = worst_bounds.min(rate);
    }

    println!("fonts        {fonts}");
    println!("glyphs       {glyphs}");
    println!("alpha levels {alpha_levels:?}");
    println!("worst edge-ink rate {worst_bounds:.3}");

    assert_eq!(fonts, EXPECTED_FONTS, "wrong number of fonts found");
    assert!(glyphs > 800, "only {glyphs} glyphs decoded");
    // The old reading scored 0.44 here; a correct one is near 1. The bound is
    // set below the observed 0.967 to leave room for a glyph whose leftmost
    // column is genuinely a fully-transparent antialiasing step.
    assert!(
        worst_bounds > 0.90,
        "worst edge-ink rate {worst_bounds:.3}: glyph boxes do not bound the ink"
    );
}
