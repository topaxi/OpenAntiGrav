//! Every blob of one WAD on a disc image that a sniff of its first bytes keeps,
//! decompressed whole, for the ground truths that walk an archive for its
//! `.vex` files.
//!
//! Finding a `.vex` means looking inside every entry, and those walks used to
//! read and LZSS-decompress all of them (the whole of the PS2's `WADS2.WAD`) to
//! keep the ones carrying the magic. [`blobs`] decompresses only the first
//! `head` bytes of each entry, reads the whole of it only when `sniff` keeps
//! them, and leaves the caller's own filter to run on the full blob. A sniff
//! that is a necessary condition of that filter finds exactly what the full
//! walk found: `vex::has_magic` reads bytes 12 to 16, so [`VEX_HEAD`] is enough
//! for it. The PS2 PVS walk went from 33 s to 13 s on 2026-10-12.
//!
//! `tests/<dir>/mod.rs` is not a test target of its own, which is why this
//! is a directory.

#![allow(dead_code)]

use oag_disc::{DiscImage, Entry};
use oag_formats::lzss;
use oag_formats::wad::{self, Compression, Directory};

/// Bytes `vex::has_magic` needs.
pub const VEX_HEAD: usize = 16;

/// The archive's own disc entry, by its full path on the disc.
pub fn archive(disc: &mut DiscImage, archive_path: &str) -> Entry {
    disc.entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == archive_path)
        .unwrap_or_else(|| panic!("{archive_path} present"))
        .clone()
}

/// Every non-empty entry of `archive_path` whose first `head` decompressed
/// bytes `sniff` keeps, decompressed whole, with its directory index, in
/// directory order. A zlib entry is a panic: no disc archive these walks read
/// stores one.
pub fn blobs(
    disc: &mut DiscImage,
    archive_path: &str,
    head: usize,
    sniff: impl Fn(&[u8]) -> bool,
) -> Vec<(usize, Vec<u8>)> {
    let archive = archive(disc, archive_path);
    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, Directory::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    // A literal is nine bits, the most any output byte costs, so this much of
    // the stream always yields `head` bytes.
    let head_raw = (head as u64 * 9).div_ceil(8) + 1;
    let mut out = Vec::new();
    for (index, entry) in dir.entries.iter().enumerate() {
        if entry.size == 0 {
            continue;
        }
        let mut read = |len: u64| {
            disc.read_entry_range(&archive, u64::from(entry.offset), len)
                .expect("blob")
        };
        let size = u64::from(entry.size);
        let blob = match entry.compression {
            Compression::None => {
                if !sniff(&read(size.min(head as u64))) {
                    continue;
                }
                read(size)
            }
            Compression::Lzss => {
                let first = lzss::decompress(&read(size.min(head_raw)), head);
                if first.is_ok_and(|first| !sniff(&first)) {
                    continue;
                }
                lzss::decompress(&read(size), entry.size_uncompressed as usize).expect("lzss")
            }
            Compression::Zlib => panic!("{archive_path} entry {index}: unexpected zlib entry"),
        };
        out.push((index, blob));
    }
    out
}
