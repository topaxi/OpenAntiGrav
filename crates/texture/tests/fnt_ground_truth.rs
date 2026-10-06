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
//! The atlas layout is settled not by a block geometry but by where the pixels
//! start. Three independent checks agree, and each fails on the earlier reading:
//!
//! - The atlas block closes **exactly** at `64 + clut_size + width * height / 2`
//!   with no padding, which pins the header at 64 bytes.
//! - The palette then reads as a **full 16-level alpha ramp**. Read 48 bytes
//!   early it has 12 fully transparent entries, which no font can be drawn with.
//! - Every glyph's `u0`/`u1` is a **tight** horizontal bound, so the leftmost and
//!   rightmost column of each box has to contain ink. It does, on essentially
//!   every glyph; on the old reading it does on well under half.

use std::collections::BTreeMap;
use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::swizzle;
use oag_formats::wad::{self, Compression, Directory};
use oag_texture::fnt::{self, Font};
use oag_texture::texture::Texture;

/// The PSP archive that holds the fonts.
const FE_WAD: &str = "PSP_GAME/USRDIR/FE.wad";

/// Fonts the disc is known to carry.
const EXPECTED_FONTS: usize = 5;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// Inked pixels over total, across the ten digit glyphs.
///
/// Separates a bare stroke from a pre-outlined one: a normal typeface inks about
/// a quarter of a digit's box, a baked outline nearly two thirds.
fn digit_ink(font: &Font) -> (usize, usize) {
    let (mut inked, mut total) = (0usize, 0usize);
    for ch in "0123456789".chars() {
        let Some(g) = font
            .glyphs
            .iter()
            .find(|g| char::from_u32(u32::from(g.codepoint)) == Some(ch))
        else {
            continue;
        };
        for y in g.v0..g.v1 {
            for x in g.u0..g.u1 {
                total += 1;
                inked += usize::from(font.alpha_at(usize::from(x), usize::from(y)) > 128);
            }
        }
    }
    (inked, total)
}

/// Fraction of glyphs whose leftmost and rightmost box column both hold ink.
///
/// `u1 - u0` is the declared width, so both edge columns must be inked.
/// Vertically it is not tight (`v0`/`v1` are shared by a whole atlas row), so
/// rows are not tested.
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
    let mut outlined = 0usize;
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

        // A quantised antialiasing ramp uses its whole palette; read 48 bytes early
        // it has four or five distinct alphas and a dozen dead entries.
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

        // Distinct greys and the inked fraction of a digit's box separate the two
        // HUD fonts from the three menu ones - see `outlined` below.
        let greys = font
            .palette
            .iter()
            .filter(|c| c[3] != 0)
            .map(|c| c[0])
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let (ink, ink_total) = digit_ink(&font);
        let ink_rate = ink as f64 / ink_total.max(1) as f64;
        if font.is_outlined() {
            outlined += 1;
            // A pre-outlined glyph covers far more of its box than a bare stroke:
            // measured 62-65 % against 25-26 %.
            assert!(
                ink_rate > 0.45,
                "entry {index}: outlined but only {ink_rate:.3} of a digit box is inked"
            );
        } else {
            assert_eq!(
                greys, 1,
                "entry {index}: not outlined, so the palette should hold one grey"
            );
            assert!(
                ink_rate < 0.45,
                "entry {index}: not outlined but {ink_rate:.3} of a digit box is inked"
            );
        }

        let (checked, rate) = tight_horizontal_bounds(&font);
        println!(
            "  entry {index:2} {}x{} lh={:2} glyphs={:3} checked={checked:3} \
             edge-ink={rate:.3} greys={greys} digit-ink={ink_rate:.3} outlined={}",
            font.width,
            font.height,
            font.line_height,
            font.glyphs.len(),
            font.is_outlined()
        );
        worst_bounds = worst_bounds.min(rate);
    }

    println!("fonts        {fonts}");
    println!("glyphs       {glyphs}");
    println!("alpha levels {alpha_levels:?}");
    println!("outlined     {outlined} of {fonts}");
    println!("worst edge-ink rate {worst_bounds:.3}");

    assert_eq!(fonts, EXPECTED_FONTS, "wrong number of fonts found");
    assert!(glyphs > 800, "only {glyphs} glyphs decoded");
    // `PulseHud.fnt` and `small.fnt` (the `HUD` and `HUDSmall` roles) and nothing
    // else. A renderer must act on it: their palette RGB is a body/outline mask, so
    // alpha alone turns a digit into a solid box (see `oag_game::font` and
    // `docs/formats/fnt.md`).
    assert_eq!(
        outlined, 2,
        "expected exactly two outlined fonts, found {outlined}"
    );
    // The old reading scored 0.44; a correct one is near 1. The bound sits below
    // the observed 0.967 for a glyph whose leftmost column is a transparent
    // antialiasing step.
    assert!(
        worst_bounds > 0.90,
        "worst edge-ink rate {worst_bounds:.3}: glyph boxes do not bound the ink"
    );
}

/// Mean pixel step across a swizzle-block seam, over the mean step elsewhere.
///
/// The GE's blocks are 16 bytes wide. A correct decode has nothing special at
/// those boundaries (ratio near 1); a wrong swizzle splices unrelated pixels there
/// and the ratio climbs. This says which reading the `+0x07` flag selects, for
/// both answers.
fn seam_ratio(indices: &[u8], width: usize, height: usize, block_pixels: usize) -> f64 {
    let (mut seam, mut seam_n, mut other, mut other_n) = (0u64, 0u64, 0u64, 0u64);
    for y in 0..height {
        for x in 0..width - 1 {
            let at = y * width + x;
            let step = u64::from(i32::from(indices[at]).abs_diff(i32::from(indices[at + 1])));
            if (x + 1) % block_pixels == 0 {
                seam += step;
                seam_n += 1;
            } else {
                other += step;
                other_n += 1;
            }
        }
    }
    if seam_n == 0 || other_n == 0 {
        return 1.0;
    }
    #[expect(clippy::cast_precision_loss, reason = "sums over a small texture")]
    let ratio = (seam as f64 / seam_n as f64) / (other as f64 / other_n as f64).max(1e-9);
    ratio
}

#[test]
#[ignore = "needs a PSP disc image under data/images"]
fn mip_textures_honour_the_swizzle_flag() {
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

    let mut textures = 0;
    let mut flagged = 0;
    let mut discriminating = 0;
    let mut flag_wins = 0;

    for (index, entry) in dir.entries.iter().enumerate() {
        if entry.size == 0 {
            continue;
        }
        let blob = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let Ok(parsed) = Texture::parse(&blob) else {
            continue;
        };
        textures += 1;
        let (w, h) = (usize::from(parsed.width), usize::from(parsed.height));
        let bpp = usize::from(parsed.bits_per_pixel);
        let row_bytes = w * bpp / 8;
        // `unknown[2]` is `+0x07`, whose bit 0 is the swizzle flag.
        let is_swizzled = parsed.unknown[2] & swizzle::FLAG_SWIZZLED != 0;
        flagged += usize::from(is_swizzled);

        // One block column makes the swizzle the identity, so a texture that narrow
        // cannot tell the readings apart.
        if row_bytes <= swizzle::SWIZZLE_BLOCK_BYTES {
            continue;
        }
        discriminating += 1;

        let stored = &blob[16 + (1usize << bpp) * 4..];
        let expand = |bytes: &[u8]| -> Vec<u8> {
            if bpp == 8 {
                bytes.to_vec()
            } else {
                bytes.iter().flat_map(|&b| [b & 0x0f, b >> 4]).collect()
            }
        };
        let linear = expand(stored);
        let unswizzled = expand(&swizzle::unswizzle(stored, row_bytes, h));
        let block_pixels = swizzle::SWIZZLE_BLOCK_BYTES * 8 / bpp;

        let as_linear = seam_ratio(&linear, w, h, block_pixels);
        let as_unswizzled = seam_ratio(&unswizzled, w, h, block_pixels);
        let (chosen, rejected) = if is_swizzled {
            (as_unswizzled, as_linear)
        } else {
            (as_linear, as_unswizzled)
        };
        flag_wins += usize::from(chosen < rejected);
        println!(
            "  entry {index:2} {w:3}x{h:<3} bpp={bpp} flag={} seam linear={as_linear:5.2} \
             unswizzled={as_unswizzled:5.2} {}",
            u8::from(is_swizzled),
            if chosen < rejected { "ok" } else { "MISS" }
        );
    }

    println!("textures {textures}, flagged {flagged}, discriminating {discriminating}");
    println!("flag picks the smoother reading on {flag_wins} of {discriminating}");

    assert!(textures >= 13, "only {textures} textures found");
    assert!(
        flagged > 0,
        "no texture sets the flag, so this proves nothing"
    );
    assert!(
        discriminating >= 10,
        "only {discriminating} wide enough to check"
    );
    // Both answers are exercised (some blobs linear, some swizzled) and the flag
    // picks the seam-free reading. The one allowed miss is a 32x16 blob, four
    // blocks, with almost nothing to average over.
    assert!(
        flag_wins + 1 >= discriminating,
        "the +0x07 flag picked the seamier reading on {} of {discriminating}",
        discriminating - flag_wins
    );
}
