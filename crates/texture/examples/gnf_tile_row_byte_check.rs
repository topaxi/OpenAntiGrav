//! Scratch diagnostic (not kept): is the "odd tile row near-black" pattern
//! `docs/formats/gnf.md`'s Tiling section reports a genuinely tiled BC7
//! payload, or is it the same unexplained PSARC "garbage" population that
//! page's own "Block data location" section already measures on a fraction
//! of this archive family's large multi-block entries?
//!
//! Checks, per PSARC block covering `Harimau_c1_Livery.gnf`'s pixel data:
//! whether the block's first bytes decode as a BC7 block with a valid mode
//! (any of the eight leading-zero-then-one patterns in byte 0..2), or as
//! all-zero / no-valid-mode - which a BC7 encoder never emits - and prints
//! the archive's own block table entries (offset window, size, first_block)
//! for that entry so a periodic corruption can be matched against PSARC
//! block boundaries directly instead of guessed at.

use oag_assets::psarc::Archive;
use oag_texture::gnf::{SurfaceFormat, Texture};

fn has_valid_bc7_mode(byte0: u8) -> bool {
    // Mode is unary: N zero bits then a one bit, N in 0..=7 (mode 0..=7).
    // byte0 == 0x00 means no 1-bit in the first 8 bits - not a legal mode 0-7
    // on its own (mode 7 needs bit 7 set to 1). A real encoder never emits
    // an all-zero mode field.
    byte0 != 0x00
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/extracted/ps4/omega-eu/uroot/data03.psarc".to_string());
    let mut archive = Archive::open_file(std::path::Path::new(&archive_path))?;

    let mut candidates: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| {
            let lower = p.to_ascii_lowercase();
            lower.contains("harimau_c1") && lower.ends_with(".gnf")
        })
        .cloned()
        .collect();
    candidates.sort();
    println!("candidates: {candidates:#?}");
    let target = std::env::args()
        .nth(2)
        .unwrap_or_else(|| candidates.first().cloned().unwrap_or_default());
    if target.is_empty() {
        return Err("no harimau_c1 .gnf entry found in this archive".into());
    }
    println!("entry: {target}");

    let index = archive.index_of_path(&target).unwrap();
    let directory = archive.directory();
    let entry = directory.entries[index];
    let block_size = directory.header.block_size as u64;
    let blocks = directory.entry_blocks(index)?;
    println!(
        "entry.offset={} entry.size={} first_block={} block_size={} blocks_spanned={}",
        entry.offset,
        entry.size,
        entry.first_block,
        block_size,
        blocks.len()
    );
    for (n, b) in blocks.iter().enumerate() {
        let stored = if *b == 0 { block_size } else { u64::from(*b) };
        println!(
            "  block[{n}] (table index {}): stored_len={stored}",
            entry.first_block as usize + n
        );
    }

    let blob = archive.read_path(&target)?;
    let texture = Texture::parse(&blob)?;
    println!(
        "parsed: {}x{} format={:?} tile_mode={} data_offset={} stream_size={}",
        texture.width,
        texture.height,
        texture.surface_format,
        texture.tile_mode.0,
        texture.data_offset,
        texture.stream_size
    );
    assert_eq!(texture.surface_format, SurfaceFormat::Bc7);

    let width_blocks = texture.width.div_ceil(4);
    let height_blocks = texture.height.div_ceil(4);
    let tiles_x = width_blocks.div_ceil(8);
    let tiles_y = height_blocks.div_ceil(8);
    println!("blocks {width_blocks}x{height_blocks}, tiles {tiles_x}x{tiles_y}, tile_bytes=1024");

    // Per on-disk micro-tile (row-major over the whole surface, matching
    // decode_micro_tiled's own tile_index formula), count of the 64 BC7
    // blocks inside it whose byte[0] has no valid mode.
    for tile_row in 0..tiles_y {
        let mut zero_blocks_in_row = 0u32;
        let mut total_blocks_in_row = 0u32;
        let mut first_zero_at: Option<u64> = None;
        for tile_col in 0..tiles_x {
            let tile_index = u64::from(tile_row * tiles_x + tile_col);
            let tile_start = texture.data_offset + (tile_index * 1024) as usize;
            for i in 0..64usize {
                let start = tile_start + i * 16;
                total_blocks_in_row += 1;
                let Some(b0) = blob.get(start).copied() else {
                    continue;
                };
                if !has_valid_bc7_mode(b0) {
                    zero_blocks_in_row += 1;
                    if first_zero_at.is_none() {
                        first_zero_at = Some(start as u64);
                    }
                }
            }
        }
        let file_offset_of_row_start =
            texture.data_offset as u64 + u64::from(tile_row * tiles_x) * 1024;
        let block_index_of_row_start =
            entry.first_block as u64 + (file_offset_of_row_start / block_size);
        println!(
            "tile_row {tile_row}: zero/no-mode blocks {zero_blocks_in_row}/{total_blocks_in_row}, row starts at file offset {file_offset_of_row_start} (psarc block index ~{block_index_of_row_start}), first bad block file offset {:?}",
            first_zero_at
        );
    }

    Ok(())
}
