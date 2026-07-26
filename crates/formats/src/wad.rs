//! The WAD container used by Wipeout Pure and Pulse, on both PSP and PS2.
//!
//! ```text
//! +0x00  u32  version        always 1 in every archive seen
//! +0x04  u32  entry_count
//! +0x08  entry[entry_count]  16 bytes each
//!        padding to a 64-byte boundary
//!        blob data, each blob starting on a 64-byte boundary
//! ```
//!
//! Entry:
//!
//! ```text
//! +0x00  u32  name_hash            CRC-32 of the name, see `hash_name`
//! +0x04  u32  offset               from the start of the file
//! +0x08  u32  size_uncompressed    bit 31 selects zlib over LZSS
//! +0x0c  u32  size                 bytes actually stored
//! ```
//!
//! See `docs/formats/wad.md` for the evidence behind this layout.
//!
//! # The last two fields are easy to get backwards
//!
//! Every PSP archive stores everything uncompressed, so the two size fields are
//! always equal there and their order is unobservable. The PS2 archives *are*
//! compressed, and their offset chain resolves it: taking the fourth word as
//! the stored size makes all 193 entries of `WADSP.WAD` chain correctly, and
//! taking the third makes 2 of 193 chain.
//!
//! # Names
//!
//! Names are not stored. The directory holds only [`hash_name`] of each name,
//! so an entry can be *found* by name but not listed with one. Recovering a
//! listing means hashing a candidate name list and matching.

use std::fmt;

/// Bytes before the first entry.
pub const HEADER_LEN: usize = 8;

/// Bytes per directory entry.
pub const ENTRY_LEN: usize = 16;

/// Observed alignment of the blob region and of each blob within it.
///
/// Not enforced. It holds in every archive examined, but a deviation is a
/// finding to report rather than a reason to reject the file.
pub const BLOB_ALIGNMENT: u32 = 64;

/// The only version seen.
pub const KNOWN_VERSION: u32 = 1;

/// Bit 31 of the uncompressed-size field selects zlib over LZSS.
///
/// It is only consulted for an entry that is compressed at all: see
/// [`Compression`] and `Wad_Read`.
const ZLIB_FLAG: u32 = 0x8000_0000;

/// How a blob is stored.
///
/// # The flag alone does not decide this
///
/// `Wad_Read` asks two questions in order, and the order is the part that is
/// easy to get wrong:
///
/// 1. Does `size_in` equal `size_out & 0x7fffffff`? Then the blob is **stored**,
///    whatever bit 31 says.
/// 2. Only then does bit 31 choose zlib over LZSS.
///
/// Reading it as "bit 31 clear means LZSS" is the natural mistake, and it makes
/// nonsense of the shipped data: every one of the 1,142 entries in `Data.wad`
/// has bit 31 clear and equal sizes, so that rule would have the game LZSS-decode
/// the entire archive.
///
/// The consequence for us is a real one rather than a technicality. An
/// incompressible blob whose LZSS encoding came out exactly its own size is
/// classified `None` here and returned verbatim, which is what the game does too,
/// so we match. But it means `size_in == size_out` is **not** evidence that a
/// blob was stored rather than compressed, and `oag-wad verify` cannot tell those
/// apart either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    /// Stored verbatim. Every entry in every PSP archive.
    None,
    /// LZSS with a 8192-byte ring, 13-bit offsets and 4-bit lengths.
    ///
    /// Used by the PS2 archives.
    Lzss,
    /// zlib. Supported by the game but not seen in any shipped archive.
    Zlib,
}

impl fmt::Display for Compression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Lzss => "lzss",
            Self::Zlib => "zlib",
        })
    }
}

/// A directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// CRC-32 of the entry's name. See [`hash_name`].
    pub name_hash: u32,
    /// Offset of the blob from the start of the archive.
    pub offset: u32,
    /// Size after decompression, with the zlib flag masked off.
    ///
    /// Equal to [`size`](Self::size) when the blob is stored as-is, which is
    /// the case for every entry in every PSP archive.
    pub size_uncompressed: u32,
    /// Bytes actually occupied in the archive.
    ///
    /// This is the field the offset chain advances by, which is how its
    /// position was determined.
    pub size: u32,
    /// How the blob is stored.
    pub compression: Compression,
}

impl Entry {
    /// Whether the blob is stored compressed.
    #[must_use]
    pub fn is_compressed(&self) -> bool {
        self.compression != Compression::None
    }

    /// Ratio of decompressed to stored size, or 1.0 when stored as-is.
    #[must_use]
    pub fn compression_ratio(&self) -> f64 {
        if self.size == 0 {
            return 1.0;
        }
        f64::from(self.size_uncompressed) / f64::from(self.size)
    }

    /// One past the last byte of the blob.
    #[must_use]
    pub fn end(&self) -> u64 {
        u64::from(self.offset) + u64::from(self.size)
    }
}

/// A parsed archive directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    /// Format version. Expected to be [`KNOWN_VERSION`].
    pub version: u32,
    /// The entries, in the order they appear in the file.
    pub entries: Vec<Entry>,
}

/// Something wrong with an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the header needs.
    TooShort {
        /// Bytes required.
        need: usize,
        /// Bytes supplied.
        got: usize,
    },
    /// The version field is not one we recognise.
    UnknownVersion {
        /// The value found.
        version: u32,
    },
    /// The entry count is implausible for the archive's size.
    ImplausibleEntryCount {
        /// The value found.
        entry_count: u32,
        /// Size of the archive, when known.
        archive_len: Option<u64>,
    },
    /// An entry points outside the archive.
    EntryOutOfBounds {
        /// Index of the offending entry.
        index: usize,
        /// Where its data would end.
        end: u64,
        /// Size of the archive.
        archive_len: u64,
    },
    /// An entry's data would overlap the directory.
    EntryOverlapsDirectory {
        /// Index of the offending entry.
        index: usize,
        /// The entry's offset.
        offset: u32,
        /// Where the directory ends.
        directory_end: u64,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { need, got } => {
                write!(f, "need at least {need} bytes, got {got}")
            }
            Self::UnknownVersion { version } => write!(
                f,
                "unknown WAD version {version} (only {KNOWN_VERSION} is known)"
            ),
            Self::ImplausibleEntryCount {
                entry_count,
                archive_len,
            } => match archive_len {
                Some(len) => write!(
                    f,
                    "entry count {entry_count} needs a {}-byte directory, \
                     but the archive is only {len} bytes",
                    Directory::directory_len(*entry_count)
                ),
                None => write!(f, "implausible entry count {entry_count}"),
            },
            Self::EntryOutOfBounds {
                index,
                end,
                archive_len,
            } => write!(
                f,
                "entry {index} ends at {end} but the archive is {archive_len} bytes"
            ),
            Self::EntryOverlapsDirectory {
                index,
                offset,
                directory_end,
            } => write!(
                f,
                "entry {index} starts at {offset}, inside the directory which ends at {directory_end}"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

impl Directory {
    /// Bytes occupied by the header plus `entry_count` entries, before padding.
    #[must_use]
    pub fn directory_len(entry_count: u32) -> u64 {
        HEADER_LEN as u64 + u64::from(entry_count) * ENTRY_LEN as u64
    }

    /// Reads the entry count from the first [`HEADER_LEN`] bytes.
    ///
    /// Lets a caller size a single read of the directory rather than reading
    /// the whole archive, which matters when the archive is 315 MiB.
    pub fn peek_entry_count(header: &[u8]) -> Result<u32> {
        if header.len() < HEADER_LEN {
            return Err(Error::TooShort {
                need: HEADER_LEN,
                got: header.len(),
            });
        }

        let version = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        if version != KNOWN_VERSION {
            return Err(Error::UnknownVersion { version });
        }

        Ok(u32::from_le_bytes([
            header[4], header[5], header[6], header[7],
        ]))
    }

    /// Parses a directory.
    ///
    /// `data` must hold at least [`directory_len`](Self::directory_len) bytes;
    /// anything beyond that is ignored, so passing the whole archive is fine.
    ///
    /// `archive_len` enables bounds checking. Pass `None` when the total size
    /// is not known, and entry bounds go unchecked.
    pub fn parse(data: &[u8], archive_len: Option<u64>) -> Result<Self> {
        let entry_count = Self::peek_entry_count(data)?;
        let version = KNOWN_VERSION;

        let needed = Self::directory_len(entry_count);

        // Guard before allocating: a corrupt count of 0xFFFFFFFF would
        // otherwise ask for a 64 GiB directory.
        if let Some(len) = archive_len
            && needed > len
        {
            return Err(Error::ImplausibleEntryCount {
                entry_count,
                archive_len: Some(len),
            });
        }
        if needed > data.len() as u64 {
            return Err(Error::TooShort {
                need: needed as usize,
                got: data.len(),
            });
        }

        let mut entries = Vec::with_capacity(entry_count as usize);
        for i in 0..entry_count as usize {
            let at = HEADER_LEN + i * ENTRY_LEN;
            let word = |n: usize| {
                let o = at + n * 4;
                u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]])
            };
            let size_out_raw = word(2);
            let size = word(3);
            let size_uncompressed = size_out_raw & !ZLIB_FLAG;

            // Two tests, in this order, mirroring `Wad_Read`: the sizes
            // agreeing means stored, and only when they differ does bit 31
            // choose between zlib and LZSS. Reversing them matters, because an
            // entry with bit 31 set and equal sizes is read verbatim by the game
            // and would be handed to inflate here.
            let compression = if size_uncompressed == size {
                Compression::None
            } else if size_out_raw & ZLIB_FLAG != 0 {
                Compression::Zlib
            } else {
                Compression::Lzss
            };

            entries.push(Entry {
                name_hash: word(0),
                offset: word(1),
                size_uncompressed,
                size,
                compression,
            });
        }

        let directory = Self { version, entries };

        if let Some(len) = archive_len {
            directory.check_bounds(len, needed)?;
        }

        Ok(directory)
    }

    fn check_bounds(&self, archive_len: u64, directory_end: u64) -> Result<()> {
        for (index, entry) in self.entries.iter().enumerate() {
            // A zero-size entry is legitimate: real archives contain them.
            // Its offset still has to be sane.
            if entry.end() > archive_len {
                return Err(Error::EntryOutOfBounds {
                    index,
                    end: entry.end(),
                    archive_len,
                });
            }
            if entry.size > 0 && u64::from(entry.offset) < directory_end {
                return Err(Error::EntryOverlapsDirectory {
                    index,
                    offset: entry.offset,
                    directory_end,
                });
            }
        }
        Ok(())
    }

    /// Total bytes across all blobs.
    #[must_use]
    pub fn payload_len(&self) -> u64 {
        self.entries.iter().map(|e| u64::from(e.size)).sum()
    }

    /// Total bytes after decompressing everything.
    #[must_use]
    pub fn uncompressed_len(&self) -> u64 {
        self.entries
            .iter()
            .map(|e| u64::from(e.size_uncompressed))
            .sum()
    }

    /// Entries stored compressed.
    ///
    /// Empty for every PSP archive; non-empty for the PS2 ones.
    #[must_use]
    pub fn compressed_entries(&self) -> Vec<(usize, &Entry)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_compressed())
            .collect()
    }

    /// Checks the observed packing rule: each blob begins at the previous
    /// blob's end rounded up to [`BLOB_ALIGNMENT`], and the first begins just
    /// after the padded directory.
    ///
    /// This is what pins down the field ordering, since no other assignment of
    /// the four words produces a consistent chain. It is a diagnostic rather
    /// than a validation: an archive that breaks it is interesting, not
    /// invalid.
    ///
    /// Returns the indices of entries that break the rule.
    #[must_use]
    pub fn offset_chain_breaks(&self) -> Vec<usize> {
        let mut breaks = Vec::new();
        let mut expected = align_up(Self::directory_len(self.entries.len() as u32));

        for (index, entry) in self.entries.iter().enumerate() {
            if u64::from(entry.offset) != expected {
                breaks.push(index);
                // Resynchronise, so one anomaly does not report every
                // subsequent entry as broken too.
                expected = align_up(entry.end());
            } else {
                expected = align_up(entry.end());
            }
        }

        breaks
    }
}

/// Rounds up to the next [`BLOB_ALIGNMENT`] boundary.
#[must_use]
pub fn align_up(value: u64) -> u64 {
    let a = u64::from(BLOB_ALIGNMENT);
    value.div_ceil(a) * a
}

/// Reflected CRC-32 polynomial, as used by the game's table builder.
const CRC32_POLY: u32 = 0xEDB8_8320;

/// Hashes an entry name the way the game does.
///
/// This is CRC-32 with the standard reflected polynomial, but **initialised to
/// zero rather than `0xFFFFFFFF`**, so it is not `zlib`'s `crc32`. The name is
/// normalised first: backslashes become forward slashes, and ASCII `A`-`Z` fold
/// to lowercase. Bytes at or above `0x80` pass through unchanged, because the
/// game's fold is driven by newlib's `_ctype_` table.
///
/// Recovered from `Wad_HashName` at `0x08940d0c` in the PSP `BOOT.BIN`, and
/// verified against 176 real entries across four archives. See
/// `docs/formats/wad.md`.
///
/// ```
/// use oag_formats::wad::hash_name;
/// assert_eq!(hash_name(r"Data\FE\Images\hex_bg.mip"), 0x1aa8_7b99);
/// // Normalisation makes these three the same entry.
/// assert_eq!(hash_name("data/fe/images/hex_bg.mip"), 0x1aa8_7b99);
/// assert_eq!(hash_name(r"DATA\FE\IMAGES\HEX_BG.MIP"), 0x1aa8_7b99);
/// ```
#[must_use]
pub fn hash_name(name: &str) -> u32 {
    hash_name_bytes(name.as_bytes())
}

/// As [`hash_name`], for a name that is not valid UTF-8.
///
/// Names on these discs are ASCII, but the game hashes bytes, so this is the
/// honest signature.
#[must_use]
pub fn hash_name_bytes(name: &[u8]) -> u32 {
    let mut crc: u32 = 0;

    for &byte in name {
        let c = match byte {
            b'\\' => b'/',
            b'A'..=b'Z' => byte + 0x20,
            other => other,
        };
        crc = crc32_step(crc, c);
    }

    !crc
}

/// One byte of reflected CRC-32.
///
/// Computed rather than table-driven. A 256-entry table would be faster, but
/// hashing happens once per lookup on a handful of names, and the shift form is
/// obviously the algorithm it claims to be.
fn crc32_step(crc: u32, byte: u8) -> u32 {
    let mut v = (crc ^ u32::from(byte)) & 0xff;
    for _ in 0..8 {
        v = if v & 1 != 0 {
            (v >> 1) ^ CRC32_POLY
        } else {
            v >> 1
        };
    }
    v ^ (crc >> 8)
}

/// The leading bytes of a blob.
///
/// Blobs seen so far begin with a version byte followed by a three-character
/// ASCII type code, for example `\x01FNT`. Whether that holds for every asset
/// type is not yet established, so this reports what it sees rather than
/// insisting on the pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blob {
    /// The first byte, presumed a version.
    pub version: u8,
    /// The next three bytes, when they are printable ASCII.
    pub tag: Option<String>,
    /// The first four bytes verbatim.
    pub magic: [u8; 4],
}

impl Blob {
    /// Reads the leading bytes of a blob. Returns `None` for anything shorter
    /// than four bytes.
    #[must_use]
    pub fn peek(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let magic = [data[0], data[1], data[2], data[3]];
        let tag = magic[1..4]
            .iter()
            .all(|b| b.is_ascii_graphic())
            .then(|| String::from_utf8_lossy(&magic[1..4]).into_owned());

        Some(Self {
            version: magic[0],
            tag,
            magic,
        })
    }

    /// A short label, `\x01FNT` style when the tag is printable.
    #[must_use]
    pub fn label(&self) -> String {
        match &self.tag {
            Some(tag) => format!("v{} {tag}", self.version),
            None => format!(
                "{:02x}{:02x}{:02x}{:02x}",
                self.magic[0], self.magic[1], self.magic[2], self.magic[3]
            ),
        }
    }
}

#[cfg(test)]
mod tests {
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
            // Field order matters: uncompressed first, then the stored size
            // that the offset chain advances by.
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
        // 0xFFFFFFFF entries would be a 64 GiB directory. The archive length
        // check has to happen before the Vec::with_capacity.
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

    /// Bounds are checked against the stored size. An entry that decompresses
    /// to more than the archive holds is normal, not corrupt.
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

    /// The two size fields are easy to transpose, because on every PSP archive
    /// they are equal and the order cannot be observed. The offset chain is
    /// what distinguishes them, so this pins the ordering down.
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

    /// Values taken from real archive entries whose names were recovered from
    /// strings in the game binary. These are the ground truth for the hash.
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
        // Falls out of init 0 and a final complement, and is the clearest
        // single check that the initial value is 0 rather than 0xFFFFFFFF.
        assert_eq!(hash_name(""), 0xffff_ffff);
    }

    #[test]
    fn is_not_zlib_crc32() {
        // zlib's crc32 initialises to 0xFFFFFFFF. If someone "simplifies" this
        // to a stock CRC-32 call, every lookup silently misses.
        assert_ne!(
            hash_name("a"),
            0xe8b7_be43,
            "0xe8b7be43 is zlib crc32(\"a\")"
        );
    }

    #[test]
    fn high_bytes_are_not_case_folded() {
        // The game's fold comes from newlib's _ctype_, which marks only ASCII
        // A-Z as uppercase. A naive to_ascii_lowercase would agree, but a
        // locale-aware fold would not.
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

    /// `Wad_Read` tests the sizes *before* it looks at bit 31, so an entry whose
    /// sizes agree is read verbatim no matter what the flag says. Classifying
    /// that as zlib would hand a stored blob to inflate.
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
}
