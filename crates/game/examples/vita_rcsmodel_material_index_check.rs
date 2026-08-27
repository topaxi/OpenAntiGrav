//! Scratch probe: the follow-up to `vita_rcsmodel_material_index_probe`, which
//! found exactly one offset that behaves like a material index -
//! `record - 0x18`, read as a little-endian `u16`.
//!
//! That probe's discriminator was *coverage*: the mean number of distinct
//! values a multi-material model uses. Every other surviving offset in the
//! window averaged 3.2 or fewer; this one averages 80.3 and reaches every
//! entry of at least one model's whole table. This probe is the part that says
//! what the field actually resolves to, on the two files a race loads, and
//! whether the resolved material makes sense next to the submesh's own
//! authored name.
//!
//! ```sh
//! cargo run -q --release -p oag-game --example vita_rcsmodel_material_index_check
//! ```

use std::collections::BTreeMap;

use oag_formats::rcsmodel::psp2;

const PACKAGE: &str = "data/extracted/vita/PCSF00007/base/PSP2/data.psarc";

const FILES: [&str; 3] = [
    "data/art/published/Ships/feisar2048/3/ship.rcsmodel",
    "data/art/published/environments/altima/track.rcsmodel",
    "data/Weapons/bomb_shockwave.rcsmodel",
];

fn u16_at(file: &[u8], at: usize) -> Option<u16> {
    file.get(at..at + 2)
        .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
}

fn u32_at(file: &[u8], at: usize) -> Option<u32> {
    file.get(at..at + 4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
}

fn main() -> anyhow::Result<()> {
    let mut archive = oag_assets::psarc::Archive::open(PACKAGE)?;
    for path in FILES {
        let Ok(blob) = archive.read_path(path) else {
            println!("{path}: not in the archive");
            continue;
        };
        let model = psp2::parse(&blob)?;
        let Some(cpu) = model.sections.first() else {
            continue;
        };
        println!(
            "\n=== {path}: {} submesh(es), {} material(s)",
            model.submeshes.len(),
            model.materials.len()
        );

        let mut histogram: BTreeMap<u16, usize> = BTreeMap::new();
        for (n, submesh) in model.submeshes.iter().enumerate() {
            let at = cpu.at + submesh.record;
            let index = u16_at(&blob, at - 0x18).unwrap_or(u16::MAX);
            let high = u16_at(&blob, at - 0x16).unwrap_or(u16::MAX);
            *histogram.entry(index).or_default() += 1;
            if n < 24 {
                let material = model.materials.get(usize::from(index));
                println!(
                    "  submesh {n:3}: index {index:4} (high half {high}) -> {} | {}",
                    material.map_or("<out of range>", |m| m.name.as_str()),
                    material
                        .and_then(|m| m.diffuse_texture())
                        .unwrap_or("<no texture>"),
                );
            }
        }
        println!(
            "  {} distinct index values over {} submeshes, {} materials declared",
            histogram.len(),
            model.submeshes.len(),
            model.materials.len()
        );
        let unused = (0..model.materials.len())
            .filter(|i| !histogram.contains_key(&(*i as u16)))
            .count();
        println!("  {unused} declared material(s) named by no submesh");

        // What the sixteen bytes either side of the record's own start hold,
        // so the field can be placed in whatever the enclosing struct is
        // rather than only as an offset.
        if let Some(first) = model.submeshes.first() {
            let at = cpu.at + first.record;
            print!("  words at record-0x20..record+0x10:");
            for k in 0..12 {
                let word = u32_at(&blob, at - 0x20 + k * 4).unwrap_or(0);
                print!(" {}{word:#x}", if k == 8 { "|" } else { "" });
            }
            println!();
        }
    }
    Ok(())
}
