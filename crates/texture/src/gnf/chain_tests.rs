//! [`Texture::block_levels`] on a hand-built micro-tiled chain.

use super::*;

/// The `Thin` micro-tile order the decoder reads: `x0, y0, x1, y1, x2, y2`.
fn tile_slot(bx: u32, by: u32) -> usize {
    let bit = |v: u32, i: u32| ((v >> i) & 1) as usize;
    bit(bx, 0)
        | bit(by, 0) << 1
        | bit(bx, 1) << 2
        | bit(by, 1) << 3
        | bit(bx, 2) << 4
        | bit(by, 2) << 5
}

/// A 16x16 BC7 `Thin_1DThin` texture with two levels: 4x4 blocks, then 2x2.
/// Every block's byte 0 is `1` (a valid mode) and byte 1 its own tag,
/// `level * 16 + by * 4 + bx`, so a wrong offset shows as a wrong tag.
fn chain(extra: usize) -> Vec<u8> {
    let contents_len = 8 + DESCRIPTOR_LEN;
    let data_offset = HEADER_LEN + contents_len;
    let mut bytes = vec![0u8; data_offset + 2 * 1024 + extra];
    bytes[0..4].copy_from_slice(b"GNF ");
    bytes[4..8].copy_from_slice(&(contents_len as u32).to_le_bytes());
    bytes[8] = 2;
    bytes[9] = 1;
    bytes[10] = 8;
    let at = 16;
    let word1 = 0x29u32 << 20;
    let word2 = 15u32 | (15u32 << 14);
    let word3 = (0x0du32 << 20) | (1 << 16); // Thin_1DThin, last mip 1
    let word4 = 15u32 << 13; // pitch 16
    for (offset, word) in [(4, word1), (8, word2), (12, word3), (16, word4)] {
        bytes[at + offset..at + offset + 4].copy_from_slice(&word.to_le_bytes());
    }
    for (level, size) in [(0u32, 4u32), (1, 2)] {
        for by in 0..size {
            for bx in 0..size {
                let start = data_offset + level as usize * 1024 + tile_slot(bx, by) * 16;
                bytes[start] = 1;
                bytes[start + 1] = (level * 16 + by * 4 + bx) as u8;
            }
        }
    }
    bytes
}

#[test]
fn each_level_comes_back_row_major_from_its_own_tile() {
    let bytes = chain(0);
    let texture = Texture::parse(&bytes).unwrap();
    let levels = texture.block_levels(&bytes).unwrap();
    assert_eq!(levels.len(), 2);
    assert_eq!(levels[0].len(), 16 * 16);
    assert_eq!(levels[1].len(), 4 * 16);
    for (level, size) in [(0usize, 4u32), (1, 2)] {
        for by in 0..size {
            for bx in 0..size {
                let at = (by * size + bx) as usize * 16;
                assert_eq!(
                    levels[level][at + 1] as u32,
                    level as u32 * 16 + by * 4 + bx
                );
            }
        }
    }
}

#[test]
fn a_file_that_is_more_than_one_chain_is_refused() {
    let bytes = chain(1024);
    let texture = Texture::parse(&bytes).unwrap();
    assert_eq!(
        texture.block_levels(&bytes),
        Err(Error::ChainLayout {
            expected: 2048,
            found: 3072
        })
    );
}

#[test]
fn a_corrupt_block_in_a_lower_level_refuses_the_chain_though_the_base_decodes() {
    let mut bytes = chain(0);
    let texture = Texture::parse(&bytes).unwrap();
    bytes[texture.data_offset + 1024 + tile_slot(1, 1) * 16] = 0;
    assert!(texture.decode(&bytes).is_ok());
    assert_eq!(
        texture.block_levels(&bytes),
        Err(Error::CorruptBlocks { count: 1 })
    );
}

#[test]
fn the_base_level_decodes_the_same_from_blocks_as_from_the_file() {
    let bytes = chain(0);
    let texture = Texture::parse(&bytes).unwrap();
    let levels = texture.block_levels(&bytes).unwrap();
    assert_eq!(
        decode_bc7_level(&levels[0], 16, 16).unwrap(),
        texture.decode(&bytes).unwrap()
    );
}
