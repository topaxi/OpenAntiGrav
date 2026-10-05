//! Diffs PSARC archives: what a patch or DLC package adds, replaces or repeats.
//!
//! ```text
//! oag-psarc-diff --other <psarc>... [--base <psarc>...] [--tsv <out>] [--no-hash]
//! ```
//!
//! Every `--other` archive's entries are classified against the union of the
//! `--base` archives (paths matched case-insensitively, slash direction
//! ignored):
//!
//! - **new**: no base archive holds the path;
//! - **identical**: a base copy has the same size and the same MD5;
//! - **replaced**: the path exists with different bytes. The cause is never read.
//!
//! With no `--base` every entry is a census row. The summary groups by asset
//! family (see `oag_tools::patch_diff::family`); `--tsv` writes one row per
//! entry: archive, path, family, class, size, md5, base size. Only names,
//! sizes and hashes are written, never bytes. `--no-hash` compares by size
//! alone (equal sizes are then reported `identical?` in the TSV and counted
//! identical), which is fast and enough for a first look.
//!
//! Base entries are hashed only when an `--other` entry shares a path and a
//! size with them, so diffing a small patch against a 13 GB base reads only the
//! overlap.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use oag_assets::psarc::Archive;
use oag_formats::psarc::md5_digest;
use oag_tools::patch_diff::{Class, classify, family, normalise};

#[derive(Parser)]
#[command(version, about = "Classify PSARC entries against a base set")]
struct Cli {
    /// The archives to classify (a patch, a DLC pack).
    #[arg(long, required = true, num_args = 1..)]
    other: Vec<PathBuf>,
    /// The archives to classify against. Omit for a plain census.
    #[arg(long, num_args = 1..)]
    base: Vec<PathBuf>,
    /// Write one row per entry here.
    #[arg(long)]
    tsv: Option<PathBuf>,
    /// Compare by size alone instead of by MD5.
    #[arg(long)]
    no_hash: bool,
}

struct BaseCopy {
    archive: usize,
    index: usize,
    size: u64,
}

#[derive(Default)]
struct Tally {
    entries: u64,
    bytes: u64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut base_archives = Vec::new();
    let mut base: BTreeMap<String, Vec<BaseCopy>> = BTreeMap::new();
    for (n, path) in cli.base.iter().enumerate() {
        let archive =
            Archive::open_file(path).with_context(|| format!("opening {}", path.display()))?;
        eprintln!("base  {} : {} paths", path.display(), archive.paths().len());
        for p in archive.paths() {
            let index = archive.index_of_path(p).context("path without an entry")?;
            let size = archive.directory().entries[index].size;
            base.entry(normalise(p)).or_default().push(BaseCopy {
                archive: n,
                index,
                size,
            });
        }
        base_archives.push(archive);
    }

    let mut tsv = match &cli.tsv {
        Some(p) => Some(std::io::BufWriter::new(
            std::fs::File::create(p).with_context(|| format!("creating {}", p.display()))?,
        )),
        None => None,
    };
    if let Some(w) = tsv.as_mut() {
        writeln!(w, "archive\tpath\tfamily\tclass\tsize\tmd5\tbase_size")?;
    }

    let mut table: BTreeMap<(String, &'static str, Class), Tally> = BTreeMap::new();
    for path in &cli.other {
        let mut other =
            Archive::open_file(path).with_context(|| format!("opening {}", path.display()))?;
        let label = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let paths = other.paths().to_vec();
        eprintln!("other {} : {} paths", path.display(), paths.len());
        for p in &paths {
            let index = other.index_of_path(p).context("path without an entry")?;
            let size = other.directory().entries[index].size;
            let copies = base.get(&normalise(p)).map_or(&[][..], Vec::as_slice);
            let need_hash = !cli.no_hash && copies.iter().any(|c| c.size == size);
            let mut md5 = String::from("-");
            let class = if cli.no_hash {
                if copies.is_empty() {
                    Class::New
                } else if copies.iter().any(|c| c.size == size) {
                    Class::Identical
                } else {
                    Class::Replaced
                }
            } else if need_hash || copies.is_empty() {
                let bytes = other.read(index).with_context(|| format!("reading {p}"))?;
                let sum = md5_digest(&bytes);
                md5 = sum.iter().map(|b| format!("{b:02x}")).collect();
                let mut seen = Vec::new();
                for c in copies {
                    if c.size == size {
                        let b = base_archives[c.archive]
                            .read(c.index)
                            .with_context(|| format!("reading base copy of {p}"))?;
                        seen.push((c.size, Some(md5_digest(&b))));
                    } else {
                        seen.push((c.size, None));
                    }
                }
                classify(size, sum, &seen)
            } else {
                Class::Replaced
            };
            let fam = family(p);
            let t = table.entry((label.clone(), fam, class)).or_default();
            t.entries += 1;
            t.bytes += size;
            if let Some(w) = tsv.as_mut() {
                let base_size = copies
                    .iter()
                    .map(|c| c.size.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                writeln!(
                    w,
                    "{label}\t{p}\t{fam}\t{class}\t{size}\t{md5}\t{base_size}"
                )?;
            }
        }
    }

    println!(
        "{:<22} {:<18} {:<10} {:>8} {:>14}",
        "archive", "family", "class", "entries", "bytes"
    );
    for ((label, fam, class), t) in &table {
        println!(
            "{label:<22} {fam:<18} {class:<10} {:>8} {:>14}",
            t.entries, t.bytes
        );
    }
    Ok(())
}
