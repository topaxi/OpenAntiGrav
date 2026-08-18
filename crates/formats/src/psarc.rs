//! The PSARC container Wipeout HD / Fury ships its assets in, on PS3.
//!
//! Big-endian throughout, which is the first thing to get wrong on a format
//! whose name is usually met on little-endian tooling.
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
//! It inflates to a newline-separated list of every other entry's path, in
//! entry order, so entry `n + 1` is the `n`th line. Its own digest field is
//! sixteen zero bytes - it has no path to hash - and it is the only entry for
//! which that is true. [`parse_manifest`] reads it; [`path_digest`] is the
//! check that ties the manifest text, the entry ordering and the entry stride
//! together.
//!
//! # This module does no I/O
//!
//! An archive is gigabytes and lives inside a disc image, so nothing here
//! slurps one. [`Directory::parse`] takes the declared table of contents,
//! [`Directory::entry_range`] names the bytes one entry needs, and
//! [`Directory::read_entry`] turns exactly those bytes into the entry. The
//! caller owns the seeking; see `oag_assets::psarc`.

use std::fmt;

mod md5;

pub use md5::digest as md5_digest;

/// The four bytes an archive starts with.
pub const MAGIC: [u8; 4] = *b"PSAR";

/// Bytes before the entry table.
pub const HEADER_LEN: usize = 32;

/// Bytes per entry, and the only stride seen.
pub const ENTRY_LEN: usize = 30;

/// Widths tried for a block-table element, narrowest first.
///
/// The width is not declared: it is whatever divides the table evenly and
/// leaves room for the highest block index any entry names. Wipeout HD uses 2,
/// because a 64 KiB block cannot deflate to more than 65,535 bytes and the
/// exporter picked the narrowest width that fits - but that is this disc's
/// choice, not the format's, so it is probed rather than assumed.
///
/// **The probe is only sound one way round.** An odd table length rules pairs
/// out, so a 3-byte width is recovered; an even one can always be read as
/// pairs, so a 4-byte width is undecidable from the table alone and the
/// narrowest wins. An archive declaring a block size above 65,535 would be the
/// case to revisit this for; none is known.
const BLOCK_WIDTHS: [usize; 3] = [2, 3, 4];

/// zlib's first byte: deflate with a 32 KiB window.
///
/// A block is stored raw whenever deflating it would not have paid, and the
/// block table records the stored length either way - so the header byte, not
/// the size, is what decides. See [`Directory::read_entry`].
const ZLIB_CMF: u8 = 0x78;

/// What went wrong reading an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Fewer bytes than the structure being read needs.
    TooShort {
        /// Bytes required.
        need: usize,
        /// Bytes supplied.
        got: usize,
    },
    /// The first four bytes are not `PSAR`.
    ///
    /// A `.psarc` inside an *encrypted* PS3 image reads as noise, so this is
    /// the error a missing decryption step produces. See
    /// `docs/formats/ps3-disc.md`.
    BadMagic {
        /// The four bytes found.
        found: [u8; 4],
    },
    /// The entry stride is not [`ENTRY_LEN`].
    UnknownEntryLen {
        /// The value found.
        entry_len: u32,
    },
    /// The declared table of contents does not fit around its own header.
    ImplausibleTocLen {
        /// The value found.
        toc_len: u32,
        /// Bytes the entry table alone would need.
        entries_need: usize,
    },
    /// No block-size width divides the block table and covers every entry.
    NoBlockWidth {
        /// Bytes left for the block table.
        table_len: usize,
        /// Highest block index any entry names.
        highest_block: u32,
    },
    /// An entry names a block outside the block table.
    BlockOutOfRange {
        /// Index of the offending entry.
        index: usize,
        /// The block it named.
        block: u32,
        /// Blocks the table holds.
        blocks: usize,
    },
    /// An entry index past the end of the entry table.
    NoSuchEntry {
        /// The index asked for.
        index: usize,
        /// Entries the archive declares.
        entries: usize,
    },
    /// A block did not inflate.
    BadBlock {
        /// Index of the entry being read.
        index: usize,
        /// Position of the block within the table.
        block: u32,
    },
    /// A block inflated to something other than the block size, and it was not
    /// the entry's last.
    ///
    /// [`Directory::entry_range`] derives an entry's block count from its
    /// declared length on the assumption that every block but the last is full.
    /// That assumption is checked rather than trusted, because a short block in
    /// the middle would silently shift everything after it.
    ShortBlock {
        /// Index of the entry being read.
        index: usize,
        /// Position of the block within the table.
        block: u32,
        /// Bytes it produced.
        got: usize,
        /// Bytes a full block holds.
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
            Self::BadBlock { index, block } => {
                write!(f, "PSARC entry {index}: block {block} did not inflate")
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

/// Shorthand for this module's results.
pub type Result<T> = std::result::Result<T, Error>;

/// The fixed header, as declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// Major version. 1 on every archive seen.
    pub version_major: u16,
    /// Minor version. 3 on every archive seen.
    pub version_minor: u16,
    /// The codec's four-character name, `zlib` on every archive seen.
    pub compression: [u8; 4],
    /// Total length of header, entry table and block table.
    pub toc_len: u32,
    /// Bytes per entry, checked against [`ENTRY_LEN`].
    pub entry_len: u32,
    /// Entries the archive declares, the manifest included.
    pub entry_count: u32,
    /// Bytes one block holds once inflated.
    pub block_size: u32,
    /// Unread. 3 on every archive seen, so nothing here can say what its bits
    /// mean.
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
    /// Uncompressed length.
    pub size: u64,
    /// Byte offset of the first block, from the archive's start.
    pub offset: u64,
}

/// An archive's table of contents, parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    /// The fixed header.
    pub header: Header,
    /// Every entry, the manifest first.
    pub entries: Vec<Entry>,
    /// Stored length of each block, `0` meaning a full stored block.
    pub blocks: Vec<u32>,
    /// Bytes per block-table element, as probed. See [`BLOCK_WIDTHS`].
    pub block_width: usize,
}

impl Directory {
    /// Reads the declared table of contents.
    ///
    /// `toc` is the archive's first `header.toc_len` bytes, the header
    /// included - so a caller reads [`HEADER_LEN`] bytes, calls
    /// [`Header::parse`] to learn `toc_len`, then reads that many and calls
    /// this.
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
        for (index, entry) in entries.iter().enumerate() {
            if entry.first_block as usize >= blocks.len() && entry.size > 0 {
                return Err(Error::BlockOutOfRange {
                    index,
                    block: entry.first_block,
                    blocks: blocks.len(),
                });
            }
        }

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
    /// The block count comes from the entry's declared length rather than from
    /// walking until enough has inflated, so the range is known before a single
    /// block is decompressed. [`Directory::read_entry`] checks the assumption
    /// that buys - see [`Error::ShortBlock`].
    ///
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
    ///
    /// `stored` must be exactly that range: this walks it block by block, and
    /// a block is inflated when it starts with zlib's own header byte and
    /// copied otherwise.
    ///
    /// # Errors
    ///
    /// [`Error::TooShort`] when `stored` is short, [`Error::BadBlock`] when a
    /// block does not inflate, [`Error::ShortBlock`] when one inflates to less
    /// than a full block without being the entry's last.
    pub fn read_entry(&self, index: usize, stored: &[u8]) -> Result<Vec<u8>> {
        let entry = self.entries.get(index).ok_or(Error::NoSuchEntry {
            index,
            entries: self.entries.len(),
        })?;
        let block_size = self.header.block_size as usize;
        let size = usize::try_from(entry.size).unwrap_or(usize::MAX);

        // **Validated before allocated**, the rule this crate applies in about
        // thirty parsers and slipped on here: `entry.size` is a u40 read
        // straight out of the file, so a crafted entry can declare a terabyte
        // and `Vec::with_capacity` would commit to it before anything checked
        // whether the entry's blocks are even in the archive. `entry_blocks`
        // is that check, and running it first bounds the declared size by the
        // file's own block table - `size.div_ceil(block_size)` blocks have to
        // exist. In practice the assets layer calls `entry_range` first, which
        // validates the same span, but `read_entry`'s own contract does not
        // require that and a public parser owes its guard to itself. Finding
        // F2 of the 2026-08-18 review.
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

            // **A full-size block is raw by definition, and must not be
            // sniffed.** A stored length of zero is the table's way of saying
            // "this block did not shrink, so it is here as it is" - there is
            // nothing to inflate, and testing its first byte asks a question
            // that has no meaning. The test only sorts *short* blocks, where a
            // deflate stream and a payload that merely did not pay are both
            // possible.
            //
            // This is not a tidiness point. `ZLIB_CMF` is `0x78`, which is an
            // ordinary byte in the middle of arbitrary content, so a raw block
            // beginning with it was handed to the inflater and failed:
            // `Data\Music\Exceeder\music_stereo.mp3` is `DATA01.PSARC` entry
            // 16, its block 574 is exactly that, and the entry was unreadable
            // until this line distinguished the two cases. See
            // `docs/formats/psarc.md`.
            let plain = if stored_len != 0 && chunk.first() == Some(&ZLIB_CMF) {
                miniz_oxide::inflate::decompress_to_vec_zlib(chunk)
                    .map_err(|_| Error::BadBlock { index, block })?
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

/// Splits the manifest's text into paths, in entry order.
///
/// Entry `n + 1` is line `n`. Blank lines are dropped, and `\r\n` is handled
/// because the archives that carry plain-text XML use CRLF throughout.
#[must_use]
pub fn parse_manifest(data: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(data)
        .lines()
        .map(|line| line.trim_end_matches('\r').to_string())
        .filter(|line| !line.trim().is_empty())
        .collect()
}

/// The digest an entry's directory row should carry for `path`.
///
/// MD5 of the path with every character uppercased, the leading slash
/// included. The lowercase spelling - which is how the paths are actually
/// stored - matches zero entries, so the uppercasing is measured rather than
/// assumed. See `docs/formats/psarc.md`.
#[must_use]
pub fn path_digest(path: &str) -> [u8; 16] {
    md5::digest(path.to_uppercase().as_bytes())
}

/// Probes the block-size width and reads the table.
fn read_block_table(rest: &[u8], entries: &[Entry]) -> Result<(Vec<u32>, usize)> {
    let highest = entries.iter().map(|e| e.first_block).max().unwrap_or(0);
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
