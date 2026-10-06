//! A flagged (`+0x06` bit 0) Pulse PSP texture is swizzled in the file, and
//! the shield shell's decode reads it that way.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # What was measured, 2026-10-01 (PPSSPP v1.20.4, `UCUS98612`)
//!
//! A GE dump of a raised shield on Talon's Junction shows `TEXMODE` bit 0
//! (swizzle) set on every draw, the shell's included. The shell's level-0
//! bytes in the dump equal the file's, byte for byte, and so do levels 1 to 3
//! (1,024, 256 and 128 bytes); an unflagged hull texture's level 0 in the dump
//! is the file's bytes **reordered** (the loader swizzles it). So the GE reads
//! everything swizzled, an unflagged file is swizzled at load, and a flagged
//! one already is. Rasterising the shell's two draws from the dump with the
//! unswizzled texture reproduces the original's pixels (blue energy within
//! 0.1 %, correlation 0.92 against the original frame); with a linear read it
//! does not (correlation 0.4). See
//! `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
//!
//! The reference below is written out here rather than taken from
//! `oag_formats::swizzle`, so a decoder that stops unswizzling cannot pass by
//! agreeing with itself.

use oag_vex::vex;

const SHELL: &str = r"Data\Ships\Assegai\shipshield.vex";

/// Reorders a linear plane into the GE's 16-byte by 8-row block order, the
/// direction the loader goes. Inverting it is what a decode must do.
fn swizzle(linear: &[u8], stride: usize, height: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(linear.len());
    for block_row in 0..height / 8 {
        for block_col in 0..stride / 16 {
            for row in 0..8 {
                let at = (block_row * 8 + row) * stride + block_col * 16;
                out.extend_from_slice(&linear[at..at + 16]);
            }
        }
    }
    out
}

/// Pixel indices of a 4-bit plane, low nibble first.
fn nibbles(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().flat_map(|b| [b & 15, b >> 4]).collect()
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn the_shield_shell_texture_is_decoded_from_its_swizzled_file_bytes() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives =
        oag_pulse::open(image.to_str().expect("utf-8 path")).expect("opening the disc");
    let blob = archives.read_name(SHELL).expect("the Assegai shell");
    let texture = vex::textures(&blob)
        .expect("textures")
        .into_iter()
        .next()
        .expect("one texture node")
        .expect("it decodes");
    assert_eq!(
        (texture.width, texture.height, texture.bits_per_pixel),
        (128, 64, 4)
    );

    // The texel block is the last thing in the file: 4,096 + 1,024 + 256 + 128
    // bytes for 4 levels, each at its own 16-byte-padded stride.
    let block = &blob[blob.len() - 5504..];
    let level0 = &block[..4096];
    let flat: Vec<u8> = nibbles(level0);
    assert_ne!(
        texture.indices, flat,
        "the shell decoded as a linear read of its bytes: the swizzle flag (+0x06 bit 0) is ignored again"
    );
    // Level 0 decodes to the order whose swizzle is the file's bytes.
    let rows: Vec<u8> = {
        // Unswizzle by brute force: find the linear plane `p` with swizzle(p) == level0
        // by laying the file's 16-byte chunks where the block order puts them.
        let mut plane = vec![0u8; 4096];
        let mut chunk = 0;
        for block_row in 0..8 {
            for block_col in 0..4 {
                for row in 0..8 {
                    let at = (block_row * 8 + row) * 64 + block_col * 16;
                    plane[at..at + 16].copy_from_slice(&level0[chunk * 16..chunk * 16 + 16]);
                    chunk += 1;
                }
            }
        }
        assert_eq!(
            swizzle(&plane, 64, 64),
            level0,
            "the reference is its own inverse"
        );
        nibbles(&plane)
    };
    assert_eq!(
        texture.indices, rows,
        "level 0 is the file's bytes, unswizzled"
    );

    // Levels 1 to 3, each swizzled on its own (the GE is given an address per
    // level: 64x32 at stride 32, 32x16 at 16, 16x8 at 16 (8 bytes of picture,
    // padded to the 16-byte stride).
    assert_eq!(
        texture.levels.len(),
        3,
        "the three authored levels below the base"
    );
    let mut offset = 4096;
    for (level, (stride, row_bytes, height)) in
        [(32usize, 32usize, 32usize), (16, 16, 16), (16, 8, 8)]
            .into_iter()
            .enumerate()
    {
        let bytes = &block[offset..offset + stride * height];
        let mut plane = vec![0u8; bytes.len()];
        let mut chunk = 0;
        for block_row in 0..height / 8 {
            for block_col in 0..stride / 16 {
                for row in 0..8 {
                    let at = (block_row * 8 + row) * stride + block_col * 16;
                    plane[at..at + 16].copy_from_slice(&bytes[chunk * 16..chunk * 16 + 16]);
                    chunk += 1;
                }
            }
        }
        assert_eq!(
            texture.levels[level],
            nibbles(
                &plane
                    .chunks(stride)
                    .flat_map(|row| row[..row_bytes].iter().copied())
                    .collect::<Vec<u8>>()
            ),
            "level {} decodes to its own unswizzle",
            level + 1
        );
        offset += stride * height;
    }
}
