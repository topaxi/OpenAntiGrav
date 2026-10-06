//! The micro-tile (`Thin_1DThin`, `TileMode(13)`) address formula probe: an exact
//! match on an isolated single-tile oracle pair and many consecutive tiles on
//! larger ones. The corruption past that point was later explained as
//! missing/garbage BC7 blocks, not a tiling bug (the exploratory hypotheses
//! below predate that; `docs/formats/gnf.md`'s "Tiling" section has the
//! evidence). `#[ignore]`d, needs real game data, in-crate because it needs
//! `pub(crate)` `crate::bcn::bc7`.

use crate::bcn::bc7;
use crate::gnf::Texture;

use super::oracle_tests::{collect_oracle_pairs, mean_abs_diff, micro_tile_index};

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

/// Micro-tile traversal order across the surface: the one axis assumed
/// (row-major) without measuring; the bit-level formulas above are direct ports.
#[derive(Clone, Copy, Debug)]
enum TileOrder {
    RowMajor,
    ColumnMajor,
    Morton,
}

/// Interleaves the low bits of `x` and `y` (x odd, y even), the Z-order of
/// `oag_texture::gxt::twiddle`; a candidate arrangement of micro *tiles* across a
/// `1D_TILED_THIN1` surface.
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

/// Tests that `TileMode(13)` is `Thin_1DThin` (micro-tiled only; GFD-Studio's
/// `TileMode.cs` has 13 `Thin_1DThin` and **14** `Thin_2DThin`, and every real
/// `.gnf` declares 13) against the bounded search's oracle pairs, with a much
/// simpler address formula and no bank/pipe/row-size unknowns.
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
                        // The single-tile sample tests only intra-tile bit order;
                        // skip it to keep the multi-tile rows visible.
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

    // Is the FIRST micro tile of a multi-tile image byte-exact (corruption starts
    // at tile 1) or already wrong (pointing at data_offset/mip-chain layout)?
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

    // Where does the SECOND on-disk 1024-byte *micro tile* (one 32x32-texel tile)
    // belong? Score its 32x32 MAD against every candidate slot in the 128x64
    // image (tiles_x=4, tiles_y=2).
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

        // Hypothesis: mip levels interleave by TILE-ROW, putting level 0's second
        // tile row after every level's row 0: 4 + 2 + 6 = 12 tiles = 12288 bytes.
        for disk_index in [12u64, 13, 14, 15] {
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
                "  mip-row-interleave hypothesis: disk tile {disk_index} -> best slot ({}, {}), MAD {:.2}",
                best.1, best.2, best.0
            );
        }

        println!("row-1 disk tiles (4..8) x row-1 slots, full MAD matrix:");
        for disk_index in 4..8u64 {
            let tile_texels = decode_tile(disk_index);
            let row: Vec<String> = (0..4u32)
                .map(|slot_x| format!("{:.1}", mad_at_slot(&tile_texels, slot_x, 1)))
                .collect();
            println!("  disk tile {disk_index}: [{}]", row.join(", "));
        }

        // word6 and the metadata offset dword, not decoded by `Texture::parse`,
        // dumped raw to check for DCC metadata between tile rows.
        let at = 16 + 0x18;
        let word6 = u32::from_le_bytes(blob[at..at + 4].try_into().unwrap());
        let meta_offset = u32::from_le_bytes(blob[at + 4..at + 8].try_into().unwrap());
        println!("word6=0x{word6:08x} meta_offset=0x{meta_offset:08x}");
        // Shift the per-row byte stride by small deltas in case metadata/padding
        // sits between the first and second tile row.
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

    // harimau_c1: write decoded and oracle PNGs to tell tiling noise (stripes,
    // blocks) from a content difference (a spatially coherent blob).
    if let Some((_, texture, blob, truth)) = pairs.iter().find(|(p, ..)| p.contains("harimau_c1")) {
        let decoded =
            decode_micro_tiled(blob, texture, micro_tile_index, TileOrder::RowMajor, false);
        let decoded_bytes: Vec<u8> = decoded.iter().flatten().copied().collect();
        let truth_bytes: Vec<u8> = truth.iter().flatten().copied().collect();
        std::fs::write(
            "/tmp/gnf_harimau_decoded.png",
            crate::png::encode_rgba(texture.width, texture.height, &decoded_bytes),
        )
        .unwrap();
        std::fs::write(
            "/tmp/gnf_harimau_truth.png",
            crate::png::encode_rgba(texture.width, texture.height, &truth_bytes),
        )
        .unwrap();
        println!("wrote /tmp/gnf_harimau_decoded.png and _truth.png");

        // Hypothesis: the tile-row stride is HALF of pitch_blocks/8 (16, not 32).
        {
            let width_blocks = texture.width.div_ceil(4);
            let height_blocks = texture.height.div_ceil(4);
            let tiles_x_half = width_blocks.div_ceil(8) / 2;
            let mut out = vec![[0u8; 4]; (texture.width * texture.height) as usize];
            for by in 0..height_blocks {
                for bx in 0..width_blocks {
                    let tile_x = bx / 8;
                    let tile_y = by / 8;
                    let tile_index = u64::from(tile_y * tiles_x_half + tile_x);
                    let pixel_index = micro_tile_index(bx % 8, by % 8);
                    let element_offset = pixel_index * 128 / 8;
                    let addr = tile_index * 1024 + u64::from(element_offset);
                    let start = texture.data_offset + addr as usize;
                    if let Some(block) = blob.get(start..start + 16).and_then(|s| s.try_into().ok())
                    {
                        let texels: [[u8; 4]; 16] = bc7(&block);
                        for ty in 0..4usize {
                            for tx in 0..4usize {
                                let px = (bx * 4) as usize + tx;
                                let py = (by * 4) as usize + ty;
                                if px < texture.width as usize && py < texture.height as usize {
                                    out[py * texture.width as usize + px] = texels[ty * 4 + tx];
                                }
                            }
                        }
                    }
                }
            }
            let mad = mean_abs_diff(&out, truth);
            println!("half-stride hypothesis (tiles_x={tiles_x_half}): whole-image MAD {mad:.2}");
        }

        // Hypothesis: even and odd tile-rows are deinterlaced (even rows first,
        // odd from total_tiles/2).
        {
            let width_blocks = texture.width.div_ceil(4);
            let height_blocks = texture.height.div_ceil(4);
            let tiles_x = width_blocks.div_ceil(8);
            let tiles_y = height_blocks.div_ceil(8);
            let half = tiles_x * (tiles_y / 2);
            let mut out = vec![[0u8; 4]; (texture.width * texture.height) as usize];
            for by in 0..height_blocks {
                for bx in 0..width_blocks {
                    let tile_x = bx / 8;
                    let tile_y = by / 8;
                    let tile_index: u64 = if tile_y % 2 == 0 {
                        u64::from((tile_y / 2) * tiles_x + tile_x)
                    } else {
                        u64::from(half) + u64::from((tile_y / 2) * tiles_x + tile_x)
                    };
                    let pixel_index = micro_tile_index(bx % 8, by % 8);
                    let element_offset = pixel_index * 128 / 8;
                    let addr = tile_index * 1024 + u64::from(element_offset);
                    let start = texture.data_offset + addr as usize;
                    if let Some(block) = blob.get(start..start + 16).and_then(|s| s.try_into().ok())
                    {
                        let texels: [[u8; 4]; 16] = bc7(&block);
                        for ty in 0..4usize {
                            for tx in 0..4usize {
                                let px = (bx * 4) as usize + tx;
                                let py = (by * 4) as usize + ty;
                                if px < texture.width as usize && py < texture.height as usize {
                                    out[py * texture.width as usize + px] = texels[ty * 4 + tx];
                                }
                            }
                        }
                    }
                }
            }
            let mad = mean_abs_diff(&out, truth);
            println!("deinterlace hypothesis: whole-image MAD {mad:.2}");
        }

        // Per-tile-row mean brightness, for the periodicity of the banding.
        for tile_row in 0..32usize {
            let y0 = tile_row * 32;
            let mut sum = 0u64;
            for y in y0..(y0 + 32).min(texture.height as usize) {
                for x in 0..texture.width as usize {
                    let p = decoded[y * texture.width as usize + x];
                    sum += u64::from(p[0]) + u64::from(p[1]) + u64::from(p[2]);
                }
            }
            let mean = sum as f64 / (32 * texture.width as usize * 3) as f64;
            println!("  tile row {tile_row}: mean brightness {mean:.1}");
        }
    }

    // Same row-boundary check on the single-mip 1024x1024 image, free of any
    // mip-chain confound.
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
