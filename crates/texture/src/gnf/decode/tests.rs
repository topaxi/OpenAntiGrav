use super::*;
use crate::gnf::{ChannelType, TileMode};

fn texture(format: SurfaceFormat, width: u32, height: u32, data_offset: usize) -> Texture {
    Texture {
        version: 2,
        alignment: 8,
        stream_size: 0,
        surface_format: format,
        channel_type: ChannelType::Unorm,
        width,
        height,
        depth: 1,
        pitch: width,
        base_mip_level: 0,
        last_mip_level: 0,
        base_array_slice: 0,
        last_array_slice: 0,
        is_pow2_pad: false,
        tile_mode: TileMode(TileMode::THIN_1D_THIN),
        data_offset,
    }
}

/// A byte stream with every value in it, some of them zero.
fn noise(len: usize) -> Vec<u8> {
    let mut state = 0x2545_f491_4f6c_dd1du64;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8 & 0xc3
        })
        .collect()
}

#[test]
fn the_tile_walk_untiles_what_the_block_walk_does() {
    // Tile-aligned, ragged on either edge, a pitch wider than the width, and a
    // level that starts part way into the file.
    for (width_blocks, height_blocks, tiles_x) in [
        (8, 8, 1),
        (16, 8, 2),
        (13, 11, 2),
        (3, 20, 1),
        (9, 9, 3),
        (1, 1, 1),
    ] {
        for (format, block_bytes) in [(SurfaceFormat::Bc7, 16), (SurfaceFormat::Bc1, 8)] {
            let tex = texture(format, width_blocks * 4, height_blocks * 4, 7);
            let tiles = tiles_x as usize * (height_blocks as usize).div_ceil(8);
            let blob = noise(7 + 5 + tiles * 64 * block_bytes);
            let grid = (width_blocks, height_blocks, tiles_x);
            let fast = untile_level(&tex, &blob, 5, grid, block_bytes).expect("fits");
            let slow = untile_level_by_block(&tex, &blob, 5, grid, block_bytes).expect("fits");
            assert_eq!(
                fast, slow,
                "{width_blocks}x{height_blocks} blocks, {tiles_x} tiles wide"
            );
        }
    }
}

#[test]
fn a_short_level_names_the_same_missing_block() {
    let tex = texture(SurfaceFormat::Bc7, 52, 44, 0);
    let grid = (13, 11, 2);
    for len in [0, 1000, 16 * 64 * 4 - 1, 16 * 64 * 4 + 40, 16 * 64 * 3] {
        let blob = noise(len);
        assert_eq!(
            untile_level(&tex, &blob, 0, grid, 16),
            untile_level_by_block(&tex, &blob, 0, grid, 16),
            "{len} bytes"
        );
    }
}
