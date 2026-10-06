//! What the WAD reader in [`super`] is asserted to do: the entry table, lookup
//! by name and hash, and the archives it refuses. Its own file because the
//! tests exceed the 200-line inline limit (`scripts/check-file-size.py`).

use super::*;

/// Builds an archive by hand. No game data is used in any test.
fn build(entries: &[(u32, u32)]) -> Vec<u8> {
    let with_sizes: Vec<_> = entries.iter().map(|&(h, s)| (h, s, s)).collect();
    build_with_compression(&with_sizes)
}

/// As [`build`], but each entry is `(hash, stored_size, uncompressed_size)`.
fn build_with_compression(entries: &[(u32, u32, u32)]) -> Vec<u8> {
    let mut dir = Vec::new();
    dir.extend(KNOWN_VERSION.to_le_bytes());
    dir.extend((entries.len() as u32).to_le_bytes());

    let mut offset = align_up(Directory::directory_len(entries.len() as u32)) as u32;
    let mut blobs: Vec<(u32, u32)> = Vec::new();

    for &(hash, size, size_uncompressed) in entries {
        dir.extend(hash.to_le_bytes());
        dir.extend(offset.to_le_bytes());
        // Uncompressed size first, then the stored size the offset chain advances by.
        dir.extend(size_uncompressed.to_le_bytes());
        dir.extend(size.to_le_bytes());
        blobs.push((offset, size));
        offset = align_up(u64::from(offset) + u64::from(size)) as u32;
    }

    let mut out = vec![0u8; offset as usize];
    out[..dir.len()].copy_from_slice(&dir);
    for (i, (at, size)) in blobs.iter().enumerate() {
        for b in 0..*size as usize {
            out[*at as usize + b] = (i as u8).wrapping_add(b as u8);
        }
    }
    out
}

#[test]
fn parses_a_directory() {
    let data = build(&[(0xf55e014c, 0x5330), (0xd61a1fb9, 0x93f0)]);
    let dir = Directory::parse(&data, Some(data.len() as u64)).unwrap();

    assert_eq!(dir.version, 1);
    assert_eq!(dir.entries.len(), 2);
    assert_eq!(dir.entries[0].name_hash, 0xf55e_014c);
    assert_eq!(dir.entries[0].size, 0x5330);
    assert_eq!(dir.entries[1].name_hash, 0xd61a_1fb9);
}

#[test]
fn the_offset_chain_holds_for_a_well_formed_archive() {
    let data = build(&[(1, 100), (2, 200), (3, 64), (4, 1)]);
    let dir = Directory::parse(&data, Some(data.len() as u64)).unwrap();
    assert!(dir.offset_chain_breaks().is_empty());
}

#[test]
fn a_broken_offset_chain_is_reported_not_rejected() {
    let mut data = build(&[(1, 100), (2, 200)]);
    // Push entry 1's blob 64 bytes later than the packing rule predicts.
    let at = HEADER_LEN + ENTRY_LEN + 4;
    let shifted = u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) + 64;
    data[at..at + 4].copy_from_slice(&shifted.to_le_bytes());
    data.resize(data.len() + 64, 0);

    let dir = Directory::parse(&data, Some(data.len() as u64)).expect("still parses");
    assert_eq!(dir.offset_chain_breaks(), vec![1]);
}

#[test]
fn peek_entry_count_avoids_reading_the_whole_archive() {
    let data = build(&[(1, 10), (2, 20), (3, 30)]);
    assert_eq!(Directory::peek_entry_count(&data[..HEADER_LEN]).unwrap(), 3);
}

#[test]
fn rejects_an_unknown_version() {
    let mut data = build(&[(1, 10)]);
    data[0] = 2;
    assert_eq!(
        Directory::parse(&data, None),
        Err(Error::UnknownVersion { version: 2 })
    );
}

#[test]
fn rejects_a_corrupt_entry_count_before_allocating() {
    // 0xFFFFFFFF entries would be a 64 GiB directory; the archive length check
    // has to come before the Vec::with_capacity.
    let mut data = build(&[(1, 10)]);
    data[4..8].copy_from_slice(&u32::MAX.to_le_bytes());

    let err = Directory::parse(&data, Some(data.len() as u64)).unwrap_err();
    assert!(matches!(err, Error::ImplausibleEntryCount { .. }));
}

#[test]
fn rejects_an_entry_pointing_past_the_end() {
    let mut data = build(&[(1, 10)]);
    let at = HEADER_LEN + 12; // the stored size, at +0x0c
    data[at..at + 4].copy_from_slice(&0xffff_u32.to_le_bytes());

    let err = Directory::parse(&data, Some(data.len() as u64)).unwrap_err();
    assert!(matches!(err, Error::EntryOutOfBounds { .. }));
}

/// Bounds are checked against the stored size: an entry decompressing to more
/// than the archive holds is normal.
#[test]
fn a_large_uncompressed_size_is_not_out_of_bounds() {
    let data = build_with_compression(&[(1, 64, 10_000_000)]);
    let dir = Directory::parse(&data, Some(data.len() as u64)).expect("should parse");
    assert_eq!(dir.entries[0].size_uncompressed, 10_000_000);
}

#[test]
fn rejects_an_entry_overlapping_the_directory() {
    let mut data = build(&[(1, 10)]);
    let at = HEADER_LEN + 4; // the offset field
    data[at..at + 4].copy_from_slice(&4u32.to_le_bytes());

    let err = Directory::parse(&data, Some(data.len() as u64)).unwrap_err();
    assert!(matches!(err, Error::EntryOverlapsDirectory { .. }));
}

#[test]
fn a_zero_size_entry_is_allowed() {
    // Real archives contain these, so they must not be treated as corrupt.
    let data = build(&[(1, 0), (2, 100)]);
    let dir = Directory::parse(&data, Some(data.len() as u64)).unwrap();
    assert_eq!(dir.entries[0].size, 0);
}

#[test]
fn reports_a_short_buffer() {
    assert_eq!(
        Directory::parse(&[0u8; 4], None),
        Err(Error::TooShort { need: 8, got: 4 })
    );
}

#[test]
fn detects_compressed_entries() {
    // Stored 100 bytes, expanding to 350.
    let data = build_with_compression(&[(1, 100, 350), (2, 64, 64)]);
    let dir = Directory::parse(&data, Some(data.len() as u64)).unwrap();

    assert_eq!(dir.compressed_entries().len(), 1);
    assert_eq!(dir.entries[0].size, 100);
    assert_eq!(dir.entries[0].size_uncompressed, 350);
    assert!((dir.entries[0].compression_ratio() - 3.5).abs() < 1e-9);
    assert!(!dir.entries[1].is_compressed());

    assert_eq!(dir.payload_len(), 164, "payload counts stored bytes");
    assert_eq!(dir.uncompressed_len(), 414);
}

/// The two size fields are easy to transpose, since on every PSP archive they
/// are equal. The offset chain distinguishes them; this pins the order.
#[test]
fn the_offset_chain_advances_by_the_stored_size_not_the_uncompressed_one() {
    let data = build_with_compression(&[(1, 100, 100_000), (2, 200, 200_000)]);
    let dir = Directory::parse(&data, Some(data.len() as u64)).unwrap();

    assert!(
        dir.offset_chain_breaks().is_empty(),
        "chain must follow the stored size; using size_uncompressed would break it"
    );
    // Directory is 8 + 2*16 = 40 -> 64. First blob at 64, 100 stored bytes,
    // so the second starts at align64(164) = 192.
    assert_eq!(dir.entries[0].offset, 64);
    assert_eq!(dir.entries[1].offset, 192);
}

#[test]
fn compression_ratio_handles_a_zero_size_entry() {
    let data = build_with_compression(&[(1, 0, 0)]);
    let dir = Directory::parse(&data, Some(data.len() as u64)).unwrap();
    assert_eq!(dir.entries[0].compression_ratio(), 1.0);
}

#[test]
fn aligns_up_to_the_blob_boundary() {
    assert_eq!(align_up(0), 0);
    assert_eq!(align_up(1), 64);
    assert_eq!(align_up(64), 64);
    assert_eq!(align_up(65), 128);
    // 8 + 27 * 16 = 440, which is where FE.wad's directory ends.
    assert_eq!(align_up(440), 448);
}

#[test]
fn reads_a_printable_blob_tag() {
    let blob = Blob::peek(b"\x01FNT\x00\x00").unwrap();
    assert_eq!(blob.version, 1);
    assert_eq!(blob.tag.as_deref(), Some("FNT"));
    assert_eq!(blob.label(), "v1 FNT");
}

#[test]
fn falls_back_to_hex_for_an_unprintable_tag() {
    let blob = Blob::peek(&[0x01, 0x00, 0xff, 0x02]).unwrap();
    assert_eq!(blob.tag, None);
    assert_eq!(blob.label(), "0100ff02");
}

#[test]
fn a_too_short_blob_has_no_tag() {
    assert_eq!(Blob::peek(b"ab"), None);
}

/// Values from real archive entries whose names were recovered from strings in
/// the game binary: the ground truth for the hash.
#[test]
fn hashes_match_real_archive_entries() {
    for (name, expected) in [
        (r"Data\FE\Images\hex_bg.mip", 0x1aa8_7b99u32),
        (r"Data\Sound\frontend.bnk", 0x75a9_1641),
        (r"Data\Psys\WO_SHIP_COLL_SPARK_DAMAGE.POB", 0xeff1_f331),
        (r"Data\FE\Profile\profile_movie.pmf", 0xed5f_544b),
    ] {
        assert_eq!(hash_name(name), expected, "{name}");
    }
}

#[test]
fn normalisation_makes_separators_and_case_irrelevant() {
    let want = 0x1aa8_7b99;
    assert_eq!(hash_name(r"Data\FE\Images\hex_bg.mip"), want);
    assert_eq!(hash_name("data/fe/images/hex_bg.mip"), want);
    assert_eq!(hash_name(r"DATA\FE\IMAGES\HEX_BG.MIP"), want);
    assert_eq!(
        hash_name("Data/FE\\Images/hex_bg.mip"),
        want,
        "mixed separators"
    );
}

#[test]
fn the_empty_name_hashes_to_all_ones() {
    // Falls out of init 0 and a final complement: the clearest check that the
    // initial value is 0 rather than 0xFFFFFFFF.
    assert_eq!(hash_name(""), 0xffff_ffff);
}

#[test]
fn is_not_zlib_crc32() {
    // zlib's crc32 initialises to 0xFFFFFFFF; a "simplification" to a stock
    // CRC-32 would silently miss every lookup.
    assert_ne!(
        hash_name("a"),
        0xe8b7_be43,
        "0xe8b7be43 is zlib crc32(\"a\")"
    );
}

#[test]
fn high_bytes_are_not_case_folded() {
    // The game's fold (newlib's _ctype_) uppercases only ASCII A-Z; a
    // locale-aware fold would differ.
    assert_ne!(hash_name_bytes(&[0xC0]), hash_name_bytes(&[0xE0]));
}

#[test]
fn reads_the_zlib_flag_and_masks_it_out() {
    let mut data = build_with_compression(&[(1, 100, 350)]);
    let at = HEADER_LEN + 8; // size_uncompressed
    data[at..at + 4].copy_from_slice(&(350u32 | 0x8000_0000).to_le_bytes());

    let dir = Directory::parse(&data, Some(data.len() as u64)).unwrap();
    assert_eq!(dir.entries[0].compression, Compression::Zlib);
    assert_eq!(
        dir.entries[0].size_uncompressed, 350,
        "the flag must not leak into the size"
    );
}

/// `Wad_Read` tests the sizes *before* bit 31, so an entry with agreeing sizes
/// is read verbatim whatever the flag says; zlib would hand a stored blob to
/// inflate.
#[test]
fn equal_sizes_mean_stored_even_with_the_zlib_flag_set() {
    let mut data = build_with_compression(&[(1, 128, 128)]);
    let at = HEADER_LEN + 8;
    data[at..at + 4].copy_from_slice(&(128u32 | 0x8000_0000).to_le_bytes());

    let dir = Directory::parse(&data, Some(data.len() as u64)).unwrap();
    assert_eq!(dir.entries[0].compression, Compression::None);
    assert_eq!(dir.entries[0].size_uncompressed, 128);
}

#[test]
fn classifies_lzss_and_uncompressed_entries() {
    let data = build_with_compression(&[(1, 100, 350), (2, 64, 64)]);
    let dir = Directory::parse(&data, Some(data.len() as u64)).unwrap();
    assert_eq!(dir.entries[0].compression, Compression::Lzss);
    assert_eq!(dir.entries[1].compression, Compression::None);
}
