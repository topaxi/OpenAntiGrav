//! Byte coverage of every `.bnk` sound bank on the PSP disc.
//!
//! **`#[ignore]`d and never run in CI.** Needs game content this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! **Scope note**: this measures bytes only. It does not touch
//! `ASSUMED_SAMPLE_RATE`, the SAS pitch question, opcode decoding or
//! `crates/game/src/audio` - see `oag_formats::sblk_coverage`'s own module
//! doc.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::sblk::{self, Bank};
use oag_formats::sblk_coverage;
use oag_formats::wad::{self, Compression, Directory};

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// Every `SBlk` blob in `archive`, decompressed.
fn banks_of(disc: &mut DiscImage, archive_path: &str) -> Vec<Vec<u8>> {
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == archive_path)
        .unwrap_or_else(|| panic!("{archive_path} present"))
        .clone();

    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, Directory::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    let mut out = Vec::new();
    for entry in &dir.entries {
        if entry.size == 0 {
            continue;
        }
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let blob = match entry.compression {
            Compression::None => raw,
            Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            Compression::Zlib => continue,
        };
        if sblk::looks_like_bank(&blob) {
            out.push(blob);
        }
    }
    out
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_psp_bank_closes_to_the_measured_floor() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open disc");

    let mut banks = Vec::new();
    for archive in ["PSP_GAME/USRDIR/FE.wad", "PSP_GAME/USRDIR/Data.wad"] {
        banks.extend(banks_of(&mut disc, archive));
    }
    assert!(!banks.is_empty(), "no banks found at all");

    let (mut total, mut claimed) = (0u64, 0u64);
    let mut worst: Option<(usize, String)> = None;
    let mut named_only = 0usize;
    for blob in &banks {
        let seen = sblk_coverage::coverage(blob);
        if seen.is_empty() {
            continue;
        }
        total += seen.len() as u64;
        claimed += seen.claimed() as u64;
        let gap_len: usize = seen.gaps(1).iter().map(|g| g.len).sum();
        if gap_len > 0 && worst.as_ref().is_none_or(|(n, _)| gap_len > *n) {
            worst = Some((gap_len, seen.describe(1)));
        }
        if Bank::parse(blob).is_ok() {
            named_only += 1;
        }
    }
    let pct = 100.0 * claimed as f64 / total.max(1) as f64;
    println!(
        "sblk: {} bank(s) ({named_only} parse), {claimed} of {total} byte(s) ({pct:.2}%)",
        banks.len()
    );
    if let Some((_, describe)) = &worst {
        println!("  worst: {describe}");
    }
    assert!(claimed > 0, "nothing claimed at all");
}
