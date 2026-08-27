//! Scratch probe: `.gxt`/`.rcsmaterial` ASCII is embedded in section B
//! (`vita_rcsmodel_texture_probe.rs` found 52,637 and 34,388 occurrences
//! corpus-wide). This locates the **pointer field** that reaches one of
//! those strings from a submesh record, the same "pair the pointer to the
//! thing it points at" method that found the buffer pointers themselves.
//!
//! For one track and one craft: find every NUL-terminated ASCII string in
//! the CPU section containing `.gxt` or `.rcsmaterial`, then scan the whole
//! CPU section for 4-byte little-endian words equal to that string's own
//! offset - a real intra-section pointer, the same shape
//! `psp2::submeshes`'s buffer pointers already are. Report each hit's
//! position relative to the nearest submesh record.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_rcsmodel_material_probe
//! ```

use std::collections::BTreeMap;

use oag_formats::rcsmodel::psp2;

const TRACK: &str = "data/extracted/vita/PCSF00007/base/PSP2/data.psarc";
const TRACK_PATH: &str = "Data/art/published/environments/altima/track.rcsmodel";
const SHIP_PATH: &str = "Data/art/published/Ships/feisar2048/3/ship.rcsmodel";

fn find_strings(bytes: &[u8], needle: &str) -> Vec<(usize, usize, String)> {
    let needle_b = needle.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + needle_b.len() <= bytes.len() {
        if &bytes[i..i + needle_b.len()] == needle_b {
            // Walk backward to the start of this printable run.
            let mut start = i;
            while start > 0 && bytes[start - 1] >= 0x20 && bytes[start - 1] < 0x7f {
                start -= 1;
            }
            // Walk forward to the terminator.
            let mut end = i + needle_b.len();
            while end < bytes.len() && bytes[end] >= 0x20 && bytes[end] < 0x7f {
                end += 1;
            }
            let s = String::from_utf8_lossy(&bytes[start..end]).to_string();
            out.push((start, end, s));
            i = end;
        } else {
            i += 1;
        }
    }
    out
}

fn probe(label: &str, blob: &[u8]) -> anyhow::Result<()> {
    println!("=== {label} ===");
    let model = psp2::parse(blob)?;
    let Some(_gpu) = model.sections.get(1) else {
        println!("  no GPU section");
        return Ok(());
    };
    let cpu = model.sections[0];
    let bytes = &blob[cpu.at..cpu.at + cpu.len];

    let mut records: Vec<usize> = model.submeshes.iter().map(|s| s.record).collect();
    records.sort_unstable();

    let mut strings: Vec<(usize, usize, String)> = Vec::new();
    for needle in [".gxt", ".rcsmaterial"] {
        strings.extend(find_strings(bytes, needle));
    }
    strings.sort_by_key(|(start, ..)| *start);
    println!(
        "  {} submeshes, {} strings found",
        records.len(),
        strings.len()
    );
    for (start, _end, s) in strings.iter().take(10) {
        println!("    string @{start:#x}: {s}");
    }

    // For each string, find every 4-byte LE word in the whole CPU section
    // equal to its own offset - a real pointer to it.
    let mut offset_from_record: BTreeMap<i64, usize> = BTreeMap::new();
    let mut total_pointer_hits = 0usize;
    for (start, _end, s) in &strings {
        let target = *start as u32;
        for w in 0..bytes.len().saturating_sub(3) {
            let word = u32::from_le_bytes(bytes[w..w + 4].try_into().unwrap());
            if word == target {
                total_pointer_hits += 1;
                // Nearest record at or before this pointer site.
                if let Some(&record) = records.iter().rev().find(|&&r| r <= w) {
                    let delta = w as i64 - record as i64;
                    *offset_from_record.entry(delta).or_insert(0) += 1;
                }
                if total_pointer_hits <= 20 {
                    let record = records.iter().rev().find(|&&r| r <= w).copied();
                    println!(
                        "    pointer @{w:#x} -> string @{start:#x} ({s}), nearest record {:?} (delta {:?})",
                        record.map(|r| format!("{r:#x}")),
                        record.map(|r| w as i64 - r as i64)
                    );
                }
            }
        }
    }
    println!(
        "  {total_pointer_hits} pointer site(s) found across {} string(s)",
        strings.len()
    );
    println!("  offset-from-nearest-record histogram (top 15):");
    let mut hist: Vec<(i64, usize)> = offset_from_record.into_iter().collect();
    hist.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (delta, count) in hist.iter().take(15) {
        println!("    delta {delta:#x} ({delta}): {count} hit(s)");
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut archive = oag_assets::psarc::Archive::open(TRACK)?;
    let track_blob = archive.read_path(TRACK_PATH)?;
    probe("altima track.rcsmodel", &track_blob)?;

    let ship_blob = archive.read_path(SHIP_PATH)?;
    probe("feisar2048/3 ship.rcsmodel", &ship_blob)?;

    Ok(())
}
