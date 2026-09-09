//! Scratch probe: across every material's diffuse `.gxt`, what format bytes
//! actually occur, and how many decode? Found while checking that the new
//! render-side texture binding (`vita_rcsmodel_render_e2e_check.rs`) works
//! end to end: every single-material model sampled hit the same
//! `Unsupported { format: 0x83010200 }` - `oag_texture::gxt`'s own docs
//! already name `0x83` (`PVRTII4BPP`) as undecoded. Is that the whole
//! picture, or does something decode?
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_rcsmodel_texture_format_census
//! ```

use std::collections::{BTreeMap, HashSet};

use oag_formats::rcsmodel::psp2;
use oag_texture::gxt;

const PACKAGES: [&str; 3] = [
    "data/extracted/vita/PCSF00007/base/PSP2/data.psarc",
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

fn main() -> anyhow::Result<()> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut by_format: BTreeMap<u32, usize> = BTreeMap::new();
    let mut decoded = 0usize;
    let mut checked = 0usize;
    let mut unresolved = 0usize;

    for package in PACKAGES {
        let Ok(mut archive) = oag_assets::psarc::Archive::open(package) else {
            continue;
        };
        let mut entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
            .cloned()
            .collect();
        entries.sort();
        for entry in entries {
            let Ok(blob) = archive.read_path(&entry) else {
                continue;
            };
            let Ok(model) = psp2::parse(&blob) else {
                continue;
            };
            for material in &model.materials {
                let Some(diffuse) = material.diffuse_texture() else {
                    continue;
                };
                if !seen.insert(diffuse.to_string()) {
                    continue;
                }
                checked += 1;
                let Ok(tex_blob) = archive.read_path(diffuse) else {
                    unresolved += 1;
                    continue;
                };
                let Ok(parsed) = gxt::Gxt::parse(&tex_blob) else {
                    unresolved += 1;
                    continue;
                };
                let Some(texture) = parsed.only() else {
                    unresolved += 1;
                    continue;
                };
                match texture.to_rgba(&tex_blob) {
                    Ok(_) => decoded += 1,
                    Err(gxt::Error::Unsupported { format }) => {
                        *by_format.entry(format).or_insert(0) += 1;
                    }
                    Err(_) => unresolved += 1,
                }
            }
        }
    }

    println!("{checked} distinct diffuse texture(s) named across every material, {decoded} decode");
    println!("{unresolved} do not resolve at all (missing entry / bad container)");
    println!("undecoded format bytes:");
    for (format, count) in &by_format {
        println!("  {format:#010x}: {count}");
    }

    Ok(())
}
