//! Scratch probe: before teaching [`oag_texture::gxt::Format`] `UBC1`/`UBC3`,
//! does the same "one block per 4x4, tightly packed mip chain, floored at
//! [`oag_texture::gxt::MIN_LEVEL_LEN`] bytes" arithmetic `UBC2`/`PVRTII4BPP`
//! already close on also close for these two? `MIN_LEVEL_LEN` was only ever
//! exercised against `UBC2` (16-byte blocks, so the 16-byte floor is free) and
//! `PVRTII4BPP` (8-byte words, needs the floor). `UBC1` is an 8-byte block -
//! the same size PVRTC's word is - so it is the first *block* format this
//! floor has to prove itself against; `UBC3` is 16 bytes, same size as
//! `UBC2`'s block, so the floor should again be free there.
//!
//! Also histograms descriptor `type` (`+0x10`) over the two formats, since
//! every texture measured elsewhere in this corpus carries `0` there and a
//! non-zero value on either would be the first one seen.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_gxt_ubc13_extent
//! ```

use std::collections::BTreeMap;

use oag_texture::gxt;

const PACKAGES: [&str; 3] = [
    "data/extracted/vita/PCSF00007/base/PSP2/data.psarc",
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

fn plain(width: u32, height: u32, unit_len: usize) -> usize {
    (width as usize).div_ceil(4) * (height as usize).div_ceil(4) * unit_len
}

fn floored(width: u32, height: u32, unit_len: usize) -> usize {
    plain(width, height, unit_len).max(16)
}

struct Report {
    label: &'static str,
    textures: usize,
    plain_ok: usize,
    floored_ok: usize,
    sizes: BTreeMap<(u16, u16, u8), usize>,
    mismatch: Vec<String>,
}

fn sweep(format_byte: u8, unit_len: usize, label: &'static str) -> Report {
    let mut report = Report {
        label,
        textures: 0,
        plain_ok: 0,
        floored_ok: 0,
        sizes: BTreeMap::new(),
        mismatch: Vec::new(),
    };

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
                if (texture.format_byte >> 24) as u8 != format_byte {
                    continue;
                }
                report.textures += 1;
                *report
                    .sizes
                    .entry((texture.width, texture.height, texture.mip_levels))
                    .or_default() += 1;
                let declared = texture.data.end - texture.data.start;
                let (mut a, mut b) = (0usize, 0usize);
                for level in 0..texture.mip_levels {
                    let (w, h) = texture.level_size(level);
                    a += plain(w, h, unit_len);
                    b += floored(w, h, unit_len);
                }
                report.plain_ok += usize::from(a == declared);
                report.floored_ok += usize::from(b == declared);
                if b != declared && report.mismatch.len() < 10 {
                    report.mismatch.push(format!(
                        "{entry}: {}x{} mips {} declared {declared}, plain {a}, floored {b}",
                        texture.width, texture.height, texture.mip_levels
                    ));
                }
            }
        }
    }

    report
}

fn type_field_histogram(format_byte: u8) -> BTreeMap<u32, usize> {
    let mut types = BTreeMap::new();
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
            if blob.len() < gxt::HEADER_LEN {
                continue;
            }
            let count = u32::from_le_bytes(blob[8..12].try_into().unwrap());
            for index in 0..count as usize {
                let at = gxt::HEADER_LEN + index * gxt::DESCRIPTOR_LEN;
                if at + gxt::DESCRIPTOR_LEN > blob.len() {
                    continue;
                }
                let fmt = u32::from_le_bytes(blob[at + 0x14..at + 0x18].try_into().unwrap());
                if (fmt >> 24) as u8 != format_byte {
                    continue;
                }
                let ty = u32::from_le_bytes(blob[at + 0x10..at + 0x14].try_into().unwrap());
                *types.entry(ty).or_default() += 1;
            }
        }
    }
    types
}

fn print_report(report: &Report) {
    println!(
        "{} ({} textures): plain closes {}, floored (16B) closes {}",
        report.label, report.textures, report.plain_ok, report.floored_ok
    );
    let mut by_count: Vec<_> = report.sizes.iter().collect();
    by_count.sort_by_key(|&(_, n)| std::cmp::Reverse(*n));
    println!("  distinct (w, h, mips), most common first:");
    for ((w, h, mips), n) in by_count.iter().take(10) {
        println!("    {w}x{h} mips {mips}: {n}");
    }
    for line in &report.mismatch {
        println!("  MISMATCH {line}");
    }
}

fn main() -> anyhow::Result<()> {
    let ubc1 = sweep(0x85, 8, "UBC1 (BC1, 8B/block)");
    print_report(&ubc1);
    println!("  type field histogram: {:?}", type_field_histogram(0x85));
    println!();

    let ubc3 = sweep(0x87, 16, "UBC3 (BC3, 16B/block)");
    print_report(&ubc3);
    println!("  type field histogram: {:?}", type_field_histogram(0x87));

    Ok(())
}
