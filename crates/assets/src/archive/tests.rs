use std::path::PathBuf;

use super::*;

#[test]
fn splits_a_disc_spec() {
    assert_eq!(
        split_disc_spec("data/images/pulse.chd:PSP_GAME/USRDIR/FE.wad"),
        Some(("data/images/pulse.chd", "PSP_GAME/USRDIR/FE.wad"))
    );
}

#[test]
fn a_plain_path_is_not_a_disc_spec() {
    assert_eq!(split_disc_spec("FE.wad"), None);
    assert_eq!(split_disc_spec("C:"), None);
}

#[test]
fn a_bare_word_is_rejected_rather_than_opened() {
    assert!(matches!(Archive::open("nonsense"), Err(Error::BadSpec(_))));
}

/// One stored (uncompressed) entry, 64-byte aligned, the way every PSP
/// archive is laid out.
fn tiny_wad(name_hash: u32, blob: &[u8]) -> Vec<u8> {
    let offset = 64u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1u32.to_le_bytes()); // version
    bytes.extend_from_slice(&1u32.to_le_bytes()); // entry_count
    bytes.extend_from_slice(&name_hash.to_le_bytes());
    bytes.extend_from_slice(&offset.to_le_bytes());
    bytes.extend_from_slice(&(blob.len() as u32).to_le_bytes()); // size_uncompressed
    bytes.extend_from_slice(&(blob.len() as u32).to_le_bytes()); // size
    bytes.resize(offset as usize, 0);
    bytes.extend_from_slice(blob);
    bytes
}

/// A real zlib stream, classified the way [`Compression::Zlib`] requires:
/// bit 31 of `size_uncompressed` set, and a stored size that differs from
/// it. Unlike [`tiny_lzss_wad`], this blob really is what it claims -
/// there is no `oag-formats` deflate encoder to fake it with, so the
/// test compresses with `miniz_oxide` (a dev-dependency here, not a
/// runtime one) and [`wad::decompress_zlib`] has to undo real deflate
/// output, not a hand-shaped stand-in.
fn tiny_zlib_wad(name_hash: u32, plain: &[u8]) -> Vec<u8> {
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(plain, 6);
    assert_ne!(
        compressed.len(),
        plain.len(),
        "the fixture must actually compress, or the entry reads as stored"
    );
    let offset = 64u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1u32.to_le_bytes()); // version
    bytes.extend_from_slice(&1u32.to_le_bytes()); // entry_count
    bytes.extend_from_slice(&name_hash.to_le_bytes());
    bytes.extend_from_slice(&offset.to_le_bytes());
    bytes.extend_from_slice(&((plain.len() as u32) | 0x8000_0000).to_le_bytes());
    bytes.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
    bytes.resize(offset as usize, 0);
    bytes.extend_from_slice(&compressed);
    bytes
}

/// The same layout as [`tiny_wad`], but with the stored and uncompressed
/// sizes disagreeing, which is the only thing that makes `Directory::parse`
/// classify an entry as LZSS. The blob itself is not a real LZSS stream:
/// nothing that reads it should ever get as far as decoding.
fn tiny_lzss_wad(name_hash: u32, blob: &[u8]) -> Vec<u8> {
    let offset = 64u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1u32.to_le_bytes()); // version
    bytes.extend_from_slice(&1u32.to_le_bytes()); // entry_count
    bytes.extend_from_slice(&name_hash.to_le_bytes());
    bytes.extend_from_slice(&offset.to_le_bytes());
    bytes.extend_from_slice(&(blob.len() as u32 * 2).to_le_bytes()); // size_uncompressed
    bytes.extend_from_slice(&(blob.len() as u32).to_le_bytes()); // size
    bytes.resize(offset as usize, 0);
    bytes.extend_from_slice(blob);
    bytes
}

fn temp_file(name: &str, contents: &[u8]) -> PathBuf {
    let path = std::env::temp_dir().join(format!("oag-assets-test-{name}"));
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn len_reports_the_files_own_size() {
    let bytes = tiny_wad(0x1234_5678, b"hello world");
    let path = temp_file("len.wad", &bytes);

    let archive = Archive::open(path.to_str().unwrap()).unwrap();
    assert_eq!(archive.len(), bytes.len() as u64);
    assert!(!archive.is_empty());

    std::fs::remove_file(path).ok();
}

/// Two stored entries back to back, so that a read clamped at the end of
/// the first has something recognisable to run into if the clamp is wrong.
fn tiny_wad_pair(first: &[u8], second: &[u8]) -> Vec<u8> {
    let first_at = 64u32;
    let second_at = first_at + first.len() as u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1u32.to_le_bytes()); // version
    bytes.extend_from_slice(&2u32.to_le_bytes()); // entry_count
    for (hash, at, blob) in [(1u32, first_at, first), (2u32, second_at, second)] {
        bytes.extend_from_slice(&hash.to_le_bytes());
        bytes.extend_from_slice(&at.to_le_bytes());
        bytes.extend_from_slice(&(blob.len() as u32).to_le_bytes()); // size_uncompressed
        bytes.extend_from_slice(&(blob.len() as u32).to_le_bytes()); // size
    }
    bytes.resize(first_at as usize, 0);
    bytes.extend_from_slice(first);
    bytes.extend_from_slice(second);
    bytes
}

#[test]
fn read_range_returns_bytes_from_the_middle_of_an_entry() {
    let blob = b"0123456789abcdef";
    let bytes = tiny_wad(0x1111_2222, blob);
    let path = temp_file("range-mid.wad", &bytes);

    let mut archive = Archive::open(path.to_str().unwrap()).unwrap();
    assert_eq!(archive.read_range(0, 4, 6).unwrap(), b"456789");
    // The offset is into the entry, not into the archive: reading from
    // zero has to land on the blob, not on the directory.
    assert_eq!(archive.read_range(0, 0, 4).unwrap(), b"0123");

    std::fs::remove_file(path).ok();
}

#[test]
fn read_range_stops_at_the_end_of_the_entry() {
    let bytes = tiny_wad_pair(b"first entry", b"SECOND ENTRY");
    let path = temp_file("range-clamp.wad", &bytes);

    // The clamp has to be against the entry, not the archive. Both blobs
    // are in the file, so an over-long read that clamped on the file's
    // length would quietly return the second entry's bytes as well.
    let mut archive = Archive::open(path.to_str().unwrap()).unwrap();
    assert_eq!(archive.read_range(0, 6, 4096).unwrap(), b"entry");

    std::fs::remove_file(path).ok();
}

#[test]
fn read_range_past_the_end_of_an_entry_is_empty() {
    let bytes = tiny_wad_pair(b"first entry", b"SECOND ENTRY");
    let path = temp_file("range-past.wad", &bytes);

    let mut archive = Archive::open(path.to_str().unwrap()).unwrap();
    assert!(archive.read_range(0, 11, 4).unwrap().is_empty());
    assert!(archive.read_range(0, 9_000, 4).unwrap().is_empty());

    std::fs::remove_file(path).ok();
}

#[test]
fn read_range_refuses_a_compressed_entry() {
    let bytes = tiny_lzss_wad(0x3333_4444, b"not really an lzss stream");
    let path = temp_file("range-lzss.wad", &bytes);

    // Refused rather than seeked into, and refused rather than decoded:
    // the blob is not a valid stream, so reaching the decoder at all would
    // surface as `BadBlob` instead.
    let mut archive = Archive::open(path.to_str().unwrap()).unwrap();
    assert!(matches!(
        archive.read_range(0, 4, 4),
        Err(Error::RangeIntoCompressed { .. })
    ));

    std::fs::remove_file(path).ok();
}

#[test]
fn read_range_name_resolves_the_name_and_reports_a_missing_one() {
    let name = r"Data\Sound\gentrak.bnk";
    let bytes = tiny_wad(wad::hash_name(name), b"0123456789abcdef");
    let path = temp_file("range-name.wad", &bytes);

    let mut archive = Archive::open(path.to_str().unwrap()).unwrap();
    assert_eq!(archive.read_range_name(name, 10, 3).unwrap(), b"abc");
    assert!(matches!(
        archive.read_range_name(r"Data\Sound\nothing.bnk", 0, 3),
        Err(Error::NoSuchEntry { .. })
    ));

    std::fs::remove_file(path).ok();
}

#[test]
fn read_raw_matches_read_for_a_stored_entry() {
    let blob = b"a stored entry is never decompressed";
    let bytes = tiny_wad(0xdead_beef, blob);
    let path = temp_file("raw.wad", &bytes);

    let mut archive = Archive::open(path.to_str().unwrap()).unwrap();
    assert_eq!(archive.read_raw(0).unwrap(), blob);
    assert_eq!(archive.read(0).unwrap(), blob);

    std::fs::remove_file(path).ok();
}

/// `Compression::Zlib` was refused outright until Pure's DLC packs turned
/// out to actually use it - see `docs/formats/dlc-pack.md`. This is the
/// first real coverage of that arm, not a stand-in fixture: the blob is
/// genuine deflate output, not a hand-shaped "looks compressed enough"
/// stream the way [`tiny_lzss_wad`]'s is for LZSS.
#[test]
fn a_zlib_entry_inflates_to_the_original_bytes() {
    let plain = b"zlib zlib zlib zlib zlib zlib zlib zlib zlib zlib zlib zlib";
    let bytes = tiny_zlib_wad(0x1357_9bdf, plain);
    let path = temp_file("zlib.wad", &bytes);

    let mut archive = Archive::open(path.to_str().unwrap()).unwrap();
    assert_eq!(
        archive.directory().entries[0].compression,
        Compression::Zlib
    );
    assert_eq!(archive.read(0).unwrap(), plain);

    std::fs::remove_file(path).ok();
}

/// A zlib entry cannot be seeked into any more than an LZSS one can - both
/// reach into state built up from the start of the stream. See
/// `read_range_refuses_a_compressed_entry` above for the LZSS half.
#[test]
fn read_range_refuses_a_zlib_entry_too() {
    let plain = b"zlib zlib zlib zlib zlib zlib zlib zlib zlib zlib zlib zlib";
    let bytes = tiny_zlib_wad(0x2468_ace0, plain);
    let path = temp_file("zlib-range.wad", &bytes);

    let mut archive = Archive::open(path.to_str().unwrap()).unwrap();
    assert!(matches!(
        archive.read_range(0, 0, 4),
        Err(Error::RangeIntoCompressed { .. })
    ));

    std::fs::remove_file(path).ok();
}
