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

use crate::{Error, Result};

/// The first four bytes of a sparse archive.
pub const MAGIC: [u8; 4] = *b"OAGX";

const VERSION: u32 = 1;
const HEADER: usize = 20;

/// An archive's recorded ranges, parsed.
#[derive(Debug)]
pub struct Sparse {
    len: u64,
    /// `(offset, len, start in data)`, ascending by offset.
    ranges: Vec<(u64, u64, usize)>,
    data: Vec<u8>,
}

impl Sparse {
    /// Whether `head` (a file's first bytes) is a sparse archive.
    #[must_use]
    pub fn is_sparse(head: &[u8]) -> bool {
        head.starts_with(&MAGIC)
    }

    /// Parses a whole sparse file.
    ///
    /// # Errors
    ///
    /// [`Error::BadSparse`] on a file that is cut short, has another version, or whose
    /// ranges run past the archive's length or overlap.
    pub fn parse(label: &str, bytes: Vec<u8>) -> Result<Self> {
        let bad = |why: &str| Error::BadSparse {
            archive: label.to_string(),
            why: why.to_string(),
        };
        if bytes.len() < HEADER || !Self::is_sparse(&bytes) {
            return Err(bad("not a sparse archive"));
        }
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4 bytes"));
        let wide = |at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().expect("8 bytes"));
        if word(4) != VERSION {
            return Err(bad("another version"));
        }
        let len = wide(8);
        let count = word(16) as usize;
        let table_end = count
            .checked_mul(16)
            .and_then(|table| table.checked_add(HEADER))
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| bad("the range table is cut short"))?;
        let mut ranges = Vec::with_capacity(count);
        let (mut at_data, mut previous_end) = (0usize, 0u64);
        for n in 0..count {
            let (offset, size) = (wide(HEADER + 16 * n), wide(HEADER + 16 * n + 8));
            let end = offset.checked_add(size).filter(|end| *end <= len);
            let Some(end) = end.filter(|_| offset >= previous_end) else {
                return Err(bad("a range overlaps another or runs past the archive"));
            };
            ranges.push((offset, size, at_data));
            at_data += usize::try_from(size).map_err(|_| bad("a range is too large"))?;
            previous_end = end;
        }
        if table_end + at_data != bytes.len() {
            return Err(bad("the data does not match the range table"));
        }
        let data = bytes[table_end..].to_vec();
        Ok(Self { len, ranges, data })
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

    /// `len` bytes at `offset`, when one recorded range holds them all.
    #[must_use]
    pub fn read(&self, offset: u64, len: u64) -> Option<&[u8]> {
        if len == 0 {
            return Some(&[]);
        }
        let end = offset.checked_add(len)?;
        let at = self.ranges.partition_point(|range| range.0 <= offset);
        let &(start, size, in_data) = self.ranges.get(at.checked_sub(1)?)?;
        if end > start + size {
            return None;
        }
        let from = in_data + usize::try_from(offset - start).ok()?;
        self.data.get(from..from + usize::try_from(len).ok()?)
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
    use super::*;

    fn built() -> Sparse {
        let whole: Vec<u8> = (0..=99).collect();
        let ranges = [(10, 5), (40, 10)];
        let bytes = write(100, &ranges, |offset, len| {
            Ok(whole[offset as usize..(offset + len) as usize].to_vec())
        })
        .expect("writes");
        Sparse::parse("t", bytes).expect("parses")
    }

    #[test]
    fn a_recorded_range_reads_back_and_the_rest_is_refused() {
        let sparse = built();
        assert_eq!(sparse.len(), 100);
        assert_eq!(sparse.present(), 15);
        assert_eq!(sparse.read(12, 3), Some(&[12u8, 13, 14][..]));
        assert_eq!(sparse.read(40, 10).map(<[u8]>::len), Some(10));
        assert_eq!(sparse.read(9, 2), None, "starts before a range");
        assert_eq!(sparse.read(14, 2), None, "runs past a range");
        assert_eq!(sparse.read(0, 1), None, "in a hole");
        assert_eq!(
            sparse.read(77, 0),
            Some(&[][..]),
            "nothing asked is nothing read"
        );
    }

    #[test]
    fn a_truncated_file_is_refused_by_name() {
        let mut bytes = write(100, &[(0, 4)], |_, len| Ok(vec![7; len as usize])).expect("writes");
        bytes.pop();
        let error = Sparse::parse("Data.wad", bytes).expect_err("cut short");
        assert!(error.to_string().contains("Data.wad"), "{error}");
    }
}
