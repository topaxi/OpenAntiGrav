//! A container file holding only the byte ranges of an archive that were read.
//!
//! This is what a server extraction stores in place of an archive: the same
//! file name at the same relative path, the archive's full length, and the
//! bytes of just the ranges a race load read. Everything an archive's reader
//! asks of its source (the header, the table of contents, an entry's bytes) is
//! a range read, so a reader opened on this behaves exactly as on the whole
//! archive for every read the extraction recorded, and fails by name for any
//! other ([`crate::Error::NotExtracted`]) rather than answering with zeros.
//!
//! # Layout (little endian)
//!
//! | bytes | field |
//! | --- | --- |
//! | 4 | magic [`MAGIC`] |
//! | 4 | version, 1 |
//! | 8 | the archive's length |
//! | 4 | range count `n` |
//! | `16 n` | `(offset: u64, len: u64)` per range, ascending, never touching |
//! | rest | the ranges' bytes, concatenated in the same order |

use std::io::{Read, Seek, SeekFrom};

use crate::{Error, Result};

/// The first four bytes of a sparse archive.
pub const MAGIC: [u8; 4] = *b"OAGX";

const VERSION: u32 = 1;
const HEADER: usize = 20;

/// An archive's recorded ranges: the header and range table only. The ranges'
/// bytes stay in the file and are read on demand ([`Self::locate`]), so opening
/// an extraction costs its table, not its size (a Pulse `Data.wad` extraction is
/// 184 MiB, and a race process held it whole, twice).
#[derive(Debug)]
pub struct Sparse {
    len: u64,
    /// `(offset, len, position in the file)`, ascending by offset.
    ranges: Vec<(u64, u64, u64)>,
}

impl Sparse {
    /// Whether `head` (a file's first bytes) is a sparse archive.
    #[must_use]
    pub fn is_sparse(head: &[u8]) -> bool {
        head.starts_with(&MAGIC)
    }

    /// Parses a sparse file's header and range table from `file`, checking that the
    /// file holds exactly the bytes the table names. Reads nothing of the ranges.
    ///
    /// # Errors
    ///
    /// [`Error::BadSparse`] on a file that is cut short, has another version, or whose
    /// ranges run past the archive's length or overlap; an I/O error reading it.
    pub fn parse(label: &str, file: &mut (impl Read + Seek)) -> Result<Self> {
        let bad = |why: &str| Error::BadSparse {
            archive: label.to_string(),
            why: why.to_string(),
        };
        let file_len = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;
        let mut header = [0u8; HEADER];
        if file.read_exact(&mut header).is_err() || !Self::is_sparse(&header) {
            return Err(bad("not a sparse archive"));
        }
        let word = |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().expect("4"));
        let wide = |b: &[u8], at: usize| u64::from_le_bytes(b[at..at + 8].try_into().expect("8"));
        if word(&header, 4) != VERSION {
            return Err(bad("another version"));
        }
        let len = wide(&header, 8);
        let count = word(&header, 16) as usize;
        let table_end = count
            .checked_mul(16)
            .and_then(|table| table.checked_add(HEADER))
            .filter(|end| *end as u64 <= file_len)
            .ok_or_else(|| bad("the range table is cut short"))?;
        let mut table = vec![0u8; table_end - HEADER];
        file.read_exact(&mut table)
            .map_err(|_| bad("the range table is cut short"))?;
        let mut ranges = Vec::with_capacity(count);
        let (mut at_data, mut previous_end) = (table_end as u64, 0u64);
        for n in 0..count {
            let (offset, size) = (wide(&table, 16 * n), wide(&table, 16 * n + 8));
            let end = offset.checked_add(size).filter(|end| *end <= len);
            let Some(end) = end.filter(|_| offset >= previous_end) else {
                return Err(bad("a range overlaps another or runs past the archive"));
            };
            ranges.push((offset, size, at_data));
            at_data = at_data
                .checked_add(size)
                .ok_or_else(|| bad("a range is too large"))?;
            previous_end = end;
        }
        if at_data != file_len {
            return Err(bad("the data does not match the range table"));
        }
        Ok(Self { len, ranges })
    }

    /// The archive's length, as the whole archive had it.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Whether the archive is empty (never, an archive has a header).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// How many bytes of the archive are present.
    #[must_use]
    pub fn present(&self) -> u64 {
        self.ranges.iter().map(|range| range.1).sum()
    }

    /// Where in the sparse file the `len` archive bytes at `offset` start, when one
    /// recorded range holds them all.
    #[must_use]
    pub fn locate(&self, offset: u64, len: u64) -> Option<u64> {
        let end = offset.checked_add(len)?;
        let at = self.ranges.partition_point(|range| range.0 <= offset);
        let &(start, size, in_file) = self.ranges.get(at.checked_sub(1)?)?;
        (end <= start + size).then(|| in_file + (offset - start))
    }

    /// `len` bytes at `offset`, read from `file` (the file this was parsed from), when
    /// one recorded range holds them all; `Ok(None)` when none does.
    ///
    /// # Errors
    ///
    /// An I/O error reading `file`.
    pub fn read(
        &self,
        file: &mut (impl Read + Seek),
        offset: u64,
        len: u64,
    ) -> Result<Option<Vec<u8>>> {
        if len == 0 {
            return Ok(Some(Vec::new()));
        }
        let Some(at) = self.locate(offset, len) else {
            return Ok(None);
        };
        let size = usize::try_from(len).map_err(|_| Error::BadSparse {
            archive: String::new(),
            why: format!("a read of {len} bytes"),
        })?;
        file.seek(SeekFrom::Start(at))?;
        let mut buf = vec![0u8; size];
        file.read_exact(&mut buf)?;
        Ok(Some(buf))
    }
}

/// Writes a sparse archive of `len` bytes holding `ranges` (ascending, merged),
/// each filled by `fill(offset, len)`.
///
/// # Errors
///
/// Whatever `fill` returns, and a range that is out of order or past `len`.
pub fn write(
    len: u64,
    ranges: &[(u64, u64)],
    mut fill: impl FnMut(u64, u64) -> Result<Vec<u8>>,
) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(
        &u32::try_from(ranges.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    for (offset, size) in ranges {
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
    }
    for &(offset, size) in ranges {
        let bytes = fill(offset, size)?;
        if bytes.len() as u64 != size {
            return Err(Error::BadSparse {
                archive: String::new(),
                why: format!("{offset}+{size} read back {} bytes", bytes.len()),
            });
        }
        out.extend_from_slice(&bytes);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn built() -> (Sparse, Cursor<Vec<u8>>) {
        let whole: Vec<u8> = (0..=99).collect();
        let ranges = [(10, 5), (40, 10)];
        let bytes = write(100, &ranges, |offset, len| {
            Ok(whole[offset as usize..(offset + len) as usize].to_vec())
        })
        .expect("writes");
        let mut file = Cursor::new(bytes);
        (Sparse::parse("t", &mut file).expect("parses"), file)
    }

    #[test]
    fn a_recorded_range_reads_back_and_the_rest_is_refused() {
        let (sparse, mut file) = built();
        let mut read = |offset, len| sparse.read(&mut file, offset, len).expect("reads");
        assert_eq!(read(12, 3), Some(vec![12u8, 13, 14]));
        assert_eq!(read(40, 10).map(|b| b.len()), Some(10));
        assert_eq!(read(9, 2), None, "starts before a range");
        assert_eq!(read(14, 2), None, "runs past a range");
        assert_eq!(read(0, 1), None, "in a hole");
        assert_eq!(
            read(77, 0),
            Some(Vec::new()),
            "nothing asked is nothing read"
        );
        assert_eq!(sparse.len(), 100);
        assert_eq!(sparse.present(), 15);
    }

    #[test]
    fn a_truncated_file_is_refused_by_name() {
        let mut bytes = write(100, &[(0, 4)], |_, len| Ok(vec![7; len as usize])).expect("writes");
        bytes.pop();
        let error = Sparse::parse("Data.wad", &mut Cursor::new(bytes)).expect_err("cut short");
        assert!(error.to_string().contains("Data.wad"), "{error}");
    }

    /// Opening keeps only the table: an archive opened on a sparse file reads each entry
    /// from the file when asked, never from a copy made at open (a Pulse `Data.wad`
    /// extraction held whole cost a race process 572 MB instead of 24 MB,
    /// `docs/networking/online-lobby.md`, "Server memory while loading").
    #[test]
    fn an_archive_on_a_sparse_file_reads_the_file_at_each_read() {
        let wad = crate::testing::wad(&[("first", b"before"), ("second", b"other")]);
        let bytes = write(wad.len() as u64, &[(0, wad.len() as u64)], |offset, len| {
            Ok(wad[offset as usize..(offset + len) as usize].to_vec())
        })
        .expect("writes");
        let dir = crate::testing::temp_dir("sparse-lazy");
        let path = dir.join("Data.wad");
        std::fs::write(&path, &bytes).expect("writing the sparse file");
        let mut archive = crate::Archive::open(&path.to_string_lossy()).expect("opens");
        assert_eq!(archive.read_name("first").unwrap(), b"before");

        let at = bytes
            .windows(6)
            .position(|w| w == b"before")
            .expect("the entry's bytes");
        let mut changed = bytes;
        changed[at..at + 6].copy_from_slice(b"AFTER!");
        std::fs::write(&path, &changed).expect("rewriting in place, same length");
        assert_eq!(
            archive.read_name("first").unwrap(),
            b"AFTER!",
            "the open archive must read the file, not a copy of it"
        );
    }
}
