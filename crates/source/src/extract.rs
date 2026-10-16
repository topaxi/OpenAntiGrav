//! A server extraction: the slice of a title's source that a server's races read.
//!
//! A headless server needs the title's data to run a race and nothing else, and
//! a remote host should not hold a whole disc to do it. An extraction is a
//! folder laid out like the source it came from (the same relative paths) in
//! which each archive is a sparse stand-in holding only the byte ranges a recorded
//! load read ([`oag_assets::sparse`]), the small identity files that tell the
//! source's console apart are copied whole, and [`MANIFEST`] names what it is.
//! [`crate::title::open_source`] opens it like any folder source, as the title the
//! manifest names.
//!
//! This module is the mechanism: the manifest, and writing the files of a
//! [`oag_assets::record::Recording`]. Choosing which races to record is the
//! extractor's job (`oag-server extract`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use oag_assets::record::Recording;
use oag_disc::DiscImage;

/// The manifest's file name, at the extraction's root.
pub const MANIFEST: &str = "oag-extract.txt";

/// Files copied whole so a folder source identifies its console: the PSP's and PS2's
/// identity files and the package parameter file. Matched on the last path component.
const IDENTITY: [&str; 3] = ["UMD_DATA.BIN", "SYSTEM.CNF", "PARAM.SFO"];

/// The most an identity file may be to be copied.
const IDENTITY_MAX: u64 = 1 << 20;

/// One file of an extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// `sparse` (an archive's recorded ranges) or `whole`.
    pub kind: String,
    /// The path under the extraction, `/` separated.
    pub path: String,
    /// The original's length in bytes.
    pub len: u64,
    /// Bytes the extraction holds of it.
    pub kept: u64,
    /// SHA-256 of the file as written, lowercase hex.
    pub sha256: String,
}

/// What an extraction says about itself.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Manifest {
    /// `Title::name` of the source.
    pub title: String,
    /// `Platform`'s display.
    pub platform: String,
    /// The source's serial, empty when it has none.
    pub serial: String,
    /// The source's file or folder name (never a full path).
    pub source: String,
    /// SHA-256 of the source image file, or `folder` for a folder source.
    pub source_sha256: String,
    /// The extractor's build string (`oag-server`'s), so two extractions diff.
    pub extractor: String,
    /// What was recorded to make it, a line each (the races swept).
    pub swept: Vec<String>,
    /// Every file.
    pub files: Vec<Row>,
}

impl Manifest {
    /// The manifest as text: `key<TAB>value` lines, then one `file` row per file.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = String::from("oag-extract\t1\n");
        for (key, value) in [
            ("title", &self.title),
            ("platform", &self.platform),
            ("serial", &self.serial),
            ("source", &self.source),
            ("source-sha256", &self.source_sha256),
            ("extractor", &self.extractor),
        ] {
            out.push_str(&format!("{key}\t{value}\n"));
        }
        for line in &self.swept {
            out.push_str(&format!("swept\t{line}\n"));
        }
        for row in &self.files {
            out.push_str(&format!(
                "file\t{}\t{}\t{}\t{}\t{}\n",
                row.kind, row.path, row.len, row.kept, row.sha256
            ));
        }
        out
    }

    /// Reads a manifest written by [`Self::to_text`].
    ///
    /// # Errors
    ///
    /// A first line that is not this format's, or a file row that is cut short.
    pub fn parse(text: &str) -> Result<Self> {
        let mut lines = text.lines();
        if lines.next() != Some("oag-extract\t1") {
            return Err(anyhow!("not an oag-extract version 1 manifest"));
        }
        let mut manifest = Self::default();
        for line in lines {
            let fields: Vec<&str> = line.split('\t').collect();
            match fields.as_slice() {
                ["title", v] => manifest.title = (*v).to_string(),
                ["platform", v] => manifest.platform = (*v).to_string(),
                ["serial", v] => manifest.serial = (*v).to_string(),
                ["source", v] => manifest.source = (*v).to_string(),
                ["source-sha256", v] => manifest.source_sha256 = (*v).to_string(),
                ["extractor", v] => manifest.extractor = (*v).to_string(),
                ["swept", v] => manifest.swept.push((*v).to_string()),
                ["file", kind, path, len, kept, sha] => manifest.files.push(Row {
                    kind: (*kind).to_string(),
                    path: (*path).to_string(),
                    len: len.parse().context("a file row's length")?,
                    kept: kept.parse().context("a file row's kept bytes")?,
                    sha256: (*sha).to_string(),
                }),
                [_, ..] if line.is_empty() => {}
                _ => return Err(anyhow!("unreadable manifest line {line:?}")),
            }
        }
        Ok(manifest)
    }

    /// The manifest of the extraction at `dir`, `None` when it is not one.
    #[must_use]
    pub fn of(dir: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(dir.join(MANIFEST)).ok()?;
        Self::parse(&text).ok()
    }
}

/// The title a source names when it is a server extraction, for
/// [`crate::title::open_source`] to open it as.
#[must_use]
pub fn title_named(source: &str) -> Option<String> {
    Manifest::of(Path::new(source)).map(|manifest| manifest.title)
}

/// The files `recording` saw, written under `out` as sparse archives and whole
/// loose files, plus the source's identity files. Reads the bytes from `source` (an image or
/// a folder). Returns what it wrote, without hashes: the caller hashes the files.
///
/// # Errors
///
/// A source that will not open, a recorded path the source lacks, or `out` not writable.
pub fn write_files(source: &Path, recording: &Recording, out: &Path) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    let mut disc = if source.is_dir() {
        None
    } else {
        Some(DiscImage::open(source).with_context(|| format!("opening {}", source.display()))?)
    };
    let entries = match disc.as_mut() {
        Some(disc) => disc.entries()?.to_vec(),
        None => Vec::new(),
    };
    for file in &recording.files {
        let mut read = |offset: u64, len: u64| -> oag_assets::Result<Vec<u8>> {
            Ok(match disc.as_mut() {
                Some(disc) => {
                    let entry = entries
                        .iter()
                        .find(|entry| entry.path == file.path)
                        .ok_or_else(|| oag_assets::Error::NotOnDisc {
                            image: source.display().to_string(),
                            path: file.path.clone(),
                        })?;
                    disc.read_entry_range(entry, offset, len)?
                }
                None => read_range(&source.join(&file.path), offset, len)?,
            })
        };
        let bytes = oag_assets::sparse::write(file.len, &file.ranges, &mut read)
            .with_context(|| format!("extracting {}", file.path))?;
        put(out, &file.path, &bytes)?;
        rows.push(Row {
            kind: "sparse".into(),
            path: file.path.clone(),
            len: file.len,
            kept: file.ranges.iter().map(|range| range.1).sum(),
            sha256: String::new(),
        });
    }
    let mut whole: Vec<String> = recording.loose.clone();
    whole.extend(identity_files(source, &entries, disc.is_some()));
    whole.sort();
    whole.dedup();
    for path in whole {
        let bytes = match disc.as_mut() {
            Some(disc) => {
                let entry = entries
                    .iter()
                    .find(|entry| entry.path == path)
                    .ok_or_else(|| anyhow!("{path} is not on {}", source.display()))?;
                disc.read_entry(entry)?
            }
            None => std::fs::read(source.join(&path))?,
        };
        put(out, &path, &bytes)?;
        rows.push(Row {
            kind: "whole".into(),
            path,
            len: bytes.len() as u64,
            kept: bytes.len() as u64,
            sha256: String::new(),
        });
    }
    Ok(rows)
}

fn read_range(path: &Path, offset: u64, len: u64) -> oag_assets::Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; usize::try_from(len).unwrap_or(0)];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

fn put(out: &Path, path: &str, bytes: &[u8]) -> Result<()> {
    let target: PathBuf = out.join(path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target, bytes).with_context(|| format!("writing {}", target.display()))
}

/// The identity files of a source: those named in [`IDENTITY`], small enough to copy.
fn identity_files(source: &Path, entries: &[oag_disc::Entry], disc: bool) -> Vec<String> {
    let named = |path: &str| {
        let last = path.rsplit(['/', '\\']).next().unwrap_or(path);
        IDENTITY.iter().any(|name| last.eq_ignore_ascii_case(name))
    };
    if disc {
        return entries
            .iter()
            .filter(|entry| !entry.is_directory && entry.size <= IDENTITY_MAX && named(&entry.path))
            .map(|entry| entry.path.clone())
            .collect();
    }
    let mut found = Vec::new();
    walk(source, "", &mut found);
    found.retain(|path| named(path));
    found
}

fn walk(dir: &Path, prefix: &str, into: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => walk(&entry.path(), &relative, into),
            Ok(_) => into.push(relative),
            Err(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_reads_back_what_it_wrote() {
        let manifest = Manifest {
            title: "Wipeout Pulse".into(),
            platform: "PSP".into(),
            serial: "UCES-01417".into(),
            source: "pulse-psp-eu.chd".into(),
            source_sha256: "ab".into(),
            extractor: "0.1.0 (abc1234)".into(),
            swept: vec!["every circuit".into()],
            files: vec![Row {
                kind: "sparse".into(),
                path: "PSP_GAME/USRDIR/Data.wad".into(),
                len: 100,
                kept: 40,
                sha256: "cd".into(),
            }],
        };
        assert_eq!(
            Manifest::parse(&manifest.to_text()).expect("parses"),
            manifest
        );
        assert!(Manifest::parse("something else\n").is_err());
    }
}
