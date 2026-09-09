//! Scratch probe: does a submesh record carry a plain **small integer index**
//! into the material offset table?
//!
//! `oag_rcs::rcsmodel::psp2::material`'s module doc records that every
//! submesh field was checked against every material's *identity* (`+0x00`,
//! `+0x08`, the name pointer) and nothing matched. That rules out a hash or a
//! pointer - and rules out nothing about an index, because an index of 3 does
//! not resemble any material's identity at all and such a search would never
//! have seen it.
//!
//! The test has its own control group built in, which is what makes it
//! evidence rather than a guess:
//!
//! - on a model whose table names exactly **one** material, the field must be
//!   `0` on every submesh - 522 of 950 models are in this shape;
//! - on a multi-material model, every value must fall inside `[0, count)`;
//! - and across the corpus the values must actually **vary**, otherwise a
//!   field that is simply always zero passes both halves while meaning
//!   nothing.
//!
//! The window scans **before** the record as well as after: records are found
//! through the relocation table rather than by walking section B, so what this
//! reading calls `record` may sit inside a larger enclosing struct.
//!
//! ```sh
//! cargo run -q --release -p oag-game --example vita_rcsmodel_material_index_probe
//! ```

use std::collections::BTreeMap;

use oag_rcs::rcsmodel::psp2;

const PACKAGES: [&str; 3] = [
    "data/extracted/vita/PCSF00007/base/PSP2/data.psarc",
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

/// Byte offsets, relative to a submesh record's own start, that are scanned.
const FROM: i64 = -0x60;
const TO: i64 = 0x80;

/// How a candidate offset is doing, read at one of three widths.
#[derive(Default, Clone)]
struct Candidate {
    /// Submeshes seen at all.
    seen: u64,
    /// Submeshes on a one-material model whose value was not 0.
    single_nonzero: u64,
    /// Submeshes whose value was outside `[0, material count)`.
    out_of_range: u64,
    /// Distinct values seen anywhere.
    distinct: std::collections::BTreeSet<u32>,
    /// Largest value seen.
    max: u32,
    /// Summed over multi-material models: how many distinct values each used.
    used_total: u64,
    /// The best coverage any one model reached - distinct values used divided
    /// by the materials it declares. A real index into a 527-entry table on a
    /// 2,800-submesh circuit should approach 1.0; a small enum cannot.
    best_coverage: f64,
    /// Multi-material models counted.
    multi_models: u64,
    /// Multi-material models where every declared material was named.
    full_coverage: u64,
    /// Multi-material models where at least 90% were.
    most_coverage: u64,
}

fn read(file: &[u8], at: i64, width: usize) -> Option<u32> {
    let at = usize::try_from(at).ok()?;
    match width {
        1 => file.get(at).map(|&b| u32::from(b)),
        2 => file
            .get(at..at + 2)
            .map(|b| u32::from(u16::from_le_bytes(b.try_into().unwrap()))),
        _ => file
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap())),
    }
}

fn main() -> anyhow::Result<()> {
    let mut candidates: BTreeMap<(usize, i64), Candidate> = BTreeMap::new();
    let (mut models, mut single, mut multi) = (0u64, 0u64, 0u64);

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
            let count = model.materials.len() as u32;
            if count == 0 || model.submeshes.is_empty() {
                continue;
            }
            models += 1;
            if count == 1 {
                single += 1;
            } else {
                multi += 1;
            }
            let Some(cpu) = model.sections.first() else {
                continue;
            };

            for width in [1usize, 2, 4] {
                let step = width as i64;
                let mut delta = FROM;
                while delta <= TO {
                    let mut in_model: std::collections::BTreeSet<u32> =
                        std::collections::BTreeSet::new();
                    let slot = candidates.entry((width, delta)).or_default();
                    for submesh in &model.submeshes {
                        let base = (cpu.at + submesh.record) as i64;
                        let Some(value) = read(&blob, base + delta, width) else {
                            continue;
                        };
                        slot.seen += 1;
                        if count == 1 && value != 0 {
                            slot.single_nonzero += 1;
                        }
                        if value >= count {
                            slot.out_of_range += 1;
                        }
                        if slot.distinct.len() < 4096 {
                            slot.distinct.insert(value);
                        }
                        slot.max = slot.max.max(value);
                        in_model.insert(value);
                    }
                    if count > 1 {
                        slot.multi_models += 1;
                        slot.used_total += in_model.len() as u64;
                        let coverage = in_model.len() as f64 / f64::from(count);
                        slot.best_coverage = slot.best_coverage.max(coverage);
                        slot.full_coverage += u64::from(coverage >= 1.0);
                        slot.most_coverage += u64::from(coverage >= 0.9);
                    }
                    delta += step;
                }
            }
        }
    }

    println!("{models} models with a material table ({single} single, {multi} multi)");
    let mut survivors: Vec<_> = candidates
        .iter()
        .filter(|(_, c)| c.single_nonzero == 0 && c.out_of_range == 0 && c.distinct.len() > 1)
        .collect();
    survivors.sort_by(|a, b| b.1.best_coverage.total_cmp(&a.1.best_coverage));
    println!("{} offsets satisfy every condition:", survivors.len());
    for ((width, delta), c) in &survivors {
        println!(
            "  record{delta:+#x} as u{}: {} distinct, max {}, mean {:.1} used per \
             multi-material model, best coverage {:.3}",
            width * 8,
            c.distinct.len(),
            c.max,
            c.used_total as f64 / c.multi_models.max(1) as f64,
            c.best_coverage,
        );
    }

    // The winner, in detail: the numbers the format page will quote.
    if let Some(c) = candidates.get(&(2usize, -0x18)) {
        println!(
            "record-0x18 as u16: {} submeshes checked, {} out of range, {} non-zero on a \
             single-material model, {} models reached full coverage, {} reached 90%+",
            c.seen, c.out_of_range, c.single_nonzero, c.full_coverage, c.most_coverage
        );
    }

    // The near-misses are worth seeing: an offset that is in range everywhere
    // but non-zero on a single-material model is a different field, and one
    // that is zero on singles but out of range elsewhere is probably padding
    // next to something real.
    println!("near misses (in range everywhere, but non-zero on a single-material model):");
    for ((width, delta), c) in &candidates {
        if c.out_of_range == 0 && c.single_nonzero > 0 {
            println!(
                "  record{delta:+#x} as u{}: {} single-material submeshes non-zero",
                width * 8,
                c.single_nonzero
            );
        }
    }
    Ok(())
}
