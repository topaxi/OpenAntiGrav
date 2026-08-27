//! Scratch probe: does a `PVRTII4BPP` texture's declared texel length close on
//! the same "one 8-byte word per 4x4 block, tightly packed mip chain"
//! arithmetic `oag_formats::gxt` already checks for `UBC2`?
//!
//! This is the question that has to be answered *before* teaching
//! [`oag_formats::gxt::Format`] the format byte: the moment it knows the block
//! size, `Gxt::parse` starts length-checking all 8,430 of them, and a formula
//! that is off by one clamp rejects the corpus rather than decoding it.
//!
//! Two candidates are measured against every shipped file:
//!
//! - `plain`: `ceil(w/4) * ceil(h/4) * 8` per level, which is what Vita3K's own
//!   `decompress_compressed_swizzle_texture` returns as the bytes a
//!   `PVRTII4BPP` surface consumes.
//! - `clamped`: `max(w, 8) * max(h, 8) / 2` per level, the PVRTC minimum-surface
//!   rule its reference decompressor's `XTrueDim`/`YTrueDim` clamp implies.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_gxt_pvrtc_extent
//! ```

use std::collections::BTreeMap;

use oag_formats::gxt;

const PACKAGES: [&str; 3] = [
    "data/extracted/vita/PCSF00007/base/PSP2/data.psarc",
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

fn plain(width: u32, height: u32) -> usize {
    (width as usize).div_ceil(4) * (height as usize).div_ceil(4) * 8
}

fn clamped(width: u32, height: u32) -> usize {
    plain(width, height).max(16)
}

fn main() -> anyhow::Result<()> {
    let mut textures = 0usize;
    let mut plain_ok = 0usize;
    let mut clamped_ok = 0usize;
    let mut sizes: BTreeMap<(u16, u16, u8), usize> = BTreeMap::new();
    let mut mismatch: Vec<String> = Vec::new();
    let mut deltas: BTreeMap<(i64, u32, u32), usize> = BTreeMap::new();
    let mut non_po2 = 0usize;
    let mut small_base = 0usize;

    for package in PACKAGES {
        let Ok(mut archive) = oag_assets::psarc::Archive::open(package) else {
            continue;
        };
        let mut entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".gxt"))
            .cloned()
            .collect();
        entries.sort();
        for entry in entries {
            let Ok(blob) = archive.read_path(&entry) else {
                continue;
            };
            let Ok(parsed) = gxt::Gxt::parse(&blob) else {
                continue;
            };
            for texture in &parsed.textures {
                if texture.format_byte >> 24 != 0x83 {
                    continue;
                }
                textures += 1;
                *sizes
                    .entry((texture.width, texture.height, texture.mip_levels))
                    .or_default() += 1;
                let declared = texture.data.end - texture.data.start;
                let mut a = 0usize;
                let mut b = 0usize;
                for level in 0..texture.mip_levels {
                    let (w, h) = texture.level_size(level);
                    a += plain(w, h);
                    b += clamped(w, h);
                }
                plain_ok += usize::from(a == declared);
                clamped_ok += usize::from(b == declared);
                if !texture.width.is_power_of_two() || !texture.height.is_power_of_two() {
                    non_po2 += 1;
                }
                if texture.width < 8 || texture.height < 8 {
                    small_base += 1;
                }
                let (lw, lh) = texture.level_size(texture.mip_levels - 1);
                *deltas
                    .entry((declared as i64 - a as i64, lw, lh))
                    .or_default() += 1;
                if a != declared && mismatch.len() < 20 {
                    mismatch.push(format!(
                        "{entry}: {}x{} mips {} declared {declared}, plain {a}, clamped {b}",
                        texture.width, texture.height, texture.mip_levels
                    ));
                }
            }
        }
    }

    println!("{textures} PVRTII4BPP textures");
    println!("  plain   (ceil/4 * 8) closes on {plain_ok}");
    println!("  clamped (min 16 B)   closes on {clamped_ok}");
    println!("distinct (w, h, mips), most common first:");
    let mut by_count: Vec<_> = sizes.into_iter().collect();
    by_count.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    for ((w, h, mips), n) in by_count.iter().take(20) {
        println!("  {w}x{h} mips {mips}: {n}");
    }
    println!("smallest levels seen: {} distinct sizes", by_count.len());
    println!("non-power-of-two base levels: {non_po2}");
    println!("base levels under 8 in a dimension: {small_base}");
    println!("(declared - plain, last level w, h) -> count:");
    for ((d, w, h), n) in &deltas {
        println!("  {d:+} last {w}x{h}: {n}");
    }
    for line in &mismatch {
        println!("MISMATCH {line}");
    }
    Ok(())
}
