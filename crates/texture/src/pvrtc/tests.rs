use super::*;

/// One 8-byte word: modulation first, then the colour pair, both
/// little-endian.
fn word(colour: u32, modulation: u32) -> [u8; WORD_LEN] {
    let mut out = [0u8; WORD_LEN];
    out[0..4].copy_from_slice(&modulation.to_le_bytes());
    out[4..8].copy_from_slice(&colour.to_le_bytes());
    out
}

/// A whole 8x8 surface of four identical words, so the bilinear step drops out
/// and the tests read the colour unpack and blend alone.
fn uniform(colour: u32, modulation: u32) -> Vec<u8> {
    word(colour, modulation).repeat(4)
}

fn decode(colour: u32, modulation: u32) -> Vec<[u8; 4]> {
    decode_ii_4bpp(&uniform(colour, modulation), 8, 8).expect("decodes")
}

/// Colour A opaque, `RGB 5:5:4` all ones - the widening has to reach 255 on
/// every channel, including blue's replicated low bit.
const WHITE_A_BLACK_B: u32 = 0x8000_7ffe;

/// The same word with the modulation-mode bit set.
const WHITE_A_BLACK_B_MODE1: u32 = 0x8000_7fff;

/// Colour A transparent, `ARGB 3:4:4:3` all ones. Alpha's low bit is never
/// filled in, so full alpha is 14/15, not 15/15.
const TRANSPARENT_WHITE_A: u32 = 0x0000_7ffe;

#[test]
fn a_zero_surface_decodes_to_transparent_black() {
    let out = decode(0, 0);
    assert_eq!(out.len(), 64);
    assert!(out.iter().all(|&t| t == [0, 0, 0, 0]), "{:?}", out[0]);
}

#[test]
fn modulation_zero_takes_colour_a_whole() {
    let out = decode(WHITE_A_BLACK_B, 0);
    assert!(
        out.iter().all(|&t| t == [255, 255, 255, 255]),
        "{:?}",
        out[0]
    );
}

/// Modulation code 3 in mode 0 is weight 8 of 8 - colour B, undiluted.
#[test]
fn modulation_three_takes_colour_b_whole() {
    let out = decode(WHITE_A_BLACK_B, u32::MAX);
    assert!(out.iter().all(|&t| t == [0, 0, 0, 255]), "{:?}", out[0]);
}

/// Codes 0, 1, 2, 3 in mode 0 are weights 0, 3, 5, 8 of eight, so a word cycling
/// all four codes decodes to exactly four greys at those blends of white and black.
///
/// The *set* is asserted, not the order: which code lands on which texel is the
/// mapping step's business, pinned by [`a_non_square_surface_comes_out_row_major`].
#[test]
fn mode_zero_has_four_evenly_spaced_blend_steps() {
    // 0b11_10_01_00 per row of four texels, repeated for all sixteen.
    let out = decode(WHITE_A_BLACK_B, 0xe4e4_e4e4);
    let greys: std::collections::BTreeSet<u8> = out.iter().map(|t| t[0]).collect();
    let expected: std::collections::BTreeSet<u8> = [0u8, 95, 159, 255].into_iter().collect();
    assert_eq!(greys, expected);
    assert!(out.iter().all(|t| t[3] == 255), "alpha stays opaque");
}

/// Code 2 in mode 1 is the punch-through step: weight 4 of 8 on colour, and
/// alpha forced to zero regardless of what either colour carries.
#[test]
fn mode_one_code_two_punches_alpha_through() {
    let out = decode(WHITE_A_BLACK_B_MODE1, 0xaaaa_aaaa);
    assert!(out.iter().all(|&t| t == [127, 127, 127, 0]), "{:?}", out[0]);
}

#[test]
fn transparent_mode_widens_every_channel_but_leaves_alphas_low_bit_clear() {
    let out = decode(TRANSPARENT_WHITE_A, 0);
    assert!(
        out.iter().all(|&t| t == [255, 255, 255, 238]),
        "{:?}",
        out[0]
    );
}

/// A level below [`MIN_SIDE`] decodes at the minimum surface and is cropped back.
#[test]
fn a_level_below_the_minimum_surface_still_decodes_and_is_cropped() {
    let data = uniform(WHITE_A_BLACK_B, 0);
    let out = decode_ii_4bpp(&data, 4, 4).expect("decodes");
    assert_eq!(out.len(), 16);
    assert!(out.iter().all(|&t| t == [255, 255, 255, 255]));
}

/// Short data reads as zero words: the only way a 4x4 level (two words stored,
/// four needed) can decode.
#[test]
fn short_data_reads_as_zero_words() {
    let out = decode_ii_4bpp(&word(WHITE_A_BLACK_B, 0), 8, 8).expect("decodes");
    assert_eq!(out.len(), 64);
    assert!(out.iter().any(|&t| t != [0, 0, 0, 0]), "not all absent");
}

/// The output is row-major and exactly `width * height` on a non-square surface,
/// where a transposed decode would have the right length but the wrong shape.
#[test]
fn a_non_square_surface_comes_out_row_major() {
    let mut data = Vec::new();
    // 32x8 - eight words across, two down.
    for index in 0..16u32 {
        // Every word black except one, so the lit texels locate the word.
        let colour = if index == 0 {
            WHITE_A_BLACK_B
        } else {
            0x8000_0000
        };
        data.extend_from_slice(&word(colour, 0));
    }
    let out = decode_ii_4bpp(&data, 32, 8).expect("decodes");
    assert_eq!(out.len(), 32 * 8);
    let lit = out.iter().filter(|t| t[0] > 0).count();
    assert!(
        lit > 0 && lit < out.len(),
        "one word's worth lit, got {lit}"
    );
}
