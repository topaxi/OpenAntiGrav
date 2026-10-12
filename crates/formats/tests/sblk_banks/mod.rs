//! Every `SBlk` bank in one WAD on a disc image, for the ground truths that
//! walk the banks (`sblk_cue_ground_truth.rs`, `sblk_child_ground_truth.rs`).
//!
//! A bank is a WAD entry like any other, so finding one means looking inside
//! every entry. That used to mean reading and decompressing all of them (the
//! whole of the PS2's `WADS2.WAD`) and keeping the few that parsed: most of
//! those two files' 570 s of summed test time on 2026-10-12. [`banks_in`]
//! first decompresses only the four bytes `sblk::looks_like_bank` tests before
//! anything else (the container version, in either byte order), and reads the
//! whole entry only when they match. The full blob is then held to the same
//! `looks_like_bank`, so the banks found are exactly the ones the full walk
//! found.
//!
//! `tests/<dir>/mod.rs` is not a test target of its own, which is why this
//! is a directory.

use oag_disc::DiscImage;
use oag_formats::{ByteOrder, lzss, sblk, wad};

/// Raw bytes read to sniff a compressed entry. Every LZSS token is at most
/// 17 bits and yields at least one byte, so 64 bytes always yield the four
/// the sniff needs; a short decode reads the whole entry instead.
const SNIFF_RAW: u64 = 64;

/// Whether `head` could open a bank: `looks_like_bank_as` refuses anything
/// whose first word is not the container version.
fn could_be_bank(head: &[u8]) -> bool {
    head.len() >= 4
        && [ByteOrder::Little, ByteOrder::Big]
            .into_iter()
            .any(|order| order.u32(head, 0) == sblk::VERSION)
}

/// Every `SBlk` blob in one archive, decompressed, with its entry hash.
pub fn banks_in(disc: &mut DiscImage, archive_path: &str) -> Vec<(u32, Vec<u8>)> {
    let Some(archive) = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == archive_path)
        .cloned()
    else {
        panic!("{archive_path} present");
    };
    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = wad::Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, wad::Directory::directory_len(count))
        .expect("directory");
    let dir = wad::Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    let mut out = Vec::new();
    for entry in &dir.entries {
        if entry.size == 0 {
            continue;
        }
        let mut read = |len: u64| {
            disc.read_entry_range(&archive, u64::from(entry.offset), len)
                .expect("blob")
        };
        let size = u64::from(entry.size);
        let blob = match entry.compression {
            wad::Compression::None => {
                if !could_be_bank(&read(size.min(4))) {
                    continue;
                }
                read(size)
            }
            wad::Compression::Lzss => {
                let head = lzss::decompress(&read(size.min(SNIFF_RAW)), 4);
                if head.is_ok_and(|head| !could_be_bank(&head)) {
                    continue;
                }
                lzss::decompress(&read(size), entry.size_uncompressed as usize).expect("lzss")
            }
            wad::Compression::Zlib => continue,
        };
        if sblk::looks_like_bank(&blob) {
            out.push((entry.name_hash, blob));
        }
    }
    out
}
