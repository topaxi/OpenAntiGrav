//! The `0xca5caded` container three Vita formats share.
//!
//! `.rcsmodel`, `.rcsskeleton` and `.rcsanimclip` all open with the same
//! header: a magic, a section count, the header's own length, then one
//! 32-byte descriptor per section and one relocation table per section, in
//! descriptor order. The sections follow the header back to back and their
//! lengths add up to the file's own - `docs/formats/2048-rcsmodel.md`, "The
//! container". Factored out of [`super::parse`] the day the other two formats
//! were read, so the three readers share one header walk and one set of
//! refusals rather than three copies of them.
//!
//! **Section-local pointers are already offsets.** Every shipped file's
//! `link_base` is zero, so a pointer stored in a section is the offset of its
//! target within the section the relocation entry names - nothing here
//! simulates `RcsModel_Load`'s rebase, it just reads the word.

use super::{DESCRIPTOR_BASE, DESCRIPTOR_LEN, Error, MAGIC, RELOCATION_LEN, Result, Section};

/// The header of one container: its sections, each with its relocation table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    /// The sections the header declares, in order.
    pub sections: Vec<Section>,
}

impl Container {
    /// The bytes of section `index`, or `None` for a section the header does
    /// not declare.
    #[must_use]
    pub fn section<'a>(&self, file: &'a [u8], index: usize) -> Option<&'a [u8]> {
        let s = self.sections.get(index)?;
        file.get(s.at..s.at + s.len)
    }

    /// Every relocation site of section `index`: the section-local offsets of
    /// the words that hold a pointer, in file order.
    ///
    /// The second word of each 8-byte entry names which section the pointer
    /// targets; this returns the sites alone, which is all the three readers
    /// need - a site's target section is known from where the site is walked
    /// from.
    #[must_use]
    pub fn relocation_sites(&self, file: &[u8], index: usize) -> Vec<usize> {
        let Some(s) = self.sections.get(index) else {
            return Vec::new();
        };
        (0..s.entries)
            .filter_map(|e| {
                let at = s.table + e * RELOCATION_LEN;
                file.get(at..at + 4)
                    .map(|b| u32::from_le_bytes(b.try_into().expect("four bytes")) as usize)
            })
            .collect()
    }
}

/// Reads the header: magic, section count, descriptors, relocation tables.
///
/// # Errors
///
/// Refuses a wrong magic, a header or relocation table that runs past the end
/// of the file, and sections whose lengths do not add up to the file's own -
/// the same three refusals [`super::parse`] always made, now shared with the
/// skeleton and clip readers.
pub fn read(file: &[u8]) -> Result<Container> {
    let magic = u32_at(file, 0, "magic")?;
    if magic != MAGIC {
        return Err(Error::BadMagic { magic });
    }
    let section_count = u32_at(file, 0x08, "section count")? as usize;
    let header_len = u32_at(file, 0x0c, "header length")? as usize;
    let table_base = DESCRIPTOR_BASE + section_count * DESCRIPTOR_LEN;

    let mut sections = Vec::with_capacity(section_count.min(file.len() / DESCRIPTOR_LEN));
    let mut at = header_len;
    for i in 0..section_count {
        let d = DESCRIPTOR_BASE + i * DESCRIPTOR_LEN;
        let len = u32_at(file, d + 0x04, "section size")? as usize;
        let table = table_base + u32_at(file, d + 0x08, "relocation offset")? as usize;
        let entries = u32_at(file, d + 0x0c, "relocation count")? as usize;
        let end = table
            .checked_add(entries * RELOCATION_LEN)
            .ok_or(Error::OutOfBounds {
                what: "relocation table",
                end: usize::MAX,
                len: file.len(),
            })?;
        if end > file.len() {
            return Err(Error::OutOfBounds {
                what: "relocation table",
                end,
                len: file.len(),
            });
        }
        sections.push(Section {
            at,
            len,
            table,
            entries,
        });
        at = at.checked_add(len).ok_or(Error::OutOfBounds {
            what: "section",
            end: usize::MAX,
            len: file.len(),
        })?;
    }
    if at != file.len() {
        return Err(Error::SectionsDoNotClose {
            sections: at,
            len: file.len(),
        });
    }
    Ok(Container { sections })
}

/// A little-endian `u32` at `at`, or an [`Error::OutOfBounds`] naming `what`.
pub(crate) fn u32_at(file: &[u8], at: usize, what: &'static str) -> Result<u32> {
    file.get(at..at + 4)
        .map(|b| u32::from_le_bytes(b.try_into().expect("four bytes")))
        .ok_or(Error::OutOfBounds {
            what,
            end: at + 4,
            len: file.len(),
        })
}

/// Wraps one section into a container image, for a test that builds a file
/// by hand: the header, one descriptor, its relocation table, the section.
#[cfg(test)]
pub(crate) fn wrap_section(section: &[u8], relocations: &[u32]) -> Vec<u8> {
    let table_base = DESCRIPTOR_BASE + DESCRIPTOR_LEN;
    let header_len = table_base + relocations.len() * RELOCATION_LEN;
    let mut out = vec![0u8; header_len];
    out[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    out[8..12].copy_from_slice(&1u32.to_le_bytes());
    out[12..16].copy_from_slice(&(header_len as u32).to_le_bytes());
    let d = DESCRIPTOR_BASE;
    out[d + 4..d + 8].copy_from_slice(&(section.len() as u32).to_le_bytes());
    out[d + 12..d + 16].copy_from_slice(&(relocations.len() as u32).to_le_bytes());
    for (i, &site) in relocations.iter().enumerate() {
        let at = table_base + i * RELOCATION_LEN;
        out[at..at + 4].copy_from_slice(&site.to_le_bytes());
    }
    out.extend_from_slice(section);
    out
}
