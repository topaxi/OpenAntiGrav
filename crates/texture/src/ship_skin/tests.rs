//! What [`super`] is asserted to do: the fixed-length layout, the
//! NUL-terminated header, and 4bpp unpacking. No game data in any test.

use super::*;

/// Builds a skin blob by hand, filling each block's palette and pixels with a
/// value that identifies the block, so a test can tell them apart.
fn build(team_name: &str) -> Vec<u8> {
    let mut out = vec![0u8; FILE_LEN];
    let name = team_name.as_bytes();
    assert!(name.len() < HEADER_LEN, "test header would not fit");
    out[..name.len()].copy_from_slice(name);

    let sizes = [
        (LARGE_SIDE, LARGE_SIDE),
        (LARGE_SIDE, LARGE_SIDE),
        (LARGE_SIDE, LARGE_SIDE),
        (SMALL_SIDE, SMALL_SIDE),
    ];
    let mut offset = HEADER_LEN;
    for (block, (width, height)) in sizes.into_iter().enumerate() {
        let block = block as u8;
        for entry in 0..16u8 {
            let at = offset + entry as usize * 4;
            out[at..at + 4].copy_from_slice(&[block, entry, entry, 255]);
        }
        let pixels_at = offset + PALETTE_LEN;
        // Every byte packs indices `block % 16` (low) and `(block + 1) % 16`
        // (high), so `to_rgba` on this block reads a two-colour checkerboard.
        let byte = (block % 16) | (((block + 1) % 16) << 4);
        out[pixels_at..pixels_at + width * height / 2].fill(byte);
        offset += PALETTE_LEN + width * height / 2;
    }
    assert_eq!(offset, FILE_LEN);
    out
}

/// The byte count of all sixteen shipped files, matching
/// `docs/ghidra/functions/psp-pulse-usa/ship-skin.md`'s arithmetic.
#[test]
fn file_len_is_the_disc_measured_26912_bytes() {
    assert_eq!(FILE_LEN, 26_912);
}

#[test]
fn a_well_formed_blob_parses_into_four_blocks_of_the_right_shape() {
    let blob = build("Assegai");
    let skin = parse(&blob).expect("a well-formed skin must parse");
    assert_eq!(skin.team_name, "Assegai");
    assert_eq!(skin.blocks.len(), 4);
    for (index, block) in skin.blocks.iter().enumerate() {
        let (width, height) = if index < 3 {
            (LARGE_SIDE, LARGE_SIDE)
        } else {
            (SMALL_SIDE, SMALL_SIDE)
        };
        assert_eq!(
            (block.width, block.height),
            (width, height),
            "block {index}"
        );
        assert_eq!(block.indices.len(), width * height, "block {index}");
    }
}

/// Everything after the header's terminator is exporter residue, not a
/// field - `Assegai\0ms\0` reads as `Assegai`, matching `ship-skin.md`'s
/// finding on the real file.
#[test]
fn the_header_stops_at_the_first_nul_even_when_bytes_follow() {
    let mut blob = build("Assegai");
    // `Assegai\0` is eight bytes; overwrite the next two the way a reused
    // export buffer that last held `AG Systems\0` would.
    blob[8] = b'm';
    blob[9] = b's';
    let skin = parse(&blob).expect("parses");
    assert_eq!(skin.team_name, "Assegai");
}

#[test]
fn a_ten_byte_name_fills_the_header_with_no_room_for_residue() {
    let blob = build("AG Systems");
    let skin = parse(&blob).expect("parses");
    assert_eq!(skin.team_name, "AG Systems");
}

/// Low nibble first: pixel 0 is the low half of byte 0, as in `texture::Texture`.
#[test]
fn nibbles_unpack_low_half_first() {
    let blob = build("Test");
    let skin = parse(&blob).expect("parses");
    // Block 0's packed byte is `0 | (1 << 4)`: index 0 low, index 1 high.
    assert_eq!(&skin.blocks[0].indices[..2], &[0, 1]);
}

#[test]
fn to_rgba_reads_the_palette_through_the_indices() {
    let blob = build("Test");
    let skin = parse(&blob).expect("parses");
    let rgba = skin.blocks[0].to_rgba();
    assert_eq!(rgba.len(), skin.blocks[0].indices.len() * 4);
    // Palette entry 0 is [block=0, entry=0, entry=0, 255]; entry 1 is
    // [0, 1, 1, 255] - and pixel 0 is index 0, pixel 1 is index 1.
    assert_eq!(&rgba[..4], &[0, 0, 0, 255]);
    assert_eq!(&rgba[4..8], &[0, 1, 1, 255]);
}

/// The layout fixes the size: a truncated or padded blob is refused, since
/// nothing in the file says otherwise.
#[test]
fn a_blob_of_the_wrong_length_is_refused() {
    let mut blob = build("Test");
    blob.pop();
    assert_eq!(parse(&blob), Err(Error::SizeMismatch { got: FILE_LEN - 1 }));

    let mut blob = build("Test");
    blob.push(0);
    assert_eq!(parse(&blob), Err(Error::SizeMismatch { got: FILE_LEN + 1 }));
}
