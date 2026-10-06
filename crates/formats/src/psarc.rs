//! The PSARC container Wipeout HD / Fury ships its assets in, on PS3.
//!
//! Big-endian throughout.
//!
//! ```text
//! header, 32 bytes:
//!   +0x00  char[4]  "PSAR"
//!   +0x04  u16      version major, 1
//!   +0x06  u16      version minor, 3
//!   +0x08  char[4]  compression, "zlib"
//!   +0x0c  u32      total length of header + entry table + block table
//!   +0x10  u32      bytes per entry, 30
//!   +0x14  u32      entry count, the manifest included
//!   +0x18  u32      uncompressed block size, 65536
//!   +0x1c  u32      flags
//!
//! entry, 30 bytes:
//!   +0x00  u8[16]   MD5 of the entry's own path, uppercased
//!   +0x10  u32      index of this entry's first block
//!   +0x14  u40      uncompressed length
//!   +0x19  u40      byte offset of the first block, from the archive's start
//!
//! block table:
//!   the rest of the declared length, as a flat array of block sizes
//! ```
//!
//! See `docs/formats/psarc.md` for the evidence, and `docs/formats/hd-status.md`
//! for what the archives hold.
//!
//! **Not the PSP's `PSAR`.** The firmware update payload on a UMD starts with
//! the same four bytes and is a different container entirely.
//!
//! # Entry 0 is the manifest, not a file
//!
//! On a well-behaved archive (every PS3 one seen, and Vita `2048`'s
//! `data.psarc`) it inflates to a newline-separated list of every other
//! entry's path, so entry `n + 1` is the `n`th line. Its digest field is
//! sixteen zero bytes, which no other entry has. [`parse_manifest`] reads it;
//! [`path_digest`] ties manifest text, entry order and stride together; and
//! [`match_paths_to_entries`] recovers every entry's path by that digest.
//!
//! **The PS4 Omega Collection family (`omega-ps4-eu`'s `dataNN.psarc`) breaks
//! both halves of that, and the header's version does not say so.** All five
//! base archives declare 1.4. Their manifest is NUL-delimited (`data00`: zero
//! `\n` bytes, 10,925 `\x00` bytes for 10,926 paths). The entry table is the
//! manifest followed by every file sorted by path digest, while the manifest
//! itself is unsorted (entry `n + 1` is line `n` for 3 of `data00`'s 10,926).
//! Every entry behind the manifest names exactly one manifest path and the
//! reverse, 46,005 of 46,005 across base and patch archives. (An earlier
//! reading saw placeholder rows and a mostly-dead manifest; that was a
//! short-read extraction's zero-padding, see `docs/formats/psarc.md`.)
//!
//! **`version_minor >= 4` is not the signal.** Vita `2048`'s `data.psarc`
//! also declares 1.4 and is well-behaved: newline-delimited (zero `\x00`, 18,429
//! `\n`), 18,430 lines matching its entry count, every digest matching its
//! line. Dispatching on the version once regressed every Vita-backed path
//! lookup on `main`; [`parse_manifest`] reads the manifest's own bytes instead
//! and [`match_paths_to_entries`] always matches by digest. See
//! `docs/formats/psarc.md`.
//!
//! # This module does no I/O
//!
//! An archive is gigabytes inside a disc image. [`Directory::parse`] takes the
//! table of contents, [`Directory::entry_range`] names the bytes one entry
//! needs, and [`Directory::read_entry`] turns exactly those into the entry.
//! The caller owns the seeking; see `oag_assets::psarc`.

use std::fmt;

mod md5;

pub use md5::digest as md5_digest;

/// The four bytes an archive starts with.
pub const MAGIC: [u8; 4] = *b"PSAR";

pub const HEADER_LEN: usize = 32;

pub const ENTRY_LEN: usize = 30;

/// Widths tried for a block-table element, narrowest first.
///
/// Not declared: it is whatever divides the table evenly and leaves room for
/// the highest block index any entry names. Wipeout HD uses 2, but that is
/// this disc's choice, so it is probed.
///
/// **The probe is only sound one way round.** An odd table length rules pairs
/// out, so a 3-byte width is recovered; an even one can always be read as
/// pairs, so a 4-byte width is undecidable and the narrowest wins. Revisit for
/// a block size above 65,535; none is known.
const BLOCK_WIDTHS: [usize; 3] = [2, 3, 4];

/// zlib's first byte: deflate with a 32 KiB window.
///
/// A block is stored raw when deflating would not pay, and the block table
/// records the stored length either way, so the header byte decides. See
/// [`Directory::read_entry`].
const ZLIB_CMF: u8 = 0x78;

/// What went wrong reading an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    TooShort {
        need: usize,
        got: usize,
    },
    /// The first four bytes are not `PSAR`.
    ///
    /// A `.psarc` inside an *encrypted* PS3 image reads as noise, so a missing
    /// decryption step produces this. See `docs/formats/ps3-disc.md`.
    BadMagic {
        found: [u8; 4],
    },
    /// The entry stride is not [`ENTRY_LEN`].
    UnknownEntryLen {
        entry_len: u32,
    },
    /// The declared table of contents does not fit around its own header.
    ImplausibleTocLen {
        toc_len: u32,
        /// Bytes the entry table alone would need.
        entries_need: usize,
    },
    /// No block-size width divides the block table and covers every entry.
    NoBlockWidth {
        table_len: usize,
        highest_block: u32,
    },
    /// An entry names a block outside the block table.
    BlockOutOfRange {
        index: usize,
        block: u32,
        blocks: usize,
    },
    /// An entry index past the end of the entry table.
    NoSuchEntry {
        index: usize,
        entries: usize,
    },
    /// A block inflated to something other than the block size, and it was not
    /// the entry's last.
    ///
    /// [`Directory::entry_range`] derives an entry's block count assuming every
    /// block but the last is full. Checked, because a short block in the middle
    /// would silently shift everything after it.
    ShortBlock {
        index: usize,
        block: u32,
        got: usize,
        want: u32,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { need, got } => {
                write!(f, "PSARC needs {need} bytes, got {got}")
            }
            Self::BadMagic { found } => write!(
                f,
                "not a PSARC: magic {found:02x?} (an encrypted PS3 image reads as noise)"
            ),
            Self::UnknownEntryLen { entry_len } => {
                write!(f, "PSARC entry stride {entry_len}, expected {ENTRY_LEN}")
            }
            Self::ImplausibleTocLen {
                toc_len,
                entries_need,
            } => write!(
                f,
                "PSARC table of contents declares {toc_len} bytes, \
                 but its entries alone need {entries_need}"
            ),
            Self::NoBlockWidth {
                table_len,
                highest_block,
            } => write!(
                f,
                "no PSARC block-size width divides {table_len} bytes \
                 and covers block {highest_block}"
            ),
            Self::BlockOutOfRange {
                index,
                block,
                blocks,
            } => write!(f, "PSARC entry {index} names block {block} of {blocks}"),
            Self::NoSuchEntry { index, entries } => {
                write!(f, "PSARC entry {index} of {entries}")
            }
            Self::ShortBlock {
                index,
                block,
                got,
                want,
            } => write!(
                f,
                "PSARC entry {index}: block {block} inflated to {got} bytes, expected {want}"
            ),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// The fixed header, as declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub version_major: u16,
    pub version_minor: u16,
    /// The codec's four-character name, `zlib` on every archive seen.
    pub compression: [u8; 4],
    /// Total length of header, entry table and block table.
    pub toc_len: u32,
    pub entry_len: u32,
    /// Entries the archive declares, the manifest included.
    pub entry_count: u32,
    /// Bytes one block holds once inflated.
    pub block_size: u32,
    /// Unread. 3 on every archive seen.
    pub flags: u32,
}

impl Header {
    /// Reads the header out of the archive's first [`HEADER_LEN`] bytes.
    ///
    /// # Errors
    ///
    /// [`Error::TooShort`], [`Error::BadMagic`] or [`Error::UnknownEntryLen`].
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::TooShort {
                need: HEADER_LEN,
                got: data.len(),
            });
        }
        let magic: [u8; 4] = data[0..4].try_into().expect("four bytes");
        if magic != MAGIC {
            return Err(Error::BadMagic { found: magic });
        }
        let entry_len = u32_at(data, 0x10);
        if entry_len as usize != ENTRY_LEN {
            return Err(Error::UnknownEntryLen { entry_len });
        }
        Ok(Self {
            version_major: u16_at(data, 0x04),
            version_minor: u16_at(data, 0x06),
            compression: data[8..12].try_into().expect("four bytes"),
            toc_len: u32_at(data, 0x0c),
            entry_len,
            entry_count: u32_at(data, 0x14),
            block_size: u32_at(data, 0x18),
            flags: u32_at(data, 0x1c),
        })
    }

    /// The codec name as text, for a report.
    #[must_use]
    pub fn compression_name(&self) -> String {
        String::from_utf8_lossy(&self.compression).into_owned()
    }
}

/// One directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// MD5 of the entry's path, uppercased. Sixteen zeroes on the manifest.
    pub digest: [u8; 16],
    /// Position of this entry's first block within the block table.
    pub first_block: u32,
    pub size: u64,
    /// Byte offset of the first block, from the archive's start.
    pub offset: u64,
}

/// An archive's table of contents, parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    pub header: Header,
    pub entries: Vec<Entry>,
    /// Stored length of each block, `0` meaning a full stored block.
    pub blocks: Vec<u32>,
    pub block_width: usize,
}

impl Directory {
    /// Reads the declared table of contents.
    ///
    /// `toc` is the archive's first `header.toc_len` bytes, the header
    /// included: read [`HEADER_LEN`] bytes, [`Header::parse`] for `toc_len`,
    /// read that many, call this.
    ///
    /// # Errors
    ///
    /// [`Error::TooShort`] when `toc` is shorter than the header declares,
    /// plus everything [`Header::parse`] and the block-width probe can raise.
    pub fn parse(toc: &[u8]) -> Result<Self> {
        let header = Header::parse(toc)?;
        let toc_len = header.toc_len as usize;
        if toc.len() < toc_len {
            return Err(Error::TooShort {
                need: toc_len,
                got: toc.len(),
            });
        }

        let count = header.entry_count as usize;
        let entries_end = HEADER_LEN.saturating_add(count.saturating_mul(ENTRY_LEN));
        if entries_end > toc_len {
            return Err(Error::ImplausibleTocLen {
                toc_len: header.toc_len,
                entries_need: entries_end,
            });
        }

        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let at = HEADER_LEN + i * ENTRY_LEN;
            let e = &toc[at..at + ENTRY_LEN];
            entries.push(Entry {
                digest: e[0..16].try_into().expect("sixteen bytes"),
                first_block: u32_at(e, 0x10),
                size: u40_at(e, 0x14),
                offset: u40_at(e, 0x19),
            });
        }

        let (blocks, block_width) = read_block_table(&toc[entries_end..toc_len], &entries)?;

        // Not validated per entry: a single damaged row (`first_block` past the
        // block table) used to fail the whole directory, leaving two of
        // `omega-ps4-eu`'s five archives unopenable on a short-read extraction.
        // `Directory::entry_range` runs `Error::BlockOutOfRange` lazily, so a
        // bad row fails only the path that names it.

        Ok(Self {
            header,
            entries,
            blocks,
            block_width,
        })
    }

    /// How many entries the archive holds, the manifest included.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the archive declares no entries at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Where entry `index`'s stored bytes lie: `(offset, length)` from the
    /// archive's start.
    ///
    /// The block count comes from the entry's declared length, so the range is
    /// known before any block is decompressed; [`Directory::read_entry`] checks
    /// that assumption, see [`Error::ShortBlock`].
    /// # Errors
    ///
    /// [`Error::NoSuchEntry`] or [`Error::BlockOutOfRange`].
    pub fn entry_range(&self, index: usize) -> Result<(u64, u64)> {
        let entry = self.entries.get(index).ok_or(Error::NoSuchEntry {
            index,
            entries: self.entries.len(),
        })?;
        let mut stored = 0u64;
        for block in self.entry_blocks(index)? {
            stored += if block == 0 {
                u64::from(self.header.block_size)
            } else {
                u64::from(block)
            };
        }
        Ok((entry.offset, stored))
    }

    /// The stored length of each block entry `index` spans, in order.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchEntry`] or [`Error::BlockOutOfRange`].
    pub fn entry_blocks(&self, index: usize) -> Result<Vec<u32>> {
        let entry = self.entries.get(index).ok_or(Error::NoSuchEntry {
            index,
            entries: self.entries.len(),
        })?;
        let block_size = u64::from(self.header.block_size.max(1));
        let count = usize::try_from(entry.size.div_ceil(block_size)).unwrap_or(usize::MAX);
        let first = entry.first_block as usize;
        let end = first.saturating_add(count);
        if end > self.blocks.len() {
            return Err(Error::BlockOutOfRange {
                index,
                block: u32::try_from(end.saturating_sub(1)).unwrap_or(u32::MAX),
                blocks: self.blocks.len(),
            });
        }
        Ok(self.blocks[first..end].to_vec())
    }

    /// Turns the bytes [`Directory::entry_range`] named into the entry itself.
    /// `stored` must be exactly that range. Each block is inflated when it
    /// starts with zlib's header byte and copied otherwise - **and copied anyway
    /// if the inflate fails**: a genuine stream always inflates, so a failure
    /// proves the leading `0x78` was coincidental raw content.
    ///
    /// # Errors
    ///
    /// [`Error::TooShort`] when `stored` is short, [`Error::ShortBlock`] when
    /// a block (inflated or raw) is not the full block size without being the
    /// entry's last.
    pub fn read_entry(&self, index: usize, stored: &[u8]) -> Result<Vec<u8>> {
        let entry = self.entries.get(index).ok_or(Error::NoSuchEntry {
            index,
            entries: self.entries.len(),
        })?;
        let block_size = self.header.block_size as usize;
        let size = usize::try_from(entry.size).unwrap_or(usize::MAX);

        // Validated before allocated: `entry.size` is a u40 read from the file,
        // so a crafted entry could declare a terabyte and `Vec::with_capacity`
        // would commit to it. `entry_blocks` bounds the size by the block table
        // (`size.div_ceil(block_size)` blocks must exist). A public parser owes
        // this guard to itself even though the assets layer calls `entry_range`
        // first. Finding F2 of the 2026-08-18 review.
        let blocks = self.entry_blocks(index)?;

        let mut out = Vec::with_capacity(size);
        let mut at = 0usize;
        for (n, stored_len) in blocks.into_iter().enumerate() {
            let block = entry.first_block + u32::try_from(n).unwrap_or(u32::MAX);
            let take = if stored_len == 0 {
                block_size
            } else {
                stored_len as usize
            };
            let end = at.checked_add(take).ok_or(Error::TooShort {
                need: usize::MAX,
                got: stored.len(),
            })?;
            if end > stored.len() {
                return Err(Error::TooShort {
                    need: end,
                    got: stored.len(),
                });
            }
            let chunk = &stored[at..end];
            at = end;

            // A full-size block is raw by definition and must not be sniffed: a
            // stored length of zero means "this block did not shrink". The test
            // only sorts *short* blocks. `ZLIB_CMF` is `0x78`, an ordinary byte
            // in arbitrary content, so a raw block beginning with it was handed
            // to the inflater and failed: `Data\Music\Exceeder\music_stereo.mp3`
            // is `DATA01.PSARC` entry 16, block 574. See `docs/formats/psarc.md`.
            //
            // A *short* block starting with `0x78` is the same coincidence one
            // level down, and reaches `omega-ps4-eu`: per `docs/formats/psarc.md`'s
            // "Block data location", nothing there is deflated, so entries 2431,
            // 4370 and 4659 of `data00`-`data02.psarc` raised `Error::BadBlock`
            // over one coincidental byte. A real zlib stream inflates or the
            // bytes never were one, so a failed inflate is conclusive and falls
            // back to the raw block.
            let plain = if stored_len != 0 && chunk.first() == Some(&ZLIB_CMF) {
                miniz_oxide::inflate::decompress_to_vec_zlib(chunk)
                    .unwrap_or_else(|_| chunk.to_vec())
            } else {
                chunk.to_vec()
            };
            let last = out.len() + plain.len() >= size;
            if plain.len() != block_size && !last {
                return Err(Error::ShortBlock {
                    index,
                    block,
                    got: plain.len(),
                    want: self.header.block_size,
                });
            }
            out.extend_from_slice(&plain);
        }

        out.truncate(size);
        Ok(out)
    }
}

/// Splits the manifest's text into paths.
/// Splits the manifest's text into paths.
///
/// The delimiter is read off the manifest's own bytes, not the header's
/// version: `omega-ps4-eu`'s five 1.4 archives are NUL-delimited, but Vita
/// `2048`'s 1.4 `data.psarc` is newline-delimited. A manifest with no `\n` is
/// read as NUL-delimited; anything else (including the degenerate one-entry
/// case) as newline-delimited, `\r\n` handled for the CRLF XML archives.
#[must_use]
pub fn parse_manifest(data: &[u8]) -> Vec<String> {
    if !data.contains(&b'\n') && data.contains(&0) {
        return data
            .split(|&b| b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| {
                String::from_utf8_lossy(s)
                    .trim_end_matches('\r')
                    .to_string()
            })
            .filter(|s| !s.trim().is_empty())
            .collect();
    }
    String::from_utf8_lossy(data)
        .lines()
        .map(|line| line.trim_end_matches('\r').to_string())
        .filter(|line| !line.trim().is_empty())
        .collect()
}

/// One archive path together with the directory index that stores it.
/// See [`match_paths_to_entries`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathEntry {
    pub index: usize,
    /// The path this entry stores, exactly as the manifest spells it.
    pub path: String,
}

/// Matches manifest paths to the entries that store them, by [`path_digest`]
/// rather than by position.
///
/// **Not positional, on any archive.** `entry n + 1 is manifest line n` holds
/// on PS3 and Vita archives only because every real entry's digest matches its
/// line's (11,664 of 11,664, `docs/formats/psarc.md`); it is a corollary of
/// digest matching, not a second code path. On `omega-ps4-eu` entry and
/// manifest order agree on nothing (see the module docs), and a version-only
/// dispatch regressed every Vita-backed lookup once.
///
/// A zero digest marks a placeholder row and is skipped. A manifest path with
/// no entry, and an entry matching no path (none of either on a whole Omega
/// extraction), are dropped rather than guessed at. A `first_block` too large
/// for any block table (see [`read_block_table`]) still yields a [`PathEntry`];
/// [`Directory::entry_range`] reports it unreadable.
#[must_use]
pub fn match_paths_to_entries(entries: &[Entry], manifest_paths: &[String]) -> Vec<PathEntry> {
    let mut path_by_digest: std::collections::HashMap<[u8; 16], &str> =
        std::collections::HashMap::with_capacity(manifest_paths.len());
    for path in manifest_paths {
        path_by_digest.entry(path_digest(path)).or_insert(path);
    }

    entries
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, e)| e.digest != [0u8; 16])
        .filter_map(|(index, e)| {
            path_by_digest.get(&e.digest).map(|&path| PathEntry {
                index,
                path: path.to_string(),
            })
        })
        .collect()
}

/// The digest an entry's directory row should carry for `path`.
///
/// MD5 of the path uppercased, leading slash included. The lowercase spelling,
/// which is how paths are stored, matches zero entries. See
/// `docs/formats/psarc.md`.
#[must_use]
pub fn path_digest(path: &str) -> [u8; 16] {
    md5::digest(path.to_uppercase().as_bytes())
}

/// Probes the block-size width and reads the table.
/// A zero-size entry needs no block and its `first_block` is unspecified;
/// folded into the probe's `highest`, an implausible one would fail every
/// width in [`BLOCK_WIDTHS`].
///
/// **A non-zero size does not make `first_block` trustworthy either.** Entries
/// 9042, 4676 and 318 of `omega-ps4-eu`'s `data00`/`data02`/`data04` carry a
/// real digest and a `first_block` in the billions on a short-read extraction;
/// none on a whole one (`docs/formats/psarc.md`, "No corrupt row"). No width
/// covers a `first_block` past `rest.len() / 2` (2 is the narrowest width), so
/// that bound excludes the row from `highest`. The row stays in
/// [`Directory::entries`]; [`Directory::entry_range`] reports it unreadable.
///
/// **Sound for one bad row among many good ones, not for a table of all
/// implausible rows**: `highest` then falls to its `unwrap_or(0)` floor and any
/// even-length table passes at width 2 rather than raising
/// [`Error::NoBlockWidth`]. Not a real case on `omega-ps4-eu`, so no
/// excluded-row count is kept.
fn read_block_table(rest: &[u8], entries: &[Entry]) -> Result<(Vec<u32>, usize)> {
    let narrowest = *BLOCK_WIDTHS
        .iter()
        .min()
        .expect("BLOCK_WIDTHS is non-empty");
    let ceiling = rest.len() / narrowest;
    let highest = entries
        .iter()
        .filter(|e| e.size > 0 && (e.first_block as usize) < ceiling)
        .map(|e| e.first_block)
        .max()
        .unwrap_or(0);
    let needed = highest as usize + usize::from(!entries.is_empty());
    for width in BLOCK_WIDTHS {
        if !rest.len().is_multiple_of(width) || rest.len() / width < needed {
            continue;
        }
        let blocks = rest
            .chunks_exact(width)
            .map(|c| c.iter().fold(0u32, |acc, &b| (acc << 8) | u32::from(b)))
            .collect();
        return Ok((blocks, width));
    }
    Err(Error::NoBlockWidth {
        table_len: rest.len(),
        highest_block: highest,
    })
}

fn u16_at(data: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([data[at], data[at + 1]])
}

fn u32_at(data: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

/// A 40-bit big-endian field, of which the format has two.
fn u40_at(data: &[u8], at: usize) -> u64 {
    data[at..at + 5]
        .iter()
        .fold(0u64, |acc, &b| (acc << 8) | u64::from(b))
}

#[cfg(test)]
mod tests;
