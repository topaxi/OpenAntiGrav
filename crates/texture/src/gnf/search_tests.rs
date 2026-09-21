//! A bounded search over the GCN macro-tile configuration space
//! `TileMode(13)` does not fully determine on its own - see
//! `docs/formats/gnf.md`'s "Tiling" section for why the search exists and
//! what its (negative) result was. `#[ignore]`d, needs real game data, kept
//! in-crate (rather than as an example under `examples/`) because it needs
//! `crate::bcn::bc7`, which is `pub(crate)` and invisible outside this
//! crate.
//!
//! Ported more faithfully than an earlier, deleted attempt: this includes
//! `EgBasedLib::ComputeSurfaceAlignmentsMacroTiled`'s pre-alignment steps
//! and `HwlReduceBankWidthHeight`'s per-format bank-width/height reduction
//! (both `egbaddrlib.cpp` in Mesa's MIT `addrlib`), not just the raw
//! address formula. Judged by pixel match against the oracle across several
//! differently sized real pairs - not eyeballed on one texture, which is
//! exactly the mistake the earlier attempt made.

use oag_assets::psarc::Archive;

use crate::bcn::bc7;
use crate::gnf::{SurfaceFormat, Texture};

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

/// `pipe_interleave_bytes * bank_interleave` is 256 on every GFX6-8 GPU
/// (kernel doc, `AMD_FMT_MOD_PIPE_CONFIG` commit) and `m_bankInterleave`
/// defaults to 1 in every `addrlib` chip class (`egbaddrlib.cpp`
/// constructor, never reassigned elsewhere) - neither is a search axis.
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

/// `EgBasedLib::ComputeSurfaceAlignmentsMacroTiled`'s pre-alignment steps
/// plus `HwlReduceBankWidthHeight`, both `egbaddrlib.cpp`, faithfully
/// ported for a single-sample, BC7 (128 bits/element, thickness 1) surface
/// with no tile split (`tileSplitBytes` treated as unconstrained, so
/// `tileSize = 64 * bpp / 8` unconditionally). Returns `None` for a start
/// `SanityCheckMacroTiled` or the reduction itself rejects.
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

        // Second block: reduce bankHeight, unconditionally attempted next
        // (not an "else" - the source runs both blocks in sequence).
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

fn micro_tile_index(bx: u32, by: u32) -> u32 {
    let x0 = bx & 1;
    let x1 = (bx >> 1) & 1;
    let x2 = (bx >> 2) & 1;
    let y0 = by & 1;
    let y1 = (by >> 1) & 1;
    let y2 = (by >> 2) & 1;
    x0 | (y0 << 1) | (x1 << 2) | (y1 << 3) | (x2 << 4) | (y2 << 5)
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

fn mean_abs_diff(a: &[[u8; 4]], b: &[[u8; 4]]) -> f64 {
    let mut total = 0u64;
    for (x, y) in a.iter().zip(b) {
        for c in 0..4 {
            total += u64::from(x[c].abs_diff(y[c]));
        }
    }
    total as f64 / (a.len() * 4) as f64
}

fn team_token(path: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase().replace('\\', "/");
    let parts: Vec<&str> = lower.split('/').collect();
    for (i, p) in parts.iter().enumerate() {
        if *p == "hdships" || *p == "ships" {
            return parts.get(i + 1).map(|s| (*s).to_string());
        }
    }
    None
}

fn livery_token(path: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase().replace('\\', "/");
    lower
        .split('/')
        .find(|p| p.contains("livery"))
        .map(str::to_string)
}

/// One oracle pair: the `.gnf` path, its parsed descriptor, its raw bytes,
/// and the HD `.gtf` twin already decoded to RGBA8 texels.
type OraclePair = (String, Texture, Vec<u8>, Vec<[u8; 4]>);

/// Collects up to `max` team/livery-matched, same-dimension `.gnf`/`.gtf`
/// pairs, favouring a spread of sizes (sorted by block area then taken
/// evenly) so the search is judged across several sizes, not one.
fn collect_oracle_pairs(max: usize) -> Vec<OraclePair> {
    let Some(base) = oag_testdata::exact("data/extracted/ps4/omega-eu/uroot/data03.psarc") else {
        return Vec::new();
    };
    let Ok(mut gnf_archive) = Archive::open_file(&base) else {
        return Vec::new();
    };
    let mut candidates: Vec<(String, Texture, Vec<u8>)> = Vec::new();
    let paths: Vec<String> = gnf_archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".gnf"))
        .cloned()
        .collect();
    for path in paths {
        let Ok(blob) = gnf_archive.read_path(&path) else {
            continue;
        };
        let Ok(texture) = Texture::parse(&blob) else {
            continue;
        };
        if texture.surface_format != SurfaceFormat::Bc7 || texture.tile_mode.0 != 13 {
            continue;
        }
        candidates.push((path, texture, blob));
    }
    candidates.sort_by_key(|(_, t, _)| t.width * t.height);

    let mut out = Vec::new();
    let step = (candidates.len() / max.max(1)).max(1);
    let mut i = 0;
    while i < candidates.len() && out.len() < max {
        let (path, texture, blob) = &candidates[i];
        let basename = path
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .to_ascii_lowercase()
            .replace(".gnf", ".gtf");
        if let Some((hw, hh, truth)) = find_hd_twin(&basename, path)
            && (hw, hh) == (texture.width, texture.height)
        {
            println!("oracle pair: {path} ({hw}x{hh})");
            out.push((path.clone(), *texture, blob.clone(), truth));
        }
        i += step;
    }
    out
}

fn find_hd_twin(basename: &str, gnf_path: &str) -> Option<(u32, u32, Vec<[u8; 4]>)> {
    let want_team = team_token(gnf_path)?;
    let want_livery = livery_token(gnf_path);
    for index in 0..7 {
        let path = oag_testdata::exact(&format!(
            "data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/DATA0{index}.PSARC"
        ))?;
        let Ok(mut a) = Archive::open_file(&path) else {
            continue;
        };
        let entries: Vec<String> = a
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(basename))
            .filter(|p| team_token(p).as_deref() == Some(want_team.as_str()))
            .filter(|p| want_livery.is_none() || livery_token(p) == want_livery)
            .cloned()
            .collect();
        for entry in entries {
            if let Ok(blob) = a.read_path(&entry) {
                let Ok(parsed) = crate::gtf::Gtf::parse(&blob) else {
                    continue;
                };
                let Some(t) = parsed.only() else { continue };
                let Ok(rgba) = t.to_rgba(&blob) else { continue };
                return Some((u32::from(t.width), u32::from(t.height), rgba));
            }
        }
    }
    None
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

    // Per-pair breakdown for the best candidate, so a partial match is
    // visible rather than hidden behind one averaged number.
    if let Some((_, _, best)) = results.first() {
        for (path, texture, blob, truth) in &pairs {
            let decoded = decode_with(blob, texture, best);
            let mad = mean_abs_diff(&decoded, truth);
            println!("  best candidate vs {path}: MAD {mad:.2}");
        }
    }
}

/// `ADDR_DISPLAYABLE`, `bpp == 128`: `pixelBit0=y0,1=x0,2=x1,3=x2,4=y1,5=y2`
/// (`Lib::ComputePixelIndexWithinMicroTile`, `addrlib1.cpp`).
fn micro_tile_index_displayable(bx: u32, by: u32) -> u32 {
    let x0 = bx & 1;
    let x1 = (bx >> 1) & 1;
    let x2 = (bx >> 2) & 1;
    let y0 = by & 1;
    let y1 = (by >> 1) & 1;
    let y2 = (by >> 2) & 1;
    y0 | (x0 << 1) | (x1 << 2) | (x2 << 3) | (y1 << 4) | (y2 << 5)
}

/// Micro-tile traversal order across the surface - the one axis this
/// project's own copy of the address formula assumed (row-major) without
/// measuring, unlike every bit-level formula above which is a direct port.
#[derive(Clone, Copy, Debug)]
enum TileOrder {
    RowMajor,
    ColumnMajor,
    Morton,
}

/// Interleaves the low bits of `x` and `y` (x in odd positions, y in even),
/// the same general Z-order this project's own `oag_texture::gxt::twiddle`
/// uses for the Vita's GXM block grid - a candidate for how micro *tiles*
/// (not texels) might be arranged across a `1D_TILED_THIN1` surface.
fn morton(x: u32, y: u32) -> u64 {
    fn spread(v: u32) -> u64 {
        let mut v = u64::from(v);
        v = (v | (v << 16)) & 0x0000_ffff_0000_ffff;
        v = (v | (v << 8)) & 0x00ff_00ff_00ff_00ff;
        v = (v | (v << 4)) & 0x0f0f_0f0f_0f0f_0f0f;
        v = (v | (v << 2)) & 0x3333_3333_3333_3333;
        v = (v | (v << 1)) & 0x5555_5555_5555_5555;
        v
    }
    spread(x) | (spread(y) << 1)
}

fn decode_micro_tiled(
    blob: &[u8],
    texture: &Texture,
    micro_tile_index: impl Fn(u32, u32) -> u32 + Copy,
    order: TileOrder,
    flip_y: bool,
) -> Vec<[u8; 4]> {
    let width_blocks = texture.width.div_ceil(4);
    let height_blocks = texture.height.div_ceil(4);
    let pitch_blocks = texture.pitch.div_ceil(4);
    let tiles_x = pitch_blocks.div_ceil(8);
    let tiles_y = height_blocks.div_ceil(8);
    let mut out = vec![[0u8; 4]; (texture.width * texture.height) as usize];
    for by in 0..height_blocks {
        for bx in 0..width_blocks {
            let tile_x = bx / 8;
            let tile_y = by / 8;
            let tile_index: u64 = match order {
                TileOrder::RowMajor => u64::from(tile_y * tiles_x + tile_x),
                TileOrder::ColumnMajor => u64::from(tile_x * tiles_y + tile_y),
                TileOrder::Morton => morton(tile_x, tile_y),
            };
            let micro_tile_bytes: u32 = 64 * 128 / 8;
            let pixel_index = micro_tile_index(bx % 8, by % 8);
            let element_offset = pixel_index * 128 / 8;
            let addr = tile_index * u64::from(micro_tile_bytes) + u64::from(element_offset);
            let start = texture.data_offset + addr as usize;
            let Some(block) = blob.get(start..start + 16).and_then(|s| s.try_into().ok()) else {
                continue;
            };
            let texels = bc7(&block);
            for ty in 0..4 {
                for tx in 0..4 {
                    let px = bx * 4 + tx;
                    let py_raw = by * 4 + ty;
                    let py = if flip_y {
                        texture.height.saturating_sub(1).saturating_sub(py_raw)
                    } else {
                        py_raw
                    };
                    if px < texture.width && py < texture.height {
                        out[(py * texture.width + px) as usize] = texels[(ty * 4 + tx) as usize];
                    }
                }
            }
        }
    }
    out
}

/// Tests the hypothesis that `TileMode(13)` is `Thin_1DThin` (micro-tiled
/// only - GFD-Studio's own `TileMode.cs` enum, fetched directly and
/// re-checked against this project's own prior "Thin_2DThin" label, names
/// index 13 `Thin_1DThin` and index **14** `Thin_2DThin`; every real `.gnf`
/// this project has found declares 13, not 14), against the same
/// bounded-search's oracle pairs - a completely different, much simpler
/// address formula with no bank/pipe/row-size unknowns at all.
#[test]
#[ignore]
fn micro_tiled_address_against_the_oracle_pairs() {
    let pairs = collect_oracle_pairs(6);
    println!("{} oracle pairs collected", pairs.len());
    assert!(!pairs.is_empty());

    for (path, texture, _, _) in &pairs {
        println!(
            "  {path}: width={} height={} pitch={} data_offset={} mips={}..{}",
            texture.width,
            texture.height,
            texture.pitch,
            texture.data_offset,
            texture.base_mip_level,
            texture.last_mip_level
        );
    }

    type IndexFn = fn(u32, u32) -> u32;
    let index_candidates: [(&str, IndexFn); 2] = [
        ("non_displayable (Thin)", micro_tile_index),
        ("displayable (Display)", micro_tile_index_displayable),
    ];
    let orders = [
        ("row-major", TileOrder::RowMajor),
        ("column-major", TileOrder::ColumnMajor),
        ("morton", TileOrder::Morton),
    ];
    for (index_name, index_fn) in index_candidates {
        for (order_name, order) in orders {
            for flip_y in [false, true] {
                let mut total = 0.0;
                for (path, texture, blob, truth) in &pairs {
                    let decoded = decode_micro_tiled(blob, texture, index_fn, order, flip_y);
                    let mad = mean_abs_diff(&decoded, truth);
                    total += mad;
                    if texture.width == 32 {
                        // The single-micro-tile sample only tests the
                        // intra-tile bit order, not tile traversal or flip -
                        // skip it from this per-combination print to keep
                        // the interesting (multi-tile) rows visible.
                        continue;
                    }
                    println!("  {index_name}/{order_name}/flip_y={flip_y} vs {path}: MAD {mad:.2}");
                }
                println!(
                    "{index_name}/{order_name}/flip_y={flip_y}: mean MAD {:.2}",
                    total / pairs.len() as f64
                );
            }
        }
    }

    // Diagnostic: is the FIRST micro tile of a multi-tile image byte-exact
    // (proving the corruption starts at tile 1, an inter-tile problem), or
    // is tile 0 itself already wrong for a multi-mip-level file (pointing
    // at data_offset/mip-chain layout instead)?
    for (path, texture, blob, truth) in &pairs {
        if texture.width == 32 {
            continue;
        }
        let decoded =
            decode_micro_tiled(blob, texture, micro_tile_index, TileOrder::RowMajor, false);
        let w = texture.width as usize;
        let mut first_tile_mad = 0.0;
        let mut n = 0u32;
        for y in 0..32.min(texture.height as usize) {
            for x in 0..32.min(texture.width as usize) {
                let d = decoded[y * w + x];
                let t = truth[y * w + x];
                for c in 0..4 {
                    first_tile_mad += f64::from(d[c].abs_diff(t[c]));
                }
                n += 4;
            }
        }
        println!(
            "  first-32x32-texels MAD for {path}: {:.2}",
            first_tile_mad / f64::from(n)
        );
    }

    // Where does the SECOND on-disk 1024-byte *micro tile* (64 blocks, one
    // full 32x32-texel tile, using the already-validated intra-tile pixel
    // order) actually belong spatially? Decode it once and score it - full
    // 32x32 MAD, not one block - against every candidate tile slot in the
    // 128x64 image (tiles_x=4, tiles_y=2, 8 tiles total).
    if let Some((path, texture, blob, truth)) = pairs
        .iter()
        .find(|(p, ..)| p.contains("Holographic_02_GLOW") && p.contains("icaras"))
    {
        let decode_tile = |disk_tile_index: u64| -> [[u8; 4]; 32 * 32] {
            let tile_start = texture.data_offset + (disk_tile_index * 1024) as usize;
            let mut tile_texels = [[0u8; 4]; 32 * 32];
            for by in 0..8u32 {
                for bx in 0..8u32 {
                    let pixel_index = micro_tile_index(bx, by);
                    let element_offset = (pixel_index * 128 / 8) as usize;
                    let block: [u8; 16] = blob
                        [tile_start + element_offset..tile_start + element_offset + 16]
                        .try_into()
                        .unwrap();
                    let texels = bc7(&block);
                    for ty in 0..4usize {
                        for tx in 0..4usize {
                            let px = bx as usize * 4 + tx;
                            let py = by as usize * 4 + ty;
                            tile_texels[py * 32 + px] = texels[ty * 4 + tx];
                        }
                    }
                }
            }
            tile_texels
        };
        let mad_at_slot = |tile_texels: &[[u8; 4]; 32 * 32], slot_x: u32, slot_y: u32| -> f64 {
            let mut mad = 0.0;
            let mut n = 0u32;
            for ty in 0..32usize {
                for tx in 0..32usize {
                    let px = slot_x as usize * 32 + tx;
                    let py = slot_y as usize * 32 + ty;
                    if px < texture.width as usize && py < texture.height as usize {
                        let t = truth[py * texture.width as usize + px];
                        let d = tile_texels[ty * 32 + tx];
                        for c in 0..4 {
                            mad += f64::from(d[c].abs_diff(t[c]));
                        }
                        n += 4;
                    }
                }
            }
            mad / f64::from(n.max(1))
        };
        println!("for every on-disk micro tile (0..8), best-matching spatial slot in {path}:");
        for disk_index in 0..8u64 {
            let tile_texels = decode_tile(disk_index);
            let mut best = (f64::MAX, 0u32, 0u32);
            for slot_y in 0..2u32 {
                for slot_x in 0..4u32 {
                    let mad = mad_at_slot(&tile_texels, slot_x, slot_y);
                    if mad < best.0 {
                        best = (mad, slot_x, slot_y);
                    }
                }
            }
            println!(
                "  on-disk tile {disk_index} -> best slot ({}, {}), MAD {:.2}",
                best.1, best.2, best.0
            );
        }

        // Full matrix for row 1's four disk tiles (4..8) against row 1's
        // four spatial slots - is it a different permutation (serpentine,
        // transposed) rather than row 0's straightforward identity?
        println!("row-1 disk tiles (4..8) x row-1 slots, full MAD matrix:");
        for disk_index in 4..8u64 {
            let tile_texels = decode_tile(disk_index);
            let row: Vec<String> = (0..4u32)
                .map(|slot_x| format!("{:.1}", mad_at_slot(&tile_texels, slot_x, 1)))
                .collect();
            println!("  disk tile {disk_index}: [{}]", row.join(", "));
        }

        // word6 (min_lod_warning / mip stats / DCC flags) and the metadata
        // offset dword - not decoded by `Texture::parse` - dumped raw here
        // to check whether DCC metadata sits between tile rows.
        let at = 16 + 0x18;
        let word6 = u32::from_le_bytes(blob[at..at + 4].try_into().unwrap());
        let meta_offset = u32::from_le_bytes(blob[at + 4..at + 8].try_into().unwrap());
        println!("word6=0x{word6:08x} meta_offset=0x{meta_offset:08x}");
        // Try shifting the assumed per-row byte stride by small deltas, in
        // case a metadata/padding region of a few hundred bytes sits
        // between the first and second tile row.
        for extra in [0i64, 8, 16, 32, 64, 128, 256, 512] {
            for sign in [1i64, -1] {
                let shift = extra * sign;
                if shift == 0 && sign == -1 {
                    continue;
                }
                let tile_start = (texture.data_offset as i64 + 4 * 1024 + shift) as usize;
                if tile_start + 1024 > blob.len() {
                    continue;
                }
                let mut tile_texels = [[0u8; 4]; 32 * 32];
                for by in 0..8u32 {
                    for bx in 0..8u32 {
                        let pixel_index = micro_tile_index(bx, by);
                        let element_offset = (pixel_index * 128 / 8) as usize;
                        let block: [u8; 16] = blob
                            [tile_start + element_offset..tile_start + element_offset + 16]
                            .try_into()
                            .unwrap();
                        let texels = bc7(&block);
                        for ty in 0..4usize {
                            for tx in 0..4usize {
                                let px = bx as usize * 4 + tx;
                                let py = by as usize * 4 + ty;
                                tile_texels[py * 32 + px] = texels[ty * 4 + tx];
                            }
                        }
                    }
                }
                let mad = mad_at_slot(&tile_texels, 0, 1);
                if mad < 10.0 {
                    println!("  shift {shift}: slot (0,1) MAD {mad:.2} <-- candidate");
                }
            }
        }
    }

    // Same row-boundary check on the single-mip-level 1024x1024 image -
    // isolates the question from any mip-chain confound entirely.
    if let Some((_, texture, blob, truth)) = pairs.iter().find(|(p, ..)| p.contains("harimau_c1")) {
        let tiles_x = (texture.width / 4).div_ceil(8);
        let decode_tile = |disk_tile_index: u64| -> [[u8; 4]; 32 * 32] {
            let tile_start = texture.data_offset + (disk_tile_index * 1024) as usize;
            let mut tile_texels = [[0u8; 4]; 32 * 32];
            for by in 0..8u32 {
                for bx in 0..8u32 {
                    let pixel_index = micro_tile_index(bx, by);
                    let element_offset = (pixel_index * 128 / 8) as usize;
                    let block: [u8; 16] = blob
                        [tile_start + element_offset..tile_start + element_offset + 16]
                        .try_into()
                        .unwrap();
                    let texels = bc7(&block);
                    for ty in 0..4usize {
                        for tx in 0..4usize {
                            let px = bx as usize * 4 + tx;
                            let py = by as usize * 4 + ty;
                            tile_texels[py * 32 + px] = texels[ty * 4 + tx];
                        }
                    }
                }
            }
            tile_texels
        };
        let at = 16 + 0x18;
        let word6 = u32::from_le_bytes(blob[at..at + 4].try_into().unwrap());
        let meta_offset = u32::from_le_bytes(blob[at + 4..at + 8].try_into().unwrap());
        println!(
            "harimau_c1: word6=0x{word6:08x} meta_offset=0x{meta_offset:08x} stream_size={} data_offset={} base_array_slice={} last_array_slice={} is_pow2_pad={} base_mip={} last_mip={}",
            texture.stream_size,
            texture.data_offset,
            texture.base_array_slice,
            texture.last_array_slice,
            texture.is_pow2_pad,
            texture.base_mip_level,
            texture.last_mip_level
        );
        println!("harimau_c1 1024x1024 (single mip level, tiles_x={tiles_x}): scanning row 0");
        for disk_index in 0..tiles_x as u64 {
            let tile_texels = decode_tile(disk_index);
            let slot_x = (disk_index % u64::from(tiles_x)) as u32;
            let slot_y = (disk_index / u64::from(tiles_x)) as u32;
            let mut mad = 0.0;
            let mut n = 0u32;
            for ty in 0..32usize {
                for tx in 0..32usize {
                    let px = slot_x as usize * 32 + tx;
                    let py = slot_y as usize * 32 + ty;
                    if px < texture.width as usize && py < texture.height as usize {
                        let t = truth[py * texture.width as usize + px];
                        let d = tile_texels[ty * 32 + tx];
                        for c in 0..4 {
                            mad += f64::from(d[c].abs_diff(t[c]));
                        }
                        n += 4;
                    }
                }
            }
            println!(
                "  disk tile {disk_index} -> expected slot ({slot_x},{slot_y}), MAD {:.2}",
                mad / f64::from(n.max(1))
            );
        }
    }
}
