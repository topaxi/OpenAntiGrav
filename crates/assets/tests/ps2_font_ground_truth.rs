//! Validates the PS2 font path against the real disc: metrics-only `.fnt`,
//! glyph atlas in the next archive entry, `PSMT4` swizzle.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! Three claims, each pinned by a check that fails on the reading this project
//! used before:
//!
//! - **A PS2 `.fnt` carries no atlas.** Its `atlas_at` is the file's own
//!   length, so [`fnt::Font::parse`] fails on every one of them and the front
//!   end fell back to its built-in 5x7 glyphs. The metrics parse perfectly,
//!   which is what made this look like a missing file rather than a different
//!   layout.
//! - **The atlas is the entry immediately after the font.** Checked on all five
//!   rather than one: each following entry is a 4bpp texture whose dimensions
//!   cover every glyph box that font declares. A wrong pairing overruns.
//! - **The `PSMT4` unswizzle is right.** `pulse_text` is a single pure white
//!   with an alpha ramp, so every lit texel of its atlas has to fall inside a
//!   glyph box its own metrics declare. It does, exactly; the reading without
//!   the block transposition is not even a bijection and never gets this far.

use std::path::PathBuf;

use oag_pulse as pulse;
use oag_texture::{fnt, ps2_texture};

/// The five fonts, by the names hashed into both discs' archives.
const FONTS: [&str; 5] = [
    pulse::names::fonts::TEXT,
    pulse::names::fonts::PULSE_14,
    pulse::names::fonts::PULSE_20,
    pulse::names::fonts::HUD,
    pulse::names::fonts::SMALL,
];

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

#[test]
#[ignore = "needs the PS2 disc image under data/images"]
fn every_ps2_font_reads_its_atlas_out_of_the_following_entry() {
    let Some(path) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut archives = pulse::open(&path.to_string_lossy()).expect("open");

    for name in FONTS {
        let blob = archives
            .read_name(name)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let metrics = fnt::Metrics::parse(&blob).unwrap_or_else(|e| panic!("{name}: {e}"));

        assert_eq!(
            metrics.atlas_at,
            blob.len(),
            "{name}: a PS2 .fnt should stop exactly where the atlas would start"
        );
        assert!(
            !metrics.has_embedded_atlas(&blob),
            "{name}: no atlas in the file"
        );
        assert!(
            fnt::Font::parse(&blob).is_err(),
            "{name}: the embedded-atlas path must refuse it rather than invent pixels"
        );

        // The pairing rule, checked the way that can fail: the following entry
        // is a texture, and it is big enough for this font's own glyph boxes.
        let atlas = archives
            .read_following(name)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let head = ps2_texture::header(&atlas).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(head.bits_per_pixel, 4, "{name}: font atlases are 4bpp");
        assert_eq!(
            head.layout,
            ps2_texture::Layout::Psmt4,
            "{name}: 4bpp means the PSMT4 transfer shape"
        );

        let font = archives
            .read_font(name)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(font.width, head.width, "{name}: atlas width");
        assert_eq!(font.height, head.height, "{name}: atlas height");
        assert_eq!(
            font.indices.len(),
            usize::from(font.width) * usize::from(font.height),
            "{name}: one index per pixel"
        );
        assert_eq!(font.palette.len(), 16, "{name}: 4bpp CLUT");
        // A stored GS alpha cannot exceed 128, so anything above that says the
        // doubling ran. Without it the text draws at half opacity - visible on
        // screen, invisible to a decode test that only checks shapes. Four of
        // the five peak at exactly 128 and `small.fnt` at 115, so the check is
        // "above the GS ceiling" rather than "reaches 255".
        let peak = font.palette.iter().map(|c| c[3]).max().unwrap_or(0);
        assert!(
            peak > 128,
            "{name}: peak alpha {peak} is still on the GS's 0-128 scale"
        );
        assert!(!font.glyphs.is_empty(), "{name}: glyphs");

        let greys = font
            .palette
            .iter()
            .filter(|c| c[3] != 0)
            .map(|c| c[0])
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        println!(
            "{name:34} {}x{} lh={} glyphs={} greys={} peak-alpha={peak}",
            font.width,
            font.height,
            font.line_height,
            font.glyphs.len(),
            greys,
        );

        for glyph in &font.glyphs {
            assert!(
                glyph.u1 <= font.width && glyph.v1 <= font.height,
                "{name}: glyph {:#06x} box {}..{} x {}..{} overruns a {}x{} atlas",
                glyph.codepoint,
                glyph.u0,
                glyph.u1,
                glyph.v0,
                glyph.v1,
                font.width,
                font.height
            );
        }
    }
}

#[test]
#[ignore = "needs the PS2 disc image under data/images"]
fn the_psmt4_unswizzle_puts_every_lit_texel_inside_a_glyph_box() {
    let Some(path) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut archives = pulse::open(&path.to_string_lossy()).expect("open");

    // `pulse_text` and no other font, deliberately. The other four bake an
    // outline into the atlas that sits *outside* the declared box, and three of
    // them carry glyphs - button icons - that the codepoint table does not list
    // at all, so neither can reach 100 % however right the decode is. This one
    // is a single pure white with an alpha ramp and nothing unlisted, so it can,
    // and it does. See `docs/formats/fnt.md`.
    let name = pulse::names::fonts::TEXT;
    let font = archives.read_font(name).expect("font");
    let width = usize::from(font.width);

    let mut inside_a_box = vec![false; font.indices.len()];
    for glyph in &font.glyphs {
        for y in glyph.v0..glyph.v1 {
            for x in glyph.u0..glyph.u1 {
                inside_a_box[usize::from(y) * width + usize::from(x)] = true;
            }
        }
    }

    let (mut lit, mut stray) = (0usize, 0usize);
    for (at, _) in font
        .indices
        .iter()
        .enumerate()
        .filter(|(at, _)| font.alpha_at(at % width, at / width) != 0)
    {
        lit += 1;
        if !inside_a_box[at] {
            stray += 1;
        }
    }

    assert!(
        lit > 5_000,
        "{name}: only {lit} lit texels, expected a font"
    );
    assert_eq!(
        stray, 0,
        "{name}: {stray} of {lit} lit texels fall outside every declared glyph box"
    );
}
