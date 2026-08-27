//! Scratch probe: 2048's DLC re-ships Wipeout HD/Fury's own circuits and its
//! whole fourteen-team roster. Where the *same art* exists as a `.gtf` on the
//! PS3 disc and a `.gxt` in the Vita package, HD's already-trusted decode is
//! ground truth for this crate's new `PVRTII4BPP` one - the same
//! HD-as-oracle method that settled the `WO Track` tail, the vertex normal and
//! `Uv1` (see `docs/formats/2048-rcsmodel.md`).
//!
//! This is the discovery half: which basenames exist on both sides, at what
//! dimensions, and in which formats.
//!
//! ```sh
//! cargo run -q --release -p oag-game --example vita_gxt_hd_texture_pairs
//! ```

use std::collections::BTreeMap;

const VITA: [&str; 3] = [
    "data/extracted/vita/PCSF00007/base/PSP2/data.psarc",
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

fn basename(path: &str) -> String {
    let lower = path.to_ascii_lowercase().replace('\\', "/");
    let file = lower.rsplit('/').next().unwrap_or(&lower).to_string();
    file.rsplit_once('.')
        .map_or(file.clone(), |(s, _)| s.to_string())
}

fn main() -> anyhow::Result<()> {
    // Every `.gtf` Wipeout HD ships, by basename, with its own dimensions.
    let mut hd: BTreeMap<String, (String, String, u32, u32)> = BTreeMap::new();
    for index in 0..7 {
        let path = format!("data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/DATA0{index}.PSARC");
        let Ok(mut archive) = oag_assets::psarc::Archive::open(&path) else {
            continue;
        };
        let entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".gtf"))
            .cloned()
            .collect();
        for entry in entries {
            let Ok(blob) = archive.read_path(&entry) else {
                continue;
            };
            let Ok(parsed) = oag_formats::gtf::Gtf::parse(&blob) else {
                continue;
            };
            let Some(texture) = parsed.textures.first() else {
                continue;
            };
            hd.insert(
                basename(&entry),
                (
                    path.clone(),
                    entry.clone(),
                    u32::from(texture.width),
                    u32::from(texture.height),
                ),
            );
        }
    }
    println!("{} distinct HD .gtf basenames", hd.len());

    let mut pairs = 0usize;
    let mut same_size = 0usize;
    for package in VITA {
        let Ok(mut archive) = oag_assets::psarc::Archive::open(package) else {
            continue;
        };
        let entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".gxt"))
            .cloned()
            .collect();
        for entry in entries {
            let name = basename(&entry);
            let Some((hd_pkg, hd_entry, hw, hh)) = hd.get(&name) else {
                continue;
            };
            let Ok(blob) = archive.read_path(&entry) else {
                continue;
            };
            let Ok(parsed) = oag_formats::gxt::Gxt::parse(&blob) else {
                continue;
            };
            let Some(texture) = parsed.only() else {
                continue;
            };
            pairs += 1;
            if u32::from(texture.width) == *hw && u32::from(texture.height) == *hh {
                same_size += 1;
                if same_size <= 25 {
                    println!(
                        "PAIR {name}: {}x{} fmt {:02x} | vita {entry} | hd {hd_pkg}:{hd_entry}",
                        texture.width,
                        texture.height,
                        texture.format_byte >> 24,
                    );
                }
            }
        }
    }
    println!("{pairs} name pairs, {same_size} of them the same dimensions");
    Ok(())
}
