//! Scratch probe: the last undecoded uncompressed format byte, `0x98`
//! (`U8U8U8`, 13 files per `docs/formats/gxt.md`). Does the declared texel
//! length close on 3 bytes a texel (tightly packed) or 4 (padded, the same
//! stride `U8U8U8U8` already uses)?
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_gxt_u8u8u8_extent
//! ```

const PACKAGES: [&str; 3] = [
    "data/extracted/vita/PCSF00007/base/PSP2/data.psarc",
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

fn main() {
    let mut n = 0usize;
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
            if blob.len() < 12 {
                continue;
            }
            let count = u32::from_le_bytes(blob[8..12].try_into().unwrap());
            for index in 0..count as usize {
                let at = 32 + index * 32;
                if at + 32 > blob.len() {
                    continue;
                }
                let fmt = u32::from_le_bytes(blob[at + 0x14..at + 0x18].try_into().unwrap());
                if (fmt >> 24) as u8 != 0x98 {
                    continue;
                }
                let width = u16::from_le_bytes([blob[at + 0x18], blob[at + 0x19]]);
                let height = u16::from_le_bytes([blob[at + 0x1a], blob[at + 0x1b]]);
                let mips = blob[at + 0x1c];
                let offset = u32::from_le_bytes(blob[at..at + 4].try_into().unwrap());
                let length = u32::from_le_bytes(blob[at + 4..at + 8].try_into().unwrap());
                let ty = u32::from_le_bytes(blob[at + 0x10..at + 0x14].try_into().unwrap());
                let mut plain3 = 0usize;
                let mut plain4 = 0usize;
                for level in 0..mips {
                    let w = (u32::from(width) >> level).max(1) as usize;
                    let h = (u32::from(height) >> level).max(1) as usize;
                    plain3 += w * h * 3;
                    plain4 += w * h * 4;
                }
                n += 1;
                println!(
                    "{entry}: {width}x{height} mips={mips} type=0x{ty:08x} fmt=0x{fmt:08x} \
                     offset={offset} declared={length} plain3={plain3} plain4={plain4} \
                     ({package})",
                );
            }
        }
    }
    println!("{n} U8U8U8 (0x98) textures found");
}
