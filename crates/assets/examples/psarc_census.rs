//! The container-level census `docs/formats/psarc.md`'s Omega section quotes:
//! how the manifest, the entry table and the block table of a `.psarc` agree
//! with each other, and whether every entry reads.
//!
//! `cargo run -p oag-assets --release --example psarc_census -- <a.psarc>...`
//!
//! One block per archive: entries and manifest paths, zero-digest rows,
//! digest-matched paths and orphans on both sides, how many ascending runs the
//! entry table is (one on a well-formed archive), how many entries sit at the
//! same `(offset, size)` as another (the packer's deduplication), how the
//! referenced block rows split between full and short stored blocks, whether
//! `max(offset + size)` lands on the file's length, and every read error.

use std::collections::{HashMap, HashSet};

use oag_formats::psarc::{parse_manifest, path_digest};

fn main() {
    for path in std::env::args().skip(1) {
        let mut archive =
            oag_assets::psarc::Archive::open(&path).unwrap_or_else(|e| panic!("open {path}: {e}"));
        let entries = archive.directory().entries.clone();
        let block_size = u64::from(archive.directory().header.block_size);
        let manifest = archive.read(0).expect("the manifest reads");
        let paths = parse_manifest(&manifest);

        let real: Vec<usize> = (1..entries.len())
            .filter(|&i| entries[i].digest != [0u8; 16])
            .collect();
        let path_digests: HashSet<[u8; 16]> = paths.iter().map(|p| path_digest(p)).collect();
        let entry_digests: HashSet<[u8; 16]> = real.iter().map(|&i| entries[i].digest).collect();
        let runs = 1 + real
            .windows(2)
            .filter(|w| entries[w[1]].digest < entries[w[0]].digest)
            .count();

        let mut errors = Vec::new();
        let (mut full, mut short, mut other, mut referenced) = (0u64, 0u64, 0u64, 0u64);
        let mut seen = HashSet::new();
        let mut by_extent: HashMap<(u64, u64), usize> = HashMap::new();
        for &i in &real {
            let entry = entries[i];
            *by_extent.entry((entry.offset, entry.size)).or_insert(0) += 1;
            if let Err(e) = archive.read(i) {
                errors.push(format!("{i}: {e}"));
                continue;
            }
            let Ok(blocks) = archive.directory().entry_blocks(i) else {
                continue;
            };
            let count = blocks.len() as u64;
            for (k, &block) in blocks.iter().enumerate() {
                if !seen.insert(entry.first_block as usize + k) {
                    continue;
                }
                referenced += 1;
                let remaining = if k as u64 + 1 == count {
                    entry.size - (count - 1) * block_size
                } else {
                    block_size
                };
                match block {
                    0 => full += 1,
                    b if u64::from(b) == remaining => short += 1,
                    _ => other += 1,
                }
            }
        }
        let shared: usize = by_extent
            .iter()
            .filter(|(extent, n)| extent.1 > 0 && **n > 1)
            .map(|(_, n)| *n)
            .sum();
        let file_len = std::fs::metadata(&path).map_or(0, |m| m.len());
        let end = real
            .iter()
            .map(|&i| entries[i].offset + entries[i].size)
            .max()
            .unwrap_or(0);

        println!("{path}");
        println!(
            "  entries {} (manifest included), manifest paths {}, zero-digest rows {}",
            entries.len(),
            paths.len(),
            entries.len() - 1 - real.len()
        );
        println!(
            "  digest-matched {}, entries with no path {}, paths with no entry {}, ascending runs {runs}",
            archive.paths().len(),
            real.iter()
                .filter(|&&i| !path_digests.contains(&entries[i].digest))
                .count(),
            paths
                .iter()
                .filter(|p| !entry_digests.contains(&path_digest(p)))
                .count(),
        );
        println!(
            "  zero-size entries {}, entries sharing an (offset, size) {shared}, max(offset + size) {end} vs file {file_len}",
            real.iter().filter(|&&i| entries[i].size == 0).count()
        );
        println!(
            "  block rows {}, referenced {referenced}: full stored {full}, short stored {short}, other {other}",
            archive.directory().blocks.len()
        );
        println!(
            "  read errors {}: {:?}",
            errors.len(),
            &errors[..errors.len().min(4)]
        );
    }
}
