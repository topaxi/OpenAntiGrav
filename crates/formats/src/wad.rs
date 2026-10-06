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
//! equal there and their order is unobservable. The PS2 archives are
//! compressed and their offset chain resolves it: taking the fourth word as the
//! stored size chains all 193 entries of `WADSP.WAD`; the third chains 2.
//!
//! # Names
//!
//! Names are not stored, only [`hash_name`] of each: an entry can be *found* by
//! name but not listed with one. A listing means hashing a candidate list.

use std::fmt;

/// Bytes before the first entry.
pub const HEADER_LEN: usize = 8;

/// Bytes per directory entry.
pub const ENTRY_LEN: usize = 16;

/// Observed alignment of the blob region and of each blob within it.
///
/// Not enforced: a deviation is a finding, not a reason to reject the file.
pub const BLOB_ALIGNMENT: u32 = 64;

/// The only version seen.
pub const KNOWN_VERSION: u32 = 1;

/// Bit 31 of the uncompressed-size field selects zlib over LZSS.
///
/// Only consulted for a compressed entry: see [`Compression`] and `Wad_Read`.
const ZLIB_FLAG: u32 = 0x8000_0000;

/// How a blob is stored.
///
/// # The flag alone does not decide this
///
/// `Wad_Read` asks two questions in order:
///
/// 1. Does `size_in` equal `size_out & 0x7fffffff`? Then the blob is **stored**,
///    whatever bit 31 says.
/// 2. Only then does bit 31 choose zlib over LZSS.
///
/// "Bit 31 clear means LZSS" is the natural mistake: all 1,142 entries in
/// `Data.wad` have bit 31 clear and equal sizes, so it would LZSS-decode the
/// whole archive. An incompressible blob whose LZSS encoding came out its own
/// size is classified `None` and returned verbatim, as the game does, so
/// `size_in == size_out` is **not** evidence a blob was stored rather than
/// compressed, and `oag-wad verify` cannot tell them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    /// Stored verbatim. Every entry in every PSP archive.
    None,
    /// LZSS with a 8192-byte ring, 13-bit offsets and 4-bit lengths, on PS2.
    Lzss,
    /// zlib. Not in Pure or Pulse's **base disc** archives, but 187 of 192
    /// entries in Pure's Gamma DLC pack and most in its other six PSN packs.
    /// See `docs/formats/dlc-pack.md#pures-packs-decrypt-with-an-external-key-table`.
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
    /// Size after decompression, with the zlib flag masked off. Equal to
    /// [`size`](Self::size) when stored as-is.
    pub size_uncompressed: u32,
    /// Bytes occupied in the archive; the offset chain advances by it.
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
            // `PS2MUSIC.WAD` (version 16) and `PRERACE.WAD` (version 32) are
            // not corrupt: count-first PCM archives whose first word is an
            // entry count. Naming that points the reader away from a WAD
            // variant that does not exist.
            Self::UnknownVersion { version } => write!(
                f,
                "unknown WAD version {version} (only {KNOWN_VERSION} is known); \
                 if this is a PS2 `.wad`, the first word may be an entry count \
                 rather than a version - see `crate::ps2_music` and \
                 docs/formats/ps2-audio.md"
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

    /// Reads the entry count from the first [`HEADER_LEN`] bytes, so a caller
    /// can size one read of the directory instead of the whole (up to 315 MiB)
    /// archive.
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
    /// `data` must hold at least [`directory_len`](Self::directory_len) bytes;
    /// more is ignored. `archive_len` enables bounds checking; `None` skips it.
    pub fn parse(data: &[u8], archive_len: Option<u64>) -> Result<Self> {
        let entry_count = Self::peek_entry_count(data)?;
        let version = KNOWN_VERSION;

        let needed = Self::directory_len(entry_count);

        // Guard before allocating: a count of 0xFFFFFFFF would ask for 64 GiB.
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

            // Same order as `Wad_Read`: equal sizes mean stored, and only then
            // does bit 31 choose zlib or LZSS. Reversed, an entry with bit 31 set
            // and equal sizes would reach inflate though the game reads it verbatim.
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
            // Zero-size entries are legitimate; the offset must still be sane.
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
    /// blob's end rounded up to [`BLOB_ALIGNMENT`], the first just after the
    /// padded directory.
    ///
    /// This pins down the field ordering. A diagnostic, not a validation: a
    /// breaking archive is interesting, not invalid. Returns the indices of
    /// entries that break the rule.
    #[must_use]
    pub fn offset_chain_breaks(&self) -> Vec<usize> {
        let mut breaks = Vec::new();
        let mut expected = align_up(Self::directory_len(self.entries.len() as u32));

        for (index, entry) in self.entries.iter().enumerate() {
            if u64::from(entry.offset) != expected {
                breaks.push(index);
                // Resynchronise so one anomaly is not reported for every later entry.
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

/// Inflates a [`Compression::Zlib`] blob.
///
/// A thin wrapper over `miniz_oxide`, the crate's one third-party dependency,
/// so callers reach it through this module. The error is a message:
/// `miniz_oxide`'s own carries no more than its `Debug` output.
///
/// # Errors
///
/// `input` is not a valid zlib stream.
pub fn decompress_zlib(input: &[u8]) -> std::result::Result<Vec<u8>, String> {
    miniz_oxide::inflate::decompress_to_vec_zlib(input).map_err(|e| format!("{e:?}"))
}

/// Reflected CRC-32 polynomial, as used by the game's table builder.
const CRC32_POLY: u32 = 0xEDB8_8320;

/// Hashes an entry name the way the game does.
///
/// CRC-32 with the standard reflected polynomial but **initialised to zero
/// rather than `0xFFFFFFFF`**, so not `zlib`'s `crc32`. The name is normalised
/// first: backslashes become forward slashes and ASCII `A`-`Z` fold to
/// lowercase; bytes at or above `0x80` pass through (newlib's `_ctype_` table
/// drives the game's fold).
///
/// Recovered from `Wad_HashName` at `0x08940d0c` in the PSP `BOOT.BIN`, verified
/// against 176 real entries across four archives. See `docs/formats/wad.md`.
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

/// As [`hash_name`], for a name that is not valid UTF-8. The game hashes bytes.
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
/// Computed, not table-driven: hashing is once per lookup on a handful of names.
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
/// Blobs seen begin with a version byte and a three-character ASCII type code,
/// for example `\x01FNT`. Whether that holds for every asset type is not
/// established, so this reports what it sees.
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

/// What the PS2 build calls the texture a PSP name asks for, if the name is one
/// it would rewrite.
///
/// The two discs share XML, `.vex` models and authored asset paths, not the
/// compiled texture container. The PS2 texture resolver **replaces the source
/// art's extension with its own** before hashing, so one name serves both:
///
/// | Declared | PSP entry | PS2 entry |
/// | --- | --- | --- |
/// | `Data\Tex\engineFlare\Engine_noise.mip` | `008d70a2` | `e9f16c12` |
///
/// `.tga` is rewritten as well as `.mip` (the executable's table lists both).
/// Anything else returns [`None`]: a `.vex` or `.pob` is found under its
/// declared name on both discs.
///
/// # Where this comes from
///
/// `Texture_FindOrLoad` (`0x0010c1e0` in `SCES_547.48`) strips a leading
/// `WIPEOUT PSP\PS2\` build path, replaces `.TGA` then `.MIP` with `.PCT`,
/// hashes with the WAD name hash and looks it up. A name resolving to no file
/// becomes `Data/Tex/Missing.pct` (entry 6910 of `WADS2.WAD`) and is retried.
///
/// See `docs/ghidra/functions/ps2-pulse-eu/texture-names.md`, and
/// `oag_pulse::PS2_IMAGES` for the three entries found by their picture before
/// the rule was known, which now check it.
#[must_use]
pub fn ps2_texture_name(name: &str) -> Option<String> {
    let (stem, extension) = name.rsplit_once('.')?;
    (extension.eq_ignore_ascii_case("mip") || extension.eq_ignore_ascii_case("tga"))
        .then(|| format!("{stem}.pct"))
}

/// The build-time path prefix `Texture_FindOrLoad` strips before it hashes a
/// name, if present.
///
/// A `Texture` node's declared name is sometimes the artists' authoring path.
/// `Texture_FindOrLoad` (`0x0010c1e0` in `SCES_547.48`) removes this one
/// substring wherever it occurs; see
/// `docs/ghidra/functions/ps2-pulse-eu/texture-names.md`. The disc's constant
/// is uppercase and matched with a case-sensitive `strstr`; stripped
/// case-insensitively here, since [`hash_name`] lower-cases anyway.
#[must_use]
pub fn ps2_strip_build_prefix(name: &str) -> String {
    const PREFIX: &str = r"WIPEOUT PSP\PS2\";
    let lower = name.to_ascii_lowercase();
    match lower.find(&PREFIX.to_ascii_lowercase()) {
        Some(pos) => format!("{}{}", &name[..pos], &name[pos + PREFIX.len()..]),
        None => name.to_string(),
    }
}

#[cfg(test)]
mod tests;
