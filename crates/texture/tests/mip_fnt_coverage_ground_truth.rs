//! Byte coverage of every `.mip` and `.fnt` blob on the PSP disc.
//!
//! **`#[ignore]`d and never run in CI.** Needs game content this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # Why these two, and what each found
//!
//! Both are containers whose payload a header names by offset - a font's
//! codepoint table, offset table and atlas; a texture's mip chain - which is
//! exactly the shape `docs/formats/README.md#coverage` names as the highest
//! risk. `.mip` had a gap this project's own doc comments already named
//! (only level 0 of a mip chain is decoded, and `+0x09..+0x10` is never read
//! at all) and this sweep measures it rather than leaving it as prose;
//! `.fnt` closes almost completely, with the same never-read 20-byte header
//! tail as the only gap on every file swept.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::coverage::Coverage;
use oag_formats::wad::{self, Compression, Directory};
use oag_texture::{fnt, texture};

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// Every blob in `archive` that looks like a font or a texture, decompressed.
fn blobs(disc: &mut DiscImage, archive_path: &str) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
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

    let (mut fonts, mut textures) = (Vec::new(), Vec::new());
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
        if fnt::looks_like_font(&blob) {
            fonts.push(blob);
        } else if texture::Texture::looks_like_texture(&blob) {
            textures.push(blob);
        }
    }
    (fonts, textures)
}

fn sweep(label: &str, blobs: &[Vec<u8>], coverage: impl Fn(&[u8]) -> Coverage) {
    let (mut total, mut claimed) = (0u64, 0u64);
    let mut worst: Option<(usize, String)> = None;
    for blob in blobs {
        let seen = coverage(blob);
        if seen.is_empty() {
            continue;
        }
        total += seen.len() as u64;
        claimed += seen.claimed() as u64;
        let gap_len: usize = seen.gaps(1).iter().map(|g| g.len).sum();
        if gap_len > 0 && worst.as_ref().is_none_or(|(n, _)| gap_len > *n) {
            worst = Some((gap_len, seen.describe(1)));
        }
    }
    let pct = 100.0 * claimed as f64 / total.max(1) as f64;
    println!(
        "{label}: {} file(s), {claimed} of {total} byte(s) ({pct:.2}%)",
        blobs.len()
    );
    if let Some((_, describe)) = &worst {
        println!("  worst: {describe}");
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn mip_and_fnt_close_across_the_front_end_archives() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open disc");

    // Fonts: `FE.wad` alone, matching `fnt_ground_truth.rs`'s own count of 5.
    // Sweeping the other three archives too finds 5 more blobs that sniff as
    // a font by `looks_like_font`'s four-byte magic check; not chased further
    // here since `fnt_ground_truth.rs` already establishes 5 is the real
    // count and this test's own job is coverage, not a census.
    let (fonts, _) = blobs(&mut disc, "PSP_GAME/USRDIR/FE.wad");
    assert_eq!(fonts.len(), 5, "the PSP disc's five fonts, in FE.wad alone");
    sweep("fnt", &fonts, fnt::coverage);

    // Textures: every archive, since `.mip` blobs are spread across all four.
    let mut all_textures = Vec::new();
    for archive in [
        "PSP_GAME/USRDIR/FE.wad",
        "PSP_GAME/USRDIR/FEData.wad",
        "PSP_GAME/USRDIR/Data.wad",
        "PSP_GAME/USRDIR/BEData.wad",
    ] {
        let (_, textures) = blobs(&mut disc, archive);
        all_textures.extend(textures);
    }
    assert!(!all_textures.is_empty(), "no .mip blobs found at all");
    sweep("mip", &all_textures, texture::coverage);
}
