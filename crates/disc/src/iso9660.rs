//! A reader for the ISO 9660 subset used by UMD and DVD game images.
//!
//! Scope is deliberately narrow. Game discs use plain ISO 9660 with 8.3-style
//! names and a `;1` version suffix. Rock Ridge and Joliet extensions are not
//! interpreted: if a disc turns out to need them, that is a discovery worth
//! documenting rather than something to speculatively support now.
//!
//! Reference: ECMA-119, third edition (equivalent to ISO 9660).

use std::collections::HashSet;

use crate::error::{Error, Result};
use crate::source::{SECTOR_SIZE, SectorSource};

/// Volume descriptors begin here, after the 16-sector system area.
const VOLUME_DESCRIPTOR_START: u32 = 16;

/// How far to search for a primary volume descriptor before giving up.
const VOLUME_DESCRIPTOR_SEARCH_LIMIT: u32 = 64;

const DESCRIPTOR_TYPE_PRIMARY: u8 = 1;
const DESCRIPTOR_TYPE_TERMINATOR: u8 = 255;
const STANDARD_IDENTIFIER: &[u8; 5] = b"CD001";

const FLAG_DIRECTORY: u8 = 0x02;
const FLAG_NOT_FINAL_EXTENT: u8 = 0x80;

/// Guards against a directory record that points at an ancestor.
const MAX_DEPTH: u32 = 32;

/// One file or directory on the disc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Absolute path with `/` separators and no leading slash, version suffix
    /// stripped. For example `DATA/WIPEOUT.WAD`.
    pub path: String,
    /// First logical sector of the file's data.
    pub lba: u32,
    /// Size in bytes.
    pub size: u64,
    /// Whether this is a directory.
    pub is_directory: bool,
}

impl Entry {
    /// The final path component.
    #[must_use]
    pub fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// The lowercased extension, without the dot, if any.
    #[must_use]
    pub fn extension(&self) -> Option<String> {
        let name = self.name();
        let dot = name.rfind('.')?;
        // A leading dot is part of the name, not an extension marker.
        if dot == 0 {
            return None;
        }
        Some(name[dot + 1..].to_ascii_lowercase())
    }
}

/// Metadata from the primary volume descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeDescriptor {
    /// System identifier field.
    pub system_id: String,
    /// Volume identifier field, usually the disc label.
    pub volume_id: String,
    /// Publisher identifier field.
    pub publisher_id: String,
    /// Application identifier field.
    pub application_id: String,
    /// Total sectors the volume claims to occupy.
    pub volume_space_size: u32,
    /// Logical block size in bytes. Expected to be 2048.
    pub logical_block_size: u16,
    /// Volume creation timestamp, as the raw 17-byte field.
    pub created: String,
    /// First sector of the root directory.
    pub root_lba: u32,
    /// Size of the root directory in bytes.
    pub root_size: u32,
}

/// Reads the primary volume descriptor.
///
/// # Errors
///
/// Returns [`Error::NoPrimaryVolumeDescriptor`] if no PVD appears before the
/// descriptor set terminator or the search limit.
pub fn read_volume_descriptor(source: &mut dyn SectorSource) -> Result<VolumeDescriptor> {
    let mut sector = vec![0u8; SECTOR_SIZE];
    let limit = VOLUME_DESCRIPTOR_SEARCH_LIMIT.min(source.sector_count());

    for lba in VOLUME_DESCRIPTOR_START..limit {
        source.read_sector(lba, &mut sector)?;

        if &sector[1..6] != STANDARD_IDENTIFIER {
            continue;
        }
        match sector[0] {
            DESCRIPTOR_TYPE_PRIMARY => return parse_volume_descriptor(&sector, lba),
            DESCRIPTOR_TYPE_TERMINATOR => break,
            // Supplementary (Joliet) and other descriptor types are skipped.
            _ => continue,
        }
    }

    Err(Error::NoPrimaryVolumeDescriptor { searched: limit })
}

fn parse_volume_descriptor(sector: &[u8], lba: u32) -> Result<VolumeDescriptor> {
    let root_record = &sector[156..190];
    if root_record[0] == 0 {
        return Err(Error::MalformedFilesystem {
            sector: lba,
            reason: "root directory record has zero length".into(),
        });
    }

    Ok(VolumeDescriptor {
        system_id: strd(&sector[8..40]),
        volume_id: strd(&sector[40..72]),
        publisher_id: strd(&sector[318..446]),
        application_id: strd(&sector[574..702]),
        volume_space_size: both_endian_u32(&sector[80..88]),
        logical_block_size: both_endian_u16(&sector[128..132]),
        created: strd(&sector[813..830]),
        root_lba: both_endian_u32(&root_record[2..10]),
        root_size: both_endian_u32(&root_record[10..18]),
    })
}

/// Walks the whole directory tree, depth-first, and returns every entry.
///
/// Directories appear before their contents. `.` and `..` records are omitted.
pub fn walk(source: &mut dyn SectorSource) -> Result<Vec<Entry>> {
    let pvd = read_volume_descriptor(source)?;
    let mut entries = Vec::new();
    let mut visited = HashSet::new();

    walk_directory(
        source,
        pvd.root_lba,
        u64::from(pvd.root_size),
        "",
        0,
        &mut visited,
        &mut entries,
    )?;

    Ok(entries)
}

fn walk_directory(
    source: &mut dyn SectorSource,
    lba: u32,
    size: u64,
    prefix: &str,
    depth: u32,
    visited: &mut HashSet<u32>,
    out: &mut Vec<Entry>,
) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(Error::TooDeep { limit: MAX_DEPTH });
    }
    // A directory reachable twice means the tree is really a graph. Walking it
    // would either loop forever or duplicate a whole subtree.
    if !visited.insert(lba) {
        return Ok(());
    }

    let data = source.read_range(lba, size)?;
    let mut children = Vec::new();

    for record in DirectoryRecords::new(&data, lba) {
        let record = record?;

        // 0x00 is ".", 0x01 is "..". Both are structural, not real entries.
        if record.is_special {
            continue;
        }

        let path = if prefix.is_empty() {
            record.name.clone()
        } else {
            format!("{prefix}/{}", record.name)
        };

        out.push(Entry {
            path: path.clone(),
            lba: record.lba,
            size: record.size,
            is_directory: record.is_directory,
        });

        if record.is_directory {
            children.push((record.lba, record.size, path));
        }
    }

    for (child_lba, child_size, child_path) in children {
        walk_directory(
            source,
            child_lba,
            child_size,
            &child_path,
            depth + 1,
            visited,
            out,
        )?;
    }

    Ok(())
}

#[derive(Debug)]
struct RawRecord {
    name: String,
    lba: u32,
    size: u64,
    is_directory: bool,
    is_special: bool,
}

/// Iterates the directory records packed into a directory's data extent.
struct DirectoryRecords<'a> {
    data: &'a [u8],
    offset: usize,
    sector: u32,
    /// Set while accumulating the extents of a multi-extent file.
    pending: Option<RawRecord>,
}

impl<'a> DirectoryRecords<'a> {
    fn new(data: &'a [u8], sector: u32) -> Self {
        Self {
            data,
            offset: 0,
            sector,
            pending: None,
        }
    }
}

impl Iterator for DirectoryRecords<'_> {
    type Item = Result<RawRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.offset >= self.data.len() {
                // Flush a multi-extent file that ran to the end of the extent.
                return self.pending.take().map(Ok);
            }

            let record_len = self.data[self.offset] as usize;

            // A zero length means the rest of this sector is padding: records
            // may not straddle a sector boundary, so the next one starts at the
            // following boundary.
            if record_len == 0 {
                let next = (self.offset / SECTOR_SIZE + 1) * SECTOR_SIZE;
                if next <= self.offset {
                    return Some(Err(Error::MalformedFilesystem {
                        sector: self.sector,
                        reason: "directory record padding does not advance".into(),
                    }));
                }
                self.offset = next;
                continue;
            }

            if record_len < 33 || self.offset + record_len > self.data.len() {
                return Some(Err(Error::MalformedFilesystem {
                    sector: self.sector,
                    reason: format!(
                        "directory record length {record_len} at offset {} is out of bounds",
                        self.offset
                    ),
                }));
            }

            let record = &self.data[self.offset..self.offset + record_len];
            self.offset += record_len;

            let name_len = record[32] as usize;
            if 33 + name_len > record_len {
                return Some(Err(Error::MalformedFilesystem {
                    sector: self.sector,
                    reason: format!("file identifier length {name_len} overruns its record"),
                }));
            }

            let raw_name = &record[33..33 + name_len];
            let flags = record[25];
            let is_directory = flags & FLAG_DIRECTORY != 0;
            let is_special = is_directory && name_len == 1 && matches!(raw_name[0], 0 | 1);

            let parsed = RawRecord {
                name: decode_identifier(raw_name),
                lba: both_endian_u32(&record[2..10]),
                size: u64::from(both_endian_u32(&record[10..18])),
                is_directory,
                is_special,
            };

            // Files larger than 4 GiB, and some authoring quirks, split a file
            // across several consecutive records. Only the last has the
            // not-final bit clear. Join them so callers see one entry.
            match self.pending.take() {
                Some(mut acc) if acc.name == parsed.name => {
                    acc.size += parsed.size;
                    if flags & FLAG_NOT_FINAL_EXTENT != 0 {
                        self.pending = Some(acc);
                        continue;
                    }
                    return Some(Ok(acc));
                }
                Some(acc) => {
                    // Name changed mid-sequence: emit what we have and restart.
                    self.pending = Some(parsed);
                    return Some(Ok(acc));
                }
                None => {
                    if flags & FLAG_NOT_FINAL_EXTENT != 0 {
                        self.pending = Some(parsed);
                        continue;
                    }
                    return Some(Ok(parsed));
                }
            }
        }
    }
}

/// Decodes a file identifier and strips the `;N` version suffix.
fn decode_identifier(raw: &[u8]) -> String {
    let text: String = raw
        .iter()
        .map(|&b| if b.is_ascii() { b as char } else { '?' })
        .collect();

    match text.rsplit_once(';') {
        // Only strip when the suffix really is a version number.
        Some((name, version))
            if !version.is_empty() && version.bytes().all(|b| b.is_ascii_digit()) =>
        {
            // `NAME.;1` is a file with no extension; the trailing dot is noise.
            name.strip_suffix('.').unwrap_or(name).to_string()
        }
        _ => text,
    }
}

/// Reads a 32-bit both-endian field, taking the little-endian half.
fn both_endian_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Reads a 16-bit both-endian field, taking the little-endian half.
fn both_endian_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

/// Decodes a space-padded ISO 9660 `strD`/`strA` field.
fn strd(bytes: &[u8]) -> String {
    let text: String = bytes
        .iter()
        .map(|&b| if b.is_ascii() { b as char } else { '?' })
        .collect();
    text.trim_end_matches([' ', '\0']).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_version_suffix() {
        assert_eq!(decode_identifier(b"WIPEOUT.WAD;1"), "WIPEOUT.WAD");
        assert_eq!(decode_identifier(b"EBOOT.BIN;1"), "EBOOT.BIN");
        assert_eq!(decode_identifier(b"README.;1"), "README");
    }

    #[test]
    fn keeps_a_semicolon_that_is_not_a_version() {
        // Not a version suffix, so it must survive intact.
        assert_eq!(decode_identifier(b"ODD;NAME"), "ODD;NAME");
        assert_eq!(decode_identifier(b"TRAILING;"), "TRAILING;");
    }

    #[test]
    fn reads_both_endian_fields() {
        // 0x12345678 stored little-endian then big-endian.
        let field = [0x78, 0x56, 0x34, 0x12, 0x12, 0x34, 0x56, 0x78];
        assert_eq!(both_endian_u32(&field), 0x1234_5678);

        let field16 = [0x00, 0x08, 0x08, 0x00];
        assert_eq!(both_endian_u16(&field16), 2048);
    }

    #[test]
    fn trims_padded_text_fields() {
        assert_eq!(strd(b"PSP GAME                "), "PSP GAME");
        assert_eq!(strd(b"WIPEOUT\0\0\0"), "WIPEOUT");
        assert_eq!(strd(b"    "), "");
    }

    #[test]
    fn entry_reports_name_and_extension() {
        let e = Entry {
            path: "DATA/SHIPS/FEISAR.WAD".into(),
            lba: 100,
            size: 1234,
            is_directory: false,
        };
        assert_eq!(e.name(), "FEISAR.WAD");
        assert_eq!(e.extension().as_deref(), Some("wad"));

        let no_ext = Entry {
            path: "SYSTEM".into(),
            lba: 1,
            size: 0,
            is_directory: true,
        };
        assert_eq!(no_ext.extension(), None);
    }

    /// Builds a directory extent by hand so the record walker can be tested
    /// without a real disc image.
    fn make_record(name: &[u8], lba: u32, size: u32, flags: u8) -> Vec<u8> {
        let name_len = name.len();
        let mut len = 33 + name_len;
        if len % 2 == 1 {
            len += 1; // records are padded to an even length
        }

        let mut rec = vec![0u8; len];
        rec[0] = len as u8;
        rec[2..6].copy_from_slice(&lba.to_le_bytes());
        rec[6..10].copy_from_slice(&lba.to_be_bytes());
        rec[10..14].copy_from_slice(&size.to_le_bytes());
        rec[14..18].copy_from_slice(&size.to_be_bytes());
        rec[25] = flags;
        rec[32] = name_len as u8;
        rec[33..33 + name_len].copy_from_slice(name);
        rec
    }

    #[test]
    fn walks_records_and_skips_dot_entries() {
        let mut data = Vec::new();
        data.extend(make_record(&[0], 20, 2048, FLAG_DIRECTORY));
        data.extend(make_record(&[1], 20, 2048, FLAG_DIRECTORY));
        data.extend(make_record(b"EBOOT.BIN;1", 100, 4096, 0));
        data.extend(make_record(b"DATA", 200, 2048, FLAG_DIRECTORY));
        data.resize(SECTOR_SIZE, 0);

        let found: Vec<_> = DirectoryRecords::new(&data, 20)
            .map(|r| r.unwrap())
            .filter(|r| !r.is_special)
            .collect();

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "EBOOT.BIN");
        assert_eq!(found[0].lba, 100);
        assert_eq!(found[0].size, 4096);
        assert!(!found[0].is_directory);
        assert_eq!(found[1].name, "DATA");
        assert!(found[1].is_directory);
    }

    #[test]
    fn joins_multi_extent_records() {
        let mut data = Vec::new();
        data.extend(make_record(b"BIG.PAK;1", 100, 4096, FLAG_NOT_FINAL_EXTENT));
        data.extend(make_record(b"BIG.PAK;1", 102, 1024, 0));
        data.resize(SECTOR_SIZE, 0);

        let found: Vec<_> = DirectoryRecords::new(&data, 20)
            .map(|r| r.unwrap())
            .collect();

        assert_eq!(found.len(), 1, "the two extents are one file");
        assert_eq!(found[0].name, "BIG.PAK");
        assert_eq!(found[0].size, 4096 + 1024);
        assert_eq!(found[0].lba, 100, "the entry points at the first extent");
    }

    #[test]
    fn skips_padding_to_the_next_sector() {
        let mut data = vec![0u8; SECTOR_SIZE * 2];
        let first = make_record(b"A.BIN;1", 10, 1, 0);
        data[..first.len()].copy_from_slice(&first);
        // Rest of sector 0 is zero padding.
        let second = make_record(b"B.BIN;1", 11, 2, 0);
        data[SECTOR_SIZE..SECTOR_SIZE + second.len()].copy_from_slice(&second);

        let found: Vec<_> = DirectoryRecords::new(&data, 20)
            .map(|r| r.unwrap())
            .collect();

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "A.BIN");
        assert_eq!(found[1].name, "B.BIN");
    }

    #[test]
    fn rejects_a_record_that_overruns_the_extent() {
        let mut data = vec![0u8; 64];
        data[0] = 200; // claims to be longer than the buffer

        let first = DirectoryRecords::new(&data, 20).next().unwrap();
        assert!(matches!(first, Err(Error::MalformedFilesystem { .. })));
    }

    #[test]
    fn rejects_an_identifier_that_overruns_its_record() {
        let mut rec = make_record(b"OK.BIN;1", 1, 1, 0);
        rec[32] = 200; // name longer than the record

        let first = DirectoryRecords::new(&rec, 20).next().unwrap();
        assert!(matches!(first, Err(Error::MalformedFilesystem { .. })));
    }
}
