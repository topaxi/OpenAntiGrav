//! Validates the `.fnt` decoder against every font on the Wipeout HD disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The image has to be layer-1 decrypted first - `scripts/ps3iso.py decrypt`,
//! `docs/formats/ps3-disc.md`.
//!
//! # What this is for
//!
//! The sibling `fnt_ground_truth.rs` pins the atlas layout on the PSP. This one
//! pins what no PSP file can test: **which end of a word comes first**, and that
//! nothing else changed with it.
//!
//! The first four bytes are one `u32` constant in the file's own order, `\x01FNT`
//! against `TNF\x01`, so [`fnt::byte_order`] sniffs it. More than a magic check,
//! every *other* field then closes on the same reading, which this file asserts:
//!
//! - The declared tables tile the file with no slack and no overlap, so the
//!   glyph-record offsets, the atlas offset and the file's own length agree.
//! - The atlas header's `width * height / 2` equals its declared `texel_size`
//!   and the block ends exactly on the file's last byte.
//! - Every glyph's box bounds real ink, the same tight-bounds property the PSP
//!   test uses - which is also the check that pins the **nibble** order, since
//!   swapping nibbles moves ink one column out of every box.
//!
//! # And two things that are not byte order
//!
//! `flags` is 0 on every HD atlas (the PSP sets bit 0 on all five of its own):
//! HD's texels are linear and the unswizzle must not run. HD's palette alpha is
//! full-range, so the PS2's 0-128 doubling must not reach it. Each would be a
//! plausible wrong picture, not an error, so both are asserted.

use std::path::PathBuf;

use oag_formats::ByteOrder;
use oag_texture::fnt::{self, Font};

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

/// The archive holding the Latin faces. `DATA04`-`DATA06` carry four Asian ones
/// each, so this file reads **all seven** and the roles test names this one.
const LATIN_ARCHIVE: &str = "DATA02.PSARC";

/// Every `.fnt` on the disc, over all seven PSARCs. `DATA02` holds 17,
/// `DATA03` the four retro-HUD faces, and `DATA04`-`DATA06` four each.
const EXPECTED_FONTS: usize = 33;

/// Fonts in `DATA02` alone, which is where every Latin face lives.
const EXPECTED_LATIN_FONTS: usize = 17;

/// The three faces the front end and the HUD resolve to, by their language
/// plugins' `<Font>` slots: `Default`, `HUD` and `HUDSmall`.
///
/// Line heights are measured: HD's HUD face is 92 px against Pulse's 25, the
/// 720p-versus-480x272 ratio and the cheapest sanity check that the header was
/// read the right way round.
const ROLES: &[(&str, u32, u16, u16)] = &[
    ("/data/fe/fonts/helv.fnt", 33, 1024, 512),
    ("/data/fe/fonts/pulsehud.fnt", 92, 2048, 1024),
    ("/data/fe/fonts/small.fnt", 34, 512, 512),
];

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

fn archive(name: &str) -> Option<oag_assets::psarc::Archive> {
    let image = image()?;
    let spec = format!("{}:PS3_GAME/USRDIR/{name}", image.display());
    Some(oag_assets::psarc::Archive::open(&spec).expect("the archive opens"))
}

/// Every `.fnt` on the disc, as `(archive, path, bytes)`.
///
/// All seven archives, not a sample: a scoped survey of three once concluded the
/// disc had four fonts.
fn every_font() -> Vec<(String, String, Vec<u8>)> {
    let mut out = Vec::new();
    for n in 0..7 {
        let name = format!("DATA0{n}.PSARC");
        let Some(mut archive) = archive(&name) else {
            return Vec::new();
        };
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.ends_with(".fnt"))
            .cloned()
            .collect();
        for path in paths {
            let blob = archive.read_path(&path).expect("the font reads");
            out.push((name.clone(), path, blob));
        }
    }
    out
}

/// Rows of the glyph boxes' outside columns that hold ink, per nibble order.
///
/// The measurement that settles 4bpp nibble order: swapping nibbles moves every
/// pixel one column, so a sheet still reads as a font and only the boxes object.
/// `swap` reads the atlas with the nibbles the other way round.
fn box_spill(font: &Font, swap: bool) -> (usize, usize) {
    let width = usize::from(font.width);
    let alpha = |x: usize, y: usize| {
        let index = y * width + if swap { x ^ 1 } else { x };
        font.palette
            .get(usize::from(font.indices[index]))
            .map_or(0, |c| c[3])
    };
    let (mut spilt, mut boxes) = (0usize, 0usize);
    for glyph in &font.glyphs {
        if glyph.width == 0 || glyph.u0 == 0 || usize::from(glyph.u1) >= width {
            continue;
        }
        boxes += 1;
        for y in usize::from(glyph.v0)..usize::from(glyph.v1) {
            if alpha(usize::from(glyph.u0) - 1, y) != 0 || alpha(usize::from(glyph.u1), y) != 0 {
                spilt += 1;
            }
        }
    }
    (spilt, boxes)
}

/// Fraction of glyphs whose leftmost and rightmost box column both hold ink.
///
/// The PSP test's measure, doubling here as the only check distinguishing the two
/// 4bpp nibble orders (a one-pixel swap a sheet still reads as a font through).
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
#[ignore = "needs a decrypted PS3 disc image under data/images"]
fn every_hd_font_is_big_endian_and_decodes() {
    let fonts = every_font();
    if fonts.is_empty() {
        return;
    }
    assert_eq!(
        fonts.len(),
        EXPECTED_FONTS,
        "wrong number of fonts across the seven archives"
    );

    let mut glyphs = 0;
    let mut worst_bounds = f64::MAX;
    for (archive, path, blob) in &fonts {
        let path = format!("{archive} {path}");
        let path = &path;
        let blob = blob.as_slice();

        // Big-endian, sniffed rather than assumed: a `Little` answer would parse
        // into nonsense instead of failing.
        assert_eq!(
            fnt::byte_order(blob),
            Some(ByteOrder::Big),
            "{path}: not a big-endian .fnt"
        );

        let metrics = fnt::Metrics::parse(blob).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert!(
            metrics.has_embedded_atlas(blob),
            "{path}: no embedded atlas, which is the PS2 shape and not HD's"
        );
        let font = Font::parse(blob).unwrap_or_else(|e| panic!("{path}: {e}"));

        // `Font::parse` refuses a block that is not exactly
        // `64 + clut + width * height / 2`, so reaching here proves the atlas closes
        // on the last byte. That the *metrics* half tiles up to it is this:
        assert_eq!(
            metrics.atlas_at
                + fnt::ATLAS_HEADER_LEN
                + 64
                + usize::from(font.width) * usize::from(font.height) / 2,
            blob.len(),
            "{path}: the declared tables and the atlas do not tile the file"
        );

        // 16 entries, white, an alpha ramp using the full range. The PS2 path
        // doubles alpha for the GS's 0-128; anything above 128 here proves that must
        // not happen on this build.
        assert_eq!(font.palette.len(), 16, "{path}: not a 16-entry clut");
        let peak = font.palette.iter().map(|c| c[3]).max().expect("16 entries");
        assert!(
            peak > 128,
            "{path}: peak alpha {peak} - a 0-128 ramp, so the PS2 doubling would apply"
        );

        let (checked, rate) = tight_horizontal_bounds(&font);
        if checked > 0 {
            worst_bounds = worst_bounds.min(rate);
        }
        glyphs += font.glyphs.len();
        println!(
            "{path:44} {}x{} lh={:3} glyphs={:4} edge-ink {rate:.3}",
            font.width,
            font.height,
            font.line_height,
            font.glyphs.len()
        );
    }

    println!("fonts  {}", fonts.len());
    println!("glyphs {glyphs}");
    println!("worst edge-ink rate {worst_bounds:.3}");

    // 19,936 across the 33, the Asian faces carrying most. A loose floor catching
    // a decode that stops early; the edge-ink rate below is what matters.
    assert!(glyphs > 19_000, "only {glyphs} glyphs decoded");
    // Observed worst is 0.953, on `arialbd.fnt`. The nibble order is pinned by
    // `the_nibble_order_is_low_first_and_the_glyph_boxes_are_what_say_so`; this
    // bound leaves room for a transparent antialiasing edge column.
    assert!(
        worst_bounds > 0.90,
        "worst edge-ink rate {worst_bounds:.3}: glyph boxes do not bound the ink"
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image under data/images"]
fn the_three_wired_roles_have_the_metrics_the_front_end_expects() {
    let Some(mut archive) = archive(LATIN_ARCHIVE) else {
        return;
    };
    assert_eq!(
        archive
            .paths()
            .iter()
            .filter(|p| p.ends_with(".fnt"))
            .count(),
        EXPECTED_LATIN_FONTS,
        "wrong number of fonts in {LATIN_ARCHIVE}"
    );
    for &(path, line_height, width, height) in ROLES {
        let blob = archive.read_path(path).expect("the font reads");
        let font = Font::parse(&blob).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(font.line_height, line_height, "{path}: line height");
        assert_eq!((font.width, font.height), (width, height), "{path}: atlas");
        // Latin, so the roles resolve to something drawable; the Asian faces
        // legitimately carry no ASCII digits.
        for ch in "0123456789".chars() {
            assert!(
                font.glyphs
                    .iter()
                    .any(|g| char::from_u32(u32::from(g.codepoint)) == Some(ch)),
                "{path}: no glyph for {ch}"
            );
        }
    }
}

#[test]
#[ignore = "needs a decrypted PS3 disc image under data/images"]
fn hd_atlases_are_linear_where_the_psp_swizzles() {
    for (archive, path, blob) in every_font() {
        let metrics = fnt::Metrics::parse(&blob).expect("metrics");
        let flags = blob[metrics.atlas_at + 6];
        assert_eq!(
            flags & fnt::FLAG_SWIZZLED,
            0,
            "{archive} {path}: claims swizzled texels, which would need the GE block walk"
        );
    }
}

/// The nibble order, measured rather than carried over from the PSP.
///
/// A big-endian file has no obligation to reverse nibbles, and the check must be
/// numeric because the wrong order still renders as a legible font. Read the
/// right way round, no ink falls in the column left of `u0` or at `u1`.
#[test]
#[ignore = "needs a decrypted PS3 disc image under data/images"]
fn the_nibble_order_is_low_first_and_the_glyph_boxes_are_what_say_so() {
    let fonts = every_font();
    if fonts.is_empty() {
        return;
    }
    let mut worst_wrong = usize::MAX;
    for (archive, path, blob) in &fonts {
        let font = Font::parse(blob).unwrap_or_else(|e| panic!("{archive} {path}: {e}"));
        let (low, boxes) = box_spill(&font, false);
        let (high, _) = box_spill(&font, true);
        println!("{archive} {path:44} {boxes:4} boxes  low {low:6}  high {high:6}");
        assert_eq!(
            low, 0,
            "{archive} {path}: {low} row(s) of ink outside the glyph boxes over {boxes} boxes,              read low-nibble-first"
        );
        if boxes > 0 {
            worst_wrong = worst_wrong.min(high);
        }
    }
    // Zero against thousands makes the reading a measurement. The floor is per
    // font (the least any face spills read the wrong way), so a large atlas cannot
    // carry a small one.
    assert!(
        worst_wrong > 200,
        "read high-nibble-first the least-affected font spills only {worst_wrong} row(s),          so the boxes do not distinguish the two orders on this corpus"
    );
}
