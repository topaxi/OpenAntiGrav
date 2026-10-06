//! **Superseded** - kept as evidence of a real negative. See
//! `docs/formats/gnf.md`'s "Tiling", "Superseded: the macro-tile search". This
//! searched `TileMode(13)` as macro-tiled `Thin_2DThin`; it is micro-tiled
//! `Thin_1DThin` (`Thin_2DThin` is index 14), which is why it scored at chance
//! across its whole space. `micro_tile_tests.rs` is the corrected investigation.
//! `#[ignore]`d, needs real game data, in-crate because it needs `pub(crate)`
//! `crate::bcn::bc7`.
//!
//! It includes `EgBasedLib::ComputeSurfaceAlignmentsMacroTiled`'s pre-alignment
//! steps and `HwlReduceBankWidthHeight`'s per-format reduction (both
//! `egbaddrlib.cpp`, Mesa's MIT `addrlib`), not just the raw address formula, and
//! is judged by pixel match across several differently sized real pairs.

use crate::bcn::bc7;
use crate::gnf::Texture;

use super::oracle_tests::{collect_oracle_pairs, mean_abs_diff, micro_tile_index};

#[derive(Clone, Copy, Debug)]
enum PipeConfig {
    P2,
    P4_8x16,
    P4_16x16,
    P4_16x32,
    P4_32x32,
    P8_16x16_8x16,
    P8_16x32_8x16,
    P8_16x32_16x16,
    P8_32x32_8x16,
    P8_32x32_16x16,
    P8_32x32_16x32,
    P8_32x64_32x32,
    P16_32x32_8x16,
    P16_32x32_16x16,
}

const ALL_PIPE_CONFIGS: [PipeConfig; 14] = [
    PipeConfig::P2,
    PipeConfig::P4_8x16,
    PipeConfig::P4_16x16,
    PipeConfig::P4_16x32,
    PipeConfig::P4_32x32,
    PipeConfig::P8_16x16_8x16,
    PipeConfig::P8_16x32_8x16,
    PipeConfig::P8_16x32_16x16,
    PipeConfig::P8_32x32_8x16,
    PipeConfig::P8_32x32_16x16,
    PipeConfig::P8_32x32_16x32,
    PipeConfig::P8_32x64_32x32,
    PipeConfig::P16_32x32_8x16,
    PipeConfig::P16_32x32_16x16,
];

impl PipeConfig {
    fn num_pipes(self) -> u32 {
        match self {
            Self::P2 => 2,
            Self::P4_8x16 | Self::P4_16x16 | Self::P4_16x32 | Self::P4_32x32 => 4,
            Self::P8_16x16_8x16
            | Self::P8_16x32_8x16
            | Self::P8_16x32_16x16
            | Self::P8_32x32_8x16
            | Self::P8_32x32_16x16
            | Self::P8_32x32_16x32
            | Self::P8_32x64_32x32 => 8,
            Self::P16_32x32_8x16 | Self::P16_32x32_16x16 => 16,
        }
    }

    fn pipe_bits(self, tx: u32, ty: u32) -> u32 {
        let x3 = tx & 1;
        let x4 = (tx >> 1) & 1;
        let x5 = (tx >> 2) & 1;
        let x6 = (tx >> 3) & 1;
        let y3 = ty & 1;
        let y4 = (ty >> 1) & 1;
        let y5 = (ty >> 2) & 1;
        let y6 = (ty >> 3) & 1;
        match self {
            Self::P2 => x3 ^ y3,
            Self::P4_8x16 => (x4 ^ y3) | ((x3 ^ y4) << 1),
            Self::P4_16x16 => (x3 ^ y3 ^ x4) | ((x4 ^ y4) << 1),
            Self::P4_16x32 => (x3 ^ y3 ^ x4) | ((x4 ^ y5) << 1),
            Self::P4_32x32 => (x3 ^ y3 ^ x5) | ((x5 ^ y5) << 1),
            Self::P8_16x16_8x16 => (x4 ^ y3 ^ x5) | ((x3 ^ y5) << 1),
            Self::P8_16x32_8x16 => (x4 ^ y3 ^ x5) | ((x3 ^ y4) << 1) | ((x4 ^ y5) << 2),
            Self::P8_16x32_16x16 => (x3 ^ y3 ^ x4) | ((x5 ^ y4) << 1) | ((x4 ^ y5) << 2),
            Self::P8_32x32_8x16 => (x4 ^ y3 ^ x5) | ((x3 ^ y4) << 1) | ((x5 ^ y5) << 2),
            Self::P8_32x32_16x16 => (x3 ^ y3 ^ x4) | ((x4 ^ y4) << 1) | ((x5 ^ y5) << 2),
            Self::P8_32x32_16x32 => (x3 ^ y3 ^ x4) | ((x4 ^ y6) << 1) | ((x5 ^ y5) << 2),
            Self::P8_32x64_32x32 => (x3 ^ y3 ^ x5) | ((x6 ^ y5) << 1) | ((x5 ^ y6) << 2),
            Self::P16_32x32_8x16 => {
                (x4 ^ y3) | ((x3 ^ y4) << 1) | ((x5 ^ y6) << 2) | ((x6 ^ y5) << 3)
            }
            Self::P16_32x32_16x16 => {
                (x3 ^ y3 ^ x4) | ((x4 ^ y4) << 1) | ((x5 ^ y6) << 2) | ((x6 ^ y5) << 3)
            }
        }
    }

    fn pre_adjust_bank(self, bank_width: u32, tile_x: u32, bank: u32) -> u32 {
        if bank_width == 1 && matches!(self, Self::P4_32x32 | Self::P8_32x64_32x32) {
            let bit0 = bank & 1;
            let x4 = (tile_x >> 1) & 1;
            let x5 = (tile_x >> 2) & 1;
            bank | (bit0 ^ x4 ^ x5)
        } else {
            bank
        }
    }
}

/// `pipe_interleave_bytes * bank_interleave` is 256 on every GFX6-8 GPU (kernel
/// doc, `AMD_FMT_MOD_PIPE_CONFIG` commit) and `m_bankInterleave` defaults to 1 in
/// every `addrlib` chip class (`egbaddrlib.cpp` constructor): neither is a search axis.
const PIPE_INTERLEAVE_BYTES: u32 = 256;
const BANK_INTERLEAVE: u32 = 1;

#[derive(Clone, Copy, Debug)]
struct StartConfig {
    pipe: PipeConfig,
    num_banks: u32,
    bank_width0: u32,
    bank_height0: u32,
    aspect0: u32,
    row_size: u32,
}

#[derive(Clone, Copy, Debug)]
struct TileConfig {
    pipe: PipeConfig,
    num_banks: u32,
    bank_width: u32,
    bank_height: u32,
    aspect: u32,
}

/// `EgBasedLib::ComputeSurfaceAlignmentsMacroTiled`'s pre-alignment steps plus
/// `HwlReduceBankWidthHeight` (`egbaddrlib.cpp`), ported for a single-sample BC7
/// (128 bits/element, thickness 1) surface with no tile split
/// (`tileSize = 64 * bpp / 8`). `None` for a start `SanityCheckMacroTiled` or the
/// reduction rejects.
fn reduce(start: &StartConfig) -> Option<TileConfig> {
    if start.num_banks < start.aspect0 || start.pipe.num_pipes() * start.num_banks < 4 {
        return None; // SanityCheckMacroTiled
    }
    let tile_size = 64 * 128 / 8; // 1024 bytes, bpp=128 fixed for BC7

    let mut bank_width = start.bank_width0;
    let mut bank_height = start.bank_height0;
    let mut aspect = start.aspect0;

    let bank_height_align =
        (PIPE_INTERLEAVE_BYTES * BANK_INTERLEAVE / (tile_size * bank_width)).max(1);
    bank_height = pow_two_align(bank_height, bank_height_align);
    let macro_aspect_align = (PIPE_INTERLEAVE_BYTES * BANK_INTERLEAVE
        / (tile_size * start.pipe.num_pipes() * bank_width))
        .max(1);
    aspect = pow_two_align(aspect, macro_aspect_align);

    if tile_size * bank_width * bank_height > start.row_size {
        let mut still_greater = true;
        let mut bank_height_align = bank_height_align;

        // First block: reduce bankWidth, only if it started above 1.
        if bank_width > 1 {
            while still_greater && bank_width > 0 {
                bank_width >>= 1;
                if bank_width == 0 {
                    bank_width = 1;
                    break;
                }
                still_greater = tile_size * bank_width * bank_height > start.row_size;
            }
            bank_height_align =
                (PIPE_INTERLEAVE_BYTES * BANK_INTERLEAVE / (tile_size * bank_width)).max(1);
            let macro_aspect_align = (PIPE_INTERLEAVE_BYTES * BANK_INTERLEAVE
                / (tile_size * start.pipe.num_pipes() * bank_width))
                .max(1);
            aspect = pow_two_align(aspect, macro_aspect_align);
        }

        // Second block: reduce bankHeight; the source runs both blocks in sequence.
        if still_greater && bank_height > bank_height_align {
            while still_greater && bank_height > bank_height_align {
                bank_height >>= 1;
                if bank_height < bank_height_align {
                    bank_height = bank_height_align;
                    break;
                }
                still_greater = tile_size * bank_width * bank_height > start.row_size;
            }
        }
        if still_greater {
            return None; // reduction failed to satisfy the row-size constraint
        }
    }

    Some(TileConfig {
        pipe: start.pipe,
        num_banks: start.num_banks,
        bank_width,
        bank_height,
        aspect,
    })
}

fn pow_two_align(value: u32, align: u32) -> u32 {
    if align <= 1 {
        return value;
    }
    value.div_ceil(align) * align
}

fn bank_from_coord(x: u32, y: u32, cfg: &TileConfig) -> u32 {
    let pipes = cfg.pipe.num_pipes();
    let tx = x / 8 / (cfg.bank_width * pipes);
    let ty = y / 8 / cfg.bank_height;
    let x3 = tx & 1;
    let x4 = (tx >> 1) & 1;
    let x5 = (tx >> 2) & 1;
    let x6 = (tx >> 3) & 1;
    let y3 = ty & 1;
    let y4 = (ty >> 1) & 1;
    let y5 = (ty >> 2) & 1;
    let y6 = (ty >> 3) & 1;
    let bank = match cfg.num_banks {
        16 => (x3 ^ y6) | ((x4 ^ y5 ^ y6) << 1) | ((x5 ^ y4) << 2) | ((x6 ^ y3) << 3),
        8 => (x3 ^ y5) | ((x4 ^ y4 ^ y5) << 1) | ((x5 ^ y3) << 2),
        4 => (x3 ^ y4) | ((x4 ^ y3) << 1),
        2 => x3 ^ y3,
        _ => 0,
    };
    cfg.pipe.pre_adjust_bank(cfg.bank_width, x / 8, bank)
}

fn macro_tiled_address(x: u32, y: u32, pitch: u32, cfg: &TileConfig) -> u64 {
    let bpp = 128u32;
    let pipes = cfg.pipe.num_pipes();
    let num_pipe_interleave_bits = PIPE_INTERLEAVE_BYTES.trailing_zeros();
    let num_pipe_bits = pipes.trailing_zeros();
    let num_bank_interleave_bits = BANK_INTERLEAVE.trailing_zeros();
    let num_bank_bits = cfg.num_banks.trailing_zeros();

    let micro_tile_bytes = 64 * bpp / 8;
    let pixel_index = micro_tile_index(x % 8, y % 8);
    let element_offset = (pixel_index * bpp) / 8;

    let macro_tile_pitch = 8 * cfg.bank_width * pipes * cfg.aspect;
    let macro_tile_height = 8 * cfg.bank_height * cfg.num_banks / cfg.aspect.max(1);
    let macro_tile_bytes = u64::from(micro_tile_bytes)
        * u64::from(macro_tile_pitch / 8)
        * u64::from(macro_tile_height / 8)
        / u64::from(pipes * cfg.num_banks);

    let macro_tiles_per_row = pitch / macro_tile_pitch;
    let macro_tile_index_x = x / macro_tile_pitch;
    let macro_tile_index_y = y / macro_tile_height;
    let macro_tile_offset = (u64::from(macro_tile_index_y) * u64::from(macro_tiles_per_row)
        + u64::from(macro_tile_index_x))
        * macro_tile_bytes;

    let tile_row_index = (y / 8) % cfg.bank_height;
    let tile_column_index = ((x / 8) / pipes) % cfg.bank_width;
    let tile_index = tile_row_index * cfg.bank_width + tile_column_index;
    let tile_offset = u64::from(tile_index) * u64::from(micro_tile_bytes);

    let total_offset = macro_tile_offset + u64::from(element_offset) + tile_offset;

    let pipe = cfg.pipe.pipe_bits(x / 8, y / 8);
    let bank = bank_from_coord(x, y, cfg);

    let pipe_interleave_mask = u64::from((1u32 << num_pipe_interleave_bits) - 1);
    let bank_interleave_mask = u64::from((1u32 << num_bank_interleave_bits).wrapping_sub(1));
    let pipe_interleave_offset = total_offset & pipe_interleave_mask;
    let bank_interleave_offset = (total_offset >> num_pipe_interleave_bits) & bank_interleave_mask;
    let offset = total_offset >> (num_pipe_interleave_bits + num_bank_interleave_bits);

    let mut addr = pipe_interleave_offset;
    addr |= u64::from(pipe) << num_pipe_interleave_bits;
    addr |= bank_interleave_offset << (num_pipe_interleave_bits + num_pipe_bits);
    addr |=
        u64::from(bank) << (num_pipe_interleave_bits + num_pipe_bits + num_bank_interleave_bits);
    addr |= offset
        << (num_pipe_interleave_bits + num_pipe_bits + num_bank_interleave_bits + num_bank_bits);
    addr
}

fn decode_with(blob: &[u8], texture: &Texture, cfg: &TileConfig) -> Vec<[u8; 4]> {
    let width_blocks = texture.width.div_ceil(4);
    let height_blocks = texture.height.div_ceil(4);
    let pitch_blocks = texture.pitch.div_ceil(4);
    let mut out = vec![[0u8; 4]; (texture.width * texture.height) as usize];
    for by in 0..height_blocks {
        for bx in 0..width_blocks {
            let addr = macro_tiled_address(bx, by, pitch_blocks, cfg);
            let start = texture.data_offset + addr as usize;
            let Some(block) = blob.get(start..start + 16).and_then(|s| s.try_into().ok()) else {
                continue;
            };
            let texels = bc7(&block);
            for ty in 0..4 {
                for tx in 0..4 {
                    let px = bx * 4 + tx;
                    let py = by * 4 + ty;
                    if px < texture.width && py < texture.height {
                        out[(py * texture.width + px) as usize] = texels[(ty * 4 + tx) as usize];
                    }
                }
            }
        }
    }
    out
}

#[test]
#[ignore]
fn search_the_reduced_tile_config_space_against_multi_size_oracle_pairs() {
    let pairs = collect_oracle_pairs(6);
    println!("{} oracle pairs collected for the search", pairs.len());
    assert!(!pairs.is_empty(), "no oracle pairs found");

    let mut results: Vec<(f64, StartConfig, TileConfig)> = Vec::new();
    for &pipe in &ALL_PIPE_CONFIGS {
        for &num_banks in &[2u32, 4, 8, 16] {
            for &bw0 in &[1u32, 2, 4, 8] {
                for &bh0 in &[1u32, 2, 4, 8] {
                    for &aspect0 in &[1u32, 2, 4, 8] {
                        for &row_size in &[1024u32, 2048, 4096] {
                            let start = StartConfig {
                                pipe,
                                num_banks,
                                bank_width0: bw0,
                                bank_height0: bh0,
                                aspect0,
                                row_size,
                            };
                            let Some(cfg) = reduce(&start) else {
                                continue;
                            };
                            let mut total = 0.0;
                            for (_, texture, blob, truth) in &pairs {
                                let decoded = decode_with(blob, texture, &cfg);
                                total += mean_abs_diff(&decoded, truth);
                            }
                            let mean = total / pairs.len() as f64;
                            results.push((mean, start, cfg));
                        }
                    }
                }
            }
        }
    }
    results.sort_by(|a, b| a.0.total_cmp(&b.0));
    println!("{} valid configurations searched", results.len());
    for (mean, start, cfg) in results.iter().take(10) {
        println!("mean MAD {mean:.2}  start={start:?}  reduced={cfg:?}");
    }

    // Per-pair breakdown for the best candidate, so a partial match shows.
    if let Some((_, _, best)) = results.first() {
        for (path, texture, blob, truth) in &pairs {
            let decoded = decode_with(blob, texture, best);
            let mad = mean_abs_diff(&decoded, truth);
            println!("  best candidate vs {path}: MAD {mad:.2}");
        }
    }
}
