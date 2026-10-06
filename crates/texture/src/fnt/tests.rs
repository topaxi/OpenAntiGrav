//! What the `.fnt` reader in [`super`] is asserted to do, on fixtures built by
//! hand.
//!
//! Its own file under the 200-line rule in `scripts/check-file-size.py`.
//! Everything here is synthetic; the disc-backed half is
//! `crates/texture/tests/fnt_ground_truth.rs` and `fnt_hd_ground_truth.rs`.

use super::*;
use oag_formats::ByteOrder;

/// Builds a font by hand. No game data in any test.
fn font(width: u16, height: u16, glyphs: &[(u16, u8, u8, u16, u16, u16, u16)]) -> Vec<u8> {
    let count = glyphs.len();
    let codepoints_at = HEADER_LEN;
    let offsets_at = codepoints_at + count * 2;
    let records_at = offsets_at + count * 4;
    let atlas_at = records_at + count * GLYPH_LEN;

    let mut out = vec![0u8; HEADER_LEN];
    out[0] = VERSION;
    out[1..4].copy_from_slice(MAGIC);
    out[4..8].copy_from_slice(&(count as u32).to_le_bytes());
    out[8..12].copy_from_slice(&(codepoints_at as u32).to_le_bytes());
    out[12..16].copy_from_slice(&(offsets_at as u32).to_le_bytes());
    out[16..20].copy_from_slice(&13u32.to_le_bytes());
    out[24..28].copy_from_slice(&(atlas_at as u32).to_le_bytes());

    for g in glyphs {
        out.extend_from_slice(&g.0.to_le_bytes());
    }
    for i in 0..count {
        out.extend_from_slice(&((records_at + i * GLYPH_LEN) as u32).to_le_bytes());
    }
    for g in glyphs {
        out.extend_from_slice(&g.0.to_le_bytes());
        out.push(g.1);
        out.push(g.2);
        out.extend_from_slice(&g.3.to_le_bytes());
        out.extend_from_slice(&g.4.to_le_bytes());
        out.extend_from_slice(&g.5.to_le_bytes());
        out.extend_from_slice(&g.6.to_le_bytes());
        out.push(8);
        out.extend_from_slice(&[0xff; 5]);
    }

    let texel_size = usize::from(width) * usize::from(height) / 2;
    let mut atlas = vec![0u8; ATLAS_HEADER_LEN];
    atlas[0..2].copy_from_slice(&width.to_le_bytes());
    atlas[2..4].copy_from_slice(&height.to_le_bytes());
    atlas[4] = 4;
    atlas[5] = 1;
    atlas[6] = FLAG_SWIZZLED;
    atlas[8..12].copy_from_slice(&64u32.to_le_bytes());
    atlas[12..16].copy_from_slice(&(texel_size as u32).to_le_bytes());
    for i in 0..16u8 {
        atlas.extend_from_slice(&[255, 255, 255, i * 17]);
    }
    // Swizzled texels whose linear form is a horizontal ramp per row.
    let row_bytes = usize::from(width) / 2;
    let mut linear = vec![0u8; texel_size];
    for y in 0..usize::from(height) {
        for x in 0..row_bytes {
            linear[y * row_bytes + x] = ((x + y) % 251) as u8;
        }
    }
    let mut swizzled = vec![0u8; texel_size];
    let mut write = 0;
    for br in 0..usize::from(height) / SWIZZLE_BLOCK_ROWS {
        for bc in 0..row_bytes / SWIZZLE_BLOCK_BYTES {
            for r in 0..SWIZZLE_BLOCK_ROWS {
                let at = (br * SWIZZLE_BLOCK_ROWS + r) * row_bytes + bc * SWIZZLE_BLOCK_BYTES;
                swizzled[write..write + SWIZZLE_BLOCK_BYTES]
                    .copy_from_slice(&linear[at..at + SWIZZLE_BLOCK_BYTES]);
                write += SWIZZLE_BLOCK_BYTES;
            }
        }
    }
    atlas.extend_from_slice(&swizzled);
    out.extend_from_slice(&atlas);
    out
}

#[test]
fn a_font_parses_and_its_atlas_closes() {
    let data = font(64, 16, &[(65, 4, 8, 0, 4, 0, 8), (66, 5, 8, 6, 11, 0, 8)]);
    let parsed = Font::parse(&data).expect("parse");
    assert_eq!(parsed.line_height, 13);
    assert_eq!(parsed.width, 64);
    assert_eq!(parsed.height, 16);
    assert_eq!(parsed.palette.len(), 16);
    assert_eq!(parsed.indices.len(), 64 * 16);
    assert_eq!(parsed.glyphs.len(), 2);
    assert_eq!(parsed.glyphs[1].codepoint, 66);
    assert_eq!(parsed.glyphs[1].advance, 8);
}

#[test]
fn the_swizzle_is_undone() {
    let data = font(64, 16, &[(65, 4, 8, 0, 4, 0, 8)]);
    let parsed = Font::parse(&data).expect("parse");
    // The fixture's linear image is `(x + y) % 251` per byte, so pixel
    // 2x of row y is the low nibble of that.
    for y in 0..16usize {
        for x in 0..32usize {
            let byte = ((x + y) % 251) as u8;
            assert_eq!(parsed.indices[y * 64 + x * 2], byte & 0x0f, "({x},{y}) low");
            assert_eq!(
                parsed.indices[y * 64 + x * 2 + 1],
                byte >> 4,
                "({x},{y}) high"
            );
        }
    }
}

#[test]
fn an_unswizzled_atlas_is_taken_verbatim() {
    let mut data = font(64, 16, &[(65, 4, 8, 0, 4, 0, 8)]);
    let atlas_at = data.len() - (ATLAS_HEADER_LEN + 64 + 64 * 16 / 2);
    data[atlas_at + 6] = 0;
    let parsed = Font::parse(&data).expect("parse");
    // The stored bytes are swizzled: verbatim, stored byte 16 (the block's second
    // row, value 1) would land at pixel 32 of row 0; unswizzled it is pixel 0 of row 1.
    assert_eq!(parsed.indices[32], 1);
    let swizzled = Font::parse(&font(64, 16, &[(65, 4, 8, 0, 4, 0, 8)])).expect("parse");
    assert_eq!(swizzled.indices[64], 1);
    assert_ne!(swizzled.indices, parsed.indices);
}

#[test]
fn a_box_that_disagrees_with_its_size_is_refused() {
    let data = font(64, 16, &[(65, 9, 8, 0, 4, 0, 8)]);
    assert_eq!(
        Font::parse(&data),
        Err(Error::BoxMismatch { codepoint: 65 })
    );
}

#[test]
fn a_wrong_atlas_size_is_refused() {
    let mut data = font(64, 16, &[(65, 4, 8, 0, 4, 0, 8)]);
    data.push(0);
    assert!(matches!(
        Font::parse(&data),
        Err(Error::AtlasSizeMismatch { .. })
    ));
}

#[test]
fn a_non_font_is_refused() {
    assert_eq!(Font::parse(&[0u8; 64]), Err(Error::NotAFont));
    assert!(matches!(
        Font::parse(&[1u8, 70, 78]),
        Err(Error::TooShort { .. })
    ));
}

/// Rewrites a little-endian `.fnt` as the PS3 exporter would write it.
///
/// **A structural swap, not a `chunks(4).rev()`.** The file is not an array of
/// words: the magic is one, the codepoint table is `u16`s, the offset table `u32`s,
/// a glyph record mixes both with four bare bytes, and the atlas header mixes them
/// over a palette and texels that are byte streams and must not move. Walking it as
/// the parser does yields *the same font*, which
/// [`swapping_a_font_changes_nothing_but_the_byte_order`] relies on.
fn to_big_endian(le: &[u8]) -> Vec<u8> {
    let w = |at: usize| u32::from_le_bytes(le[at..at + 4].try_into().unwrap());
    let mut out = le.to_vec();
    let put32 = |out: &mut Vec<u8>, at: usize| {
        let v = u32::from_le_bytes(le[at..at + 4].try_into().unwrap());
        out[at..at + 4].copy_from_slice(&v.to_be_bytes());
    };
    let put16 = |out: &mut Vec<u8>, at: usize| {
        let v = u16::from_le_bytes(le[at..at + 2].try_into().unwrap());
        out[at..at + 2].copy_from_slice(&v.to_be_bytes());
    };

    // The magic is one 32-bit word, which is why the PS3's reads `TNF\x01`.
    out[0..4].reverse();
    for at in [0x04, 0x08, 0x0c, 0x10, 0x14, 0x18] {
        put32(&mut out, at);
    }

    let count = w(0x04) as usize;
    let codepoints_at = w(0x08) as usize;
    let offsets_at = w(0x0c) as usize;
    let atlas_at = w(0x18) as usize;
    for i in 0..count {
        put16(&mut out, codepoints_at + i * 2);
    }
    for i in 0..count {
        let record = w(offsets_at + i * 4) as usize;
        put32(&mut out, offsets_at + i * 4);
        // The codepoint, two bare bytes, then the four box edges; the advance and
        // trailing padding are bytes and stay put.
        put16(&mut out, record);
        for edge in 0..4 {
            put16(&mut out, record + 4 + edge * 2);
        }
    }

    // Width and height, then the two sizes. The palette and the texels are byte
    // streams: an RGBA entry is four bytes in order and a 4bpp pair is one byte.
    put16(&mut out, atlas_at);
    put16(&mut out, atlas_at + 2);
    put32(&mut out, atlas_at + 8);
    put32(&mut out, atlas_at + 0x0c);
    out
}

/// The magic decides the order, and nothing that is not a font claims either.
#[test]
fn the_magic_says_which_end_the_words_start_at() {
    assert_eq!(byte_order(&[1, b'F', b'N', b'T']), Some(ByteOrder::Little));
    assert_eq!(byte_order(&[b'T', b'N', b'F', 1]), Some(ByteOrder::Big));
    assert_eq!(byte_order(&[b'F', b'N', b'T', 1]), None);
    assert_eq!(byte_order(&[1, b'T', b'N', b'F']), None);
    assert_eq!(byte_order(&[0; 4]), None);
    // Under four bytes cannot be either, and must not index past the end.
    assert_eq!(byte_order(b"TNF"), None);
    assert_eq!(byte_order(&[]), None);
}

/// [`looks_like_font`] widened with [`byte_order`] and `fnt_ground_truth.rs`
/// filters on it; this pins that it accepts both orders.
#[test]
fn a_font_looks_like_one_either_way_round() {
    assert!(looks_like_font(&[1, b'F', b'N', b'T']));
    assert!(looks_like_font(&[b'T', b'N', b'F', 1]));
    assert!(!looks_like_font(b"WOtd"));
}

/// The whole claim about Wipeout HD's fonts, in one assertion: the same file
/// written the other way round is the same font.
///
/// The line height, glyph boxes, palette and every unswizzled index must come out
/// identical, much stronger than "it parses": a scalar read at the wrong end would
/// move a box, resize the atlas, or fail the size check.
#[test]
fn swapping_a_font_changes_nothing_but_the_byte_order() {
    let little = font(64, 16, &[(65, 4, 8, 0, 4, 0, 8), (66, 8, 8, 4, 12, 0, 8)]);
    let big = to_big_endian(&little);
    assert_ne!(little, big, "the fixture has scalars to swap");

    let from_little = Font::parse(&little).expect("the PSP fixture parses");
    let from_big = Font::parse(&big).expect("the same file, big-endian, parses");
    assert_eq!(from_little, from_big);
}

#[test]
fn coverage_claims_everything_but_the_reserved_header_tail_on_a_full_font() {
    let data = font(64, 16, &[(65, 4, 8, 0, 4, 0, 8), (66, 8, 8, 4, 12, 0, 8)]);
    let seen = coverage(&data);
    let gaps = seen.gaps(1);
    assert_eq!(gaps.len(), 1, "{gaps:?}");
    assert_eq!((gaps[0].at, gaps[0].len), (0x1c, 20));
}

#[test]
fn coverage_on_a_metrics_only_blob_claims_no_atlas() {
    let mut data = font(64, 16, &[(65, 4, 8, 0, 4, 0, 8)]);
    // Trim off the atlas and point `atlas_at` at the new end: the PS2 shape.
    let metrics = Metrics::parse(&data).expect("metrics-only still parses");
    data.truncate(metrics.atlas_at);
    let seen = coverage(&data);
    assert_eq!(
        seen.claimed(),
        data.len() - 20,
        "everything but the reserved header tail"
    );
}
