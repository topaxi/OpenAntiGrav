//! Reading a Vita `.vpk` in place.
//!
//! A `.vpk` is a ZIP of an installed game's folder. The community's NoNpDrm
//! dumps are made by copying the folder off a console, where the file system
//! driver has already removed the PFS layer, so the files in one are plain: the
//! archives, `sce_sys/param.sfo`, and `sce_sys/package/work.bin` beside an
//! `eboot.bin` that is still an NpDrm SELF. This engine reads none of the SELF,
//! so it needs none of the licence either.
//!
//! # What is read
//!
//! The central directory (ZIP64 included), then any range of an entry:
//!
//! - **Stored** entries are read at random, which is what a multi-gigabyte
//!   `data.psarc` needs.
//! - **Deflated** entries are inflated whole into memory and the last one kept,
//!   up to [`INFLATE_LIMIT`]. A deflated entry past the limit is refused by
//!   name, never read wrong. Re-pack it stored (`zip -0`) to read it.
//! - Encrypted entries and other compression methods are refused by name.
//!
//! # One title, several `.vpk` files
//!
//! A patch and a DLC pack ship as `.vpk` files of their own, with the same
//! `TITLE_ID` as the base. [`VpkSet::open`] unions every `.vpk` beside the one
//! named whose `param.sfo` carries the same `TITLE_ID`, each file's paths
//! prefixed by its own file stem (`base/PSP2/data.psarc`,
//! `patch/PSP2/data1.psarc`), so a title's archive names - which differ between
//! base, patch and DLC - resolve by suffix the way they do in an extracted
//! folder.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::package::{PackageFile, PackageSource};

/// The ZIP local-file magic every `.vpk` begins with.
pub const MAGIC: &[u8; 4] = b"PK\x03\x04";

/// The largest deflated entry this reads, inflated in memory.
pub const INFLATE_LIMIT: u64 = 1 << 30;

/// How many inflated bytes are kept across entries. A race reads a patch's
/// `data1` and `data2` interleaved with the base, so a one-entry cache would
/// inflate a hundred megabytes on every switch.
pub const CACHE_BUDGET: usize = 512 << 20;

const EOCD: u32 = 0x0605_4b50;
const EOCD64: u32 = 0x0606_4b50;
const EOCD64_LOCATOR: u32 = 0x0706_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const LOCAL: u32 = 0x0403_4b50;

#[derive(Debug, Clone)]
struct ZipEntry {
    name: String,
    method: u16,
    packed: u64,
    size: u64,
    local: u64,
}

#[derive(Debug)]
struct Member {
    path: PathBuf,
    file: File,
    entries: Vec<ZipEntry>,
}

/// One or more `.vpk` files of a title, read in place.
#[derive(Debug)]
pub struct VpkSet {
    members: Vec<Member>,
    files: Vec<PackageFile>,
    at: Vec<(usize, usize)>,
}

/// Inflated deflate entries, least recently used first, keyed by archive path
/// and the entry's local-header offset, within [`CACHE_BUDGET`] bytes in all (one
/// entry is always kept).
///
/// **Process-wide, not per set**: a race opens the same `.vpk` through several
/// readers (the archives, the sound banks, the boot movies), and a per-set cache
/// inflated a hundred-megabyte patch archive once for each of them.
type InflateCache = Vec<((PathBuf, u64), std::sync::Arc<Vec<u8>>)>;
static INFLATED: std::sync::Mutex<InflateCache> = std::sync::Mutex::new(Vec::new());

fn bad(path: &Path, reason: impl Into<String>) -> Error {
    Error::Package {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

fn le16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}
fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}
fn le64(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

impl Member {
    fn open(path: &Path) -> Result<Self> {
        let mut file = File::open(path).map_err(|e| Error::io(path, e))?;
        let len = file.metadata().map_err(|e| Error::io(path, e))?.len();
        let entries = read_central_directory(&mut file, len, path)?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
            entries,
        })
    }

    fn read_at(&mut self, offset: u64, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        self.file
            .seek(SeekFrom::Start(offset))
            .and_then(|_| self.file.read_exact(&mut buf))
            .map_err(|e| Error::io(&self.path, e))?;
        Ok(buf)
    }

    /// Where an entry's bytes start: past its local header, whose name and extra
    /// lengths are not the central directory's to know.
    fn data_start(&mut self, entry: &ZipEntry) -> Result<u64> {
        let head = self.read_at(entry.local, 30)?;
        if le32(&head, 0) != Some(LOCAL) {
            return Err(bad(
                &self.path,
                format!("no local header for {}", entry.name),
            ));
        }
        let (name, extra) = (le16(&head, 26).unwrap_or(0), le16(&head, 28).unwrap_or(0));
        Ok(entry.local + 30 + u64::from(name) + u64::from(extra))
    }
}

fn read_central_directory(file: &mut File, len: u64, path: &Path) -> Result<Vec<ZipEntry>> {
    let tail_len = len.min(65_557);
    file.seek(SeekFrom::Start(len - tail_len))
        .map_err(|e| Error::io(path, e))?;
    let mut tail = vec![0u8; tail_len as usize];
    file.read_exact(&mut tail).map_err(|e| Error::io(path, e))?;
    let eocd = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|&i| le32(&tail, i) == Some(EOCD))
        .ok_or_else(|| bad(path, "not a ZIP: no end-of-central-directory record"))?;
    let mut count = u64::from(le16(&tail, eocd + 10).unwrap_or(0));
    let mut size = u64::from(le32(&tail, eocd + 12).unwrap_or(0));
    let mut offset = u64::from(le32(&tail, eocd + 16).unwrap_or(0));

    if (count == 0xFFFF || size == 0xFFFF_FFFF || offset == 0xFFFF_FFFF) && eocd >= 20 {
        if le32(&tail, eocd - 20) != Some(EOCD64_LOCATOR) {
            return Err(bad(path, "ZIP64 record is missing its locator"));
        }
        let at = le64(&tail, eocd - 20 + 8).unwrap_or(0);
        file.seek(SeekFrom::Start(at))
            .map_err(|e| Error::io(path, e))?;
        let mut rec = [0u8; 56];
        file.read_exact(&mut rec).map_err(|e| Error::io(path, e))?;
        if le32(&rec, 0) != Some(EOCD64) {
            return Err(bad(path, "ZIP64 end-of-central-directory record not found"));
        }
        count = le64(&rec, 32).unwrap_or(0);
        size = le64(&rec, 40).unwrap_or(0);
        offset = le64(&rec, 48).unwrap_or(0);
    }
    if size > len || offset > len || count > 4_000_000 {
        return Err(bad(path, "central directory points outside the file"));
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| Error::io(path, e))?;
    let mut dir = vec![0u8; size as usize];
    file.read_exact(&mut dir).map_err(|e| Error::io(path, e))?;

    let mut entries = Vec::new();
    let mut at = 0usize;
    for _ in 0..count {
        if le32(&dir, at) != Some(CENTRAL) {
            return Err(bad(path, "central directory entry has no signature"));
        }
        let flags = le16(&dir, at + 8).unwrap_or(0);
        let method = le16(&dir, at + 10).unwrap_or(0);
        let mut packed = u64::from(le32(&dir, at + 20).unwrap_or(0));
        let mut usize_ = u64::from(le32(&dir, at + 24).unwrap_or(0));
        let (n, x, c) = (
            usize::from(le16(&dir, at + 28).unwrap_or(0)),
            usize::from(le16(&dir, at + 30).unwrap_or(0)),
            usize::from(le16(&dir, at + 32).unwrap_or(0)),
        );
        let mut local = u64::from(le32(&dir, at + 42).unwrap_or(0));
        let name_bytes = dir
            .get(at + 46..at + 46 + n)
            .ok_or_else(|| bad(path, "entry name runs past the directory"))?;
        let name = String::from_utf8_lossy(name_bytes).replace('\\', "/");
        let extra = dir.get(at + 46 + n..at + 46 + n + x).unwrap_or(&[]);
        let mut e = 0usize;
        while e + 4 <= extra.len() {
            let (id, sz) = (
                le16(extra, e).unwrap_or(0),
                usize::from(le16(extra, e + 2).unwrap_or(0)),
            );
            if id == 1 {
                // ZIP64 carries, in this order, only the fields whose 32-bit
                // slot was `0xFFFFFFFF`.
                let (want_size, want_packed, want_local) = (
                    usize_ == 0xFFFF_FFFF,
                    packed == 0xFFFF_FFFF,
                    local == 0xFFFF_FFFF,
                );
                let mut p = e + 4;
                if want_size {
                    usize_ = le64(extra, p).unwrap_or(usize_);
                    p += 8;
                }
                if want_packed {
                    packed = le64(extra, p).unwrap_or(packed);
                    p += 8;
                }
                if want_local {
                    local = le64(extra, p).unwrap_or(local);
                }
            }
            e += 4 + sz;
        }
        at += 46 + n + x + c;
        if name.ends_with('/') {
            continue;
        }
        if flags & 1 != 0 {
            return Err(bad(path, format!("{name} is encrypted; a .vpk is plain")));
        }
        entries.push(ZipEntry {
            name,
            method,
            packed,
            size: usize_,
            local,
        });
    }
    Ok(entries)
}

/// The `CATEGORY` of a `.vpk`'s `param.sfo`: `gd` for an application, `gp` for
/// a patch, `ac` for downloadable content. `None` if it cannot be read.
///
/// What the chooser uses to list one row per title: a patch or DLC `.vpk` is
/// opened with its base ([`VpkSet::open`]), never as a row of its own.
#[must_use]
pub fn category(path: &Path) -> Option<String> {
    let mut member = Member::open(path).ok()?;
    let entry = member
        .entries
        .iter()
        .find(|e| e.name.eq_ignore_ascii_case("sce_sys/param.sfo"))?
        .clone();
    let bytes = read_entry(&mut member, &entry, 0, 1 << 16).ok()?;
    crate::sfo::text_field(&bytes, "CATEGORY")
}

impl VpkSet {
    /// Opens `path` and every `.vpk` beside it with the same `TITLE_ID`.
    pub fn open(path: &Path) -> Result<Self> {
        let mut first = Member::open(path)?;
        let title = title_of(&mut first);
        let mut members = vec![first];
        if let (Some(title), Some(dir)) = (title.as_deref(), path.parent()) {
            let mut siblings: Vec<PathBuf> = std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .filter_map(std::result::Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    p != path
                        && p.is_file()
                        && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("vpk"))
                })
                .collect();
            siblings.sort();
            for sibling in siblings {
                let Ok(mut member) = Member::open(&sibling) else {
                    continue;
                };
                if title_of(&mut member).as_deref() == Some(title) {
                    members.push(member);
                }
            }
        }
        let mut files = Vec::new();
        let mut at = Vec::new();
        for (m, member) in members.iter().enumerate() {
            let stem = member
                .path
                .file_stem()
                .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
            for (e, entry) in member.entries.iter().enumerate() {
                files.push(PackageFile {
                    path: format!("{stem}/{}", entry.name),
                    size: entry.size,
                });
                at.push((m, e));
            }
        }
        Ok(Self { members, files, at })
    }
}

/// The `TITLE_ID` of a member's `sce_sys/param.sfo`, normalised.
fn title_of(member: &mut Member) -> Option<String> {
    let entry = member
        .entries
        .iter()
        .find(|e| e.name.eq_ignore_ascii_case("sce_sys/param.sfo"))?
        .clone();
    let bytes = read_entry(member, &entry, 0, 1 << 16).ok()?;
    crate::sfo::title_id(&bytes)
}

fn read_entry(member: &mut Member, entry: &ZipEntry, offset: u64, len: u64) -> Result<Vec<u8>> {
    if offset >= entry.size {
        return Ok(Vec::new());
    }
    let len = len.min(entry.size - offset);
    match entry.method {
        0 => {
            let start = member.data_start(entry)?;
            member.read_at(start + offset, len as usize)
        }
        8 => {
            if entry.size > INFLATE_LIMIT {
                return Err(bad(
                    &member.path,
                    format!(
                        "{} is deflated and {} bytes: too large to inflate in memory (limit {INFLATE_LIMIT}); re-pack it stored",
                        entry.name, entry.size
                    ),
                ));
            }
            let all = inflate(member, entry)?;
            Ok(all[offset as usize..(offset + len) as usize].to_vec())
        }
        m => Err(bad(
            &member.path,
            format!(
                "{} uses ZIP method {m}; only stored and deflate are read",
                entry.name
            ),
        )),
    }
}

fn inflate(member: &mut Member, entry: &ZipEntry) -> Result<Vec<u8>> {
    let start = member.data_start(entry)?;
    let packed = member.read_at(start, entry.packed as usize)?;
    let out = miniz_oxide::inflate::decompress_to_vec_with_limit(&packed, entry.size as usize)
        .map_err(|e| {
            bad(
                &member.path,
                format!("{}: inflate failed: {:?}", entry.name, e.status),
            )
        })?;
    if out.len() as u64 != entry.size {
        return Err(bad(
            &member.path,
            format!(
                "{} inflated to {} bytes, expected {}",
                entry.name,
                out.len(),
                entry.size
            ),
        ));
    }
    Ok(out)
}

impl PackageSource for VpkSet {
    fn files(&self) -> &[PackageFile] {
        &self.files
    }

    fn read(&mut self, index: usize, offset: u64, len: u64) -> Result<Vec<u8>> {
        let &(m, e) = self.at.get(index).ok_or_else(|| Error::NotFound {
            path: format!("package file #{index}"),
        })?;
        let entry = self.members[m].entries[e].clone();
        if entry.method == 8 && entry.size <= INFLATE_LIMIT {
            if offset >= entry.size {
                return Ok(Vec::new());
            }
            let key = (self.members[m].path.clone(), entry.local);
            let cached = {
                let mut cache = INFLATED.lock().expect("the inflate cache lock");
                cache.iter().position(|(k, _)| *k == key).map(|at| {
                    let hit = cache.remove(at);
                    cache.push(hit);
                    std::sync::Arc::clone(&cache.last().expect("just pushed").1)
                })
            };
            let data = match cached {
                Some(data) => data,
                None => {
                    // Inflated outside the lock: another reader of a different
                    // entry is not made to wait for this one.
                    let all = std::sync::Arc::new(inflate(&mut self.members[m], &entry)?);
                    let mut cache = INFLATED.lock().expect("the inflate cache lock");
                    cache.push((key, std::sync::Arc::clone(&all)));
                    while cache.len() > 1
                        && cache.iter().map(|(_, d)| d.len()).sum::<usize>() > CACHE_BUDGET
                    {
                        cache.remove(0);
                    }
                    all
                }
            };
            let end = (offset + len).min(entry.size);
            return Ok(data[offset as usize..end as usize].to_vec());
        }
        read_entry(&mut self.members[m], &entry, offset, len)
    }
}

#[cfg(test)]
mod tests;
