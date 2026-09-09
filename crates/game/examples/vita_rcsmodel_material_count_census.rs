//! Scratch probe: how many of the corpus's `.rcsmodel` files carry exactly
//! one material (submesh binding is then unambiguous even with the index
//! field unresolved) versus several (binding still needed)?
//!
//! Reads the file-level header `vita_rcsmodel_header_probe.rs` found:
//! `+0x44` material count, `+0x48` pointer to the offset table.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_rcsmodel_material_count_census
//! ```

use std::collections::BTreeMap;

use oag_rcs::rcsmodel::psp2;

const PACKAGES: [&str; 3] = [
    "data/extracted/vita/PCSF00007/base/PSP2/data.psarc",
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

fn main() -> anyhow::Result<()> {
    let mut by_count: BTreeMap<u32, usize> = BTreeMap::new();
    let mut files = 0usize;
    let mut bad_table_ptr = 0usize;

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
            let Some(&gpu) = model.sections.get(1) else {
                continue;
            };
            let _ = gpu;
            let cpu = model.sections[0];
            let bytes = &blob[cpu.at..cpu.at + cpu.len];
            let Some(count_bytes) = bytes.get(0x44..0x48) else {
                continue;
            };
            let count = u32::from_le_bytes(count_bytes.try_into().unwrap());
            let Some(table_bytes) = bytes.get(0x48..0x4c) else {
                continue;
            };
            let table = u32::from_le_bytes(table_bytes.try_into().unwrap()) as usize;
            if count > 8192 || table >= bytes.len() {
                bad_table_ptr += 1;
                continue;
            }
            files += 1;
            *by_count.entry(count).or_insert(0) += 1;
        }
    }

    println!("{files} files with a plausible material-count header, {bad_table_ptr} implausible");
    let single: usize = by_count.get(&1).copied().unwrap_or(0);
    let zero: usize = by_count.get(&0).copied().unwrap_or(0);
    println!("material count distribution:");
    for (count, n) in &by_count {
        println!("  {count:3} material(s): {n} file(s)");
    }
    println!(
        "single-material files: {single} ({:.1}%), zero-material: {zero}",
        100.0 * single as f64 / files.max(1) as f64
    );

    Ok(())
}
