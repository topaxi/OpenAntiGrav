//! One WAD archive, on the filesystem or inside a disc image.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};

use crate::{Error, Result};

/// Where an archive's bytes come from.
///
/// Both variants read lazily. The directory is a few kilobytes; the blobs are
/// not, and on a CHD every read costs decompression, so slurping the archive
/// would be several minutes wasted on a 315 MiB `Data.wad`.
enum Source {
    File {
        file: std::fs::File,
        len: u64,
    },
    Disc {
        disc: Box<DiscImage>,
        entry: oag_disc::Entry,
    },
}

/// A WAD archive with its directory parsed.
pub struct Archive {
    source: Source,
    label: String,
    directory: Directory,
}

impl std::fmt::Debug for Archive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Archive")
            .field("label", &self.label)
            .field("entries", &self.directory.entries.len())
            .finish()
    }
}

impl Archive {
    /// Opens `spec`, either a path to a `.wad` or `<image>:<path-on-disc>`.
    ///
    /// Splitting on the last colon rather than the first keeps Windows drive
    /// letters working.
    pub fn open(spec: &str) -> Result<Self> {
        let mut source = Self::open_source(spec)?;
        let label = spec.to_string();

        let header = source.read(0, wad::HEADER_LEN as u64)?;
        let count = Directory::peek_entry_count(&header).map_err(|source| Error::BadDirectory {
            archive: label.clone(),
            source,
        })?;

        let bytes = source.read(0, Directory::directory_len(count))?;
        let directory =
            Directory::parse(&bytes, Some(source.len())).map_err(|source| Error::BadDirectory {
                archive: label.clone(),
                source,
            })?;

        Ok(Self {
            source,
            label,
            directory,
        })
    }

    fn open_source(spec: &str) -> Result<Source> {
        if let Some((image, inner)) = split_disc_spec(spec) {
            let mut disc = DiscImage::open(image)?;
            let entry = disc
                .entries()?
                .iter()
                .find(|e| !e.is_directory && e.path.eq_ignore_ascii_case(inner))
                .cloned()
                .ok_or_else(|| Error::NotOnDisc {
                    image: image.to_string(),
                    path: inner.to_string(),
                })?;
            return Ok(Source::Disc {
                disc: Box::new(disc),
                entry,
            });
        }

        let path = PathBuf::from(spec);
        if path.extension().is_none() {
            // A bare word is far more likely to be a mistyped disc spec than a
            // real file, and the error is much more useful this way.
            return Err(Error::BadSpec(spec.to_string()));
        }
        let file = std::fs::File::open(&path)?;
        let len = file.metadata()?.len();
        Ok(Source::File { file, len })
    }

    /// How this archive was named, for error messages.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Total size of the archive itself: the file's length, or the disc
    /// entry's.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.source.len()
    }

    /// Whether the archive is empty. Always `false`: a zero-length file
    /// would have failed to parse a directory in [`Archive::open`].
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The parsed directory.
    #[must_use]
    pub fn directory(&self) -> &Directory {
        &self.directory
    }

    /// Index of the entry with this name hash.
    #[must_use]
    pub fn index_of_hash(&self, hash: u32) -> Option<usize> {
        self.directory
            .entries
            .iter()
            .position(|e| e.name_hash == hash)
    }

    /// Index of the entry with this name.
    #[must_use]
    pub fn index_of_name(&self, name: &str) -> Option<usize> {
        self.index_of_hash(wad::hash_name(name))
    }

    /// Whether an entry with this name exists.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.index_of_name(name).is_some()
    }

    /// Reads and decompresses entry `index`.
    pub fn read(&mut self, index: usize) -> Result<Vec<u8>> {
        let entry = *self.entry(index)?;
        let raw = self
            .source
            .read(u64::from(entry.offset), u64::from(entry.size))?;
        self.decode(index, &entry, raw)
    }

    /// Reads entry `index` exactly as stored, without decompressing.
    ///
    /// For diagnostics that need the compressed bytes themselves, such as
    /// checking where an LZSS stream's reader actually stopped.
    pub fn read_raw(&mut self, index: usize) -> Result<Vec<u8>> {
        let entry = *self.entry(index)?;
        self.source
            .read(u64::from(entry.offset), u64::from(entry.size))
    }

    /// Reads the first `len` bytes of entry `index` exactly as stored, never
    /// decompressing and never clamping to the entry's own size.
    ///
    /// Unlike [`Archive::peek`], this never falls back to a full decompressed
    /// read for a compressed entry: it is for callers that specifically want
    /// the stored bytes, such as inspecting a type tag that lives at the
    /// start of the compressed stream. Reading past a short entry into the
    /// next blob's padding is deliberate, not a bug: it matches what a naive
    /// fixed-size peek at a raw offset actually sees.
    pub fn peek_raw(&mut self, index: usize, len: u64) -> Result<Vec<u8>> {
        let entry = *self.entry(index)?;
        self.source.read(u64::from(entry.offset), len)
    }

    /// Reads and decompresses the entry with this name hash.
    pub fn read_hash(&mut self, hash: u32) -> Result<Vec<u8>> {
        let index = self.index_of_hash(hash).ok_or_else(|| Error::NoSuchEntry {
            archive: self.label.clone(),
            hash,
            name: None,
        })?;
        self.read(index)
    }

    /// Reads and decompresses the entry with this name.
    ///
    /// Names use the game's own spelling, backslashes included, for example
    /// `Data\Movies\Intro.PMF`.
    pub fn read_name(&mut self, name: &str) -> Result<Vec<u8>> {
        let hash = wad::hash_name(name);
        let index = self.index_of_hash(hash).ok_or_else(|| Error::NoSuchEntry {
            archive: self.label.clone(),
            hash,
            name: Some(name.to_string()),
        })?;
        self.read(index)
    }

    /// Reads the first `len` bytes of an entry, without decompressing.
    ///
    /// The point of this is headers. A `.PMF`'s parameters live in its first
    /// 2048 bytes and the file is megabytes long, so peeking is worth a
    /// dedicated call. Only valid for entries stored uncompressed, which every
    /// blob in every shipped Pulse archive is.
    pub fn peek(&mut self, index: usize, len: u64) -> Result<Vec<u8>> {
        let entry = *self.entry(index)?;
        if entry.compression != Compression::None {
            // Falling back to a full read keeps the caller honest: it still
            // gets the bytes it asked for, just not cheaply.
            let all = self.read(index)?;
            let take = (len as usize).min(all.len());
            return Ok(all[..take].to_vec());
        }
        self.source
            .read(u64::from(entry.offset), len.min(u64::from(entry.size)))
    }

    /// Uncompressed length of an entry, from the directory.
    pub fn entry_len(&self, index: usize) -> Result<u32> {
        Ok(self.entry(index)?.size_uncompressed)
    }

    fn entry(&self, index: usize) -> Result<&wad::Entry> {
        self.directory
            .entries
            .get(index)
            .ok_or_else(|| Error::NoSuchIndex {
                archive: self.label.clone(),
                count: self.directory.entries.len(),
                index,
            })
    }

    fn decode(&self, index: usize, entry: &wad::Entry, raw: Vec<u8>) -> Result<Vec<u8>> {
        match entry.compression {
            Compression::None => Ok(raw),
            Compression::Lzss => {
                let want = entry.size_uncompressed as usize;
                let out =
                    oag_formats::lzss::decompress(&raw, want).map_err(|e| Error::BadBlob {
                        archive: self.label.clone(),
                        index,
                        message: e.to_string(),
                    })?;
                // A length mismatch means the decoder is wrong even though it
                // did not error, which is the failure worth catching.
                if out.len() != want {
                    return Err(Error::BadBlob {
                        archive: self.label.clone(),
                        index,
                        message: format!("produced {} bytes, declared {want}", out.len()),
                    });
                }
                Ok(out)
            }
            Compression::Zlib => Err(Error::UnsupportedCompression {
                archive: self.label.clone(),
                index,
                compression: "zlib".to_string(),
            }),
        }
    }
}

impl Source {
    fn len(&self) -> u64 {
        match self {
            Self::File { len, .. } => *len,
            Self::Disc { entry, .. } => entry.size,
        }
    }

    fn read(&mut self, offset: u64, len: u64) -> Result<Vec<u8>> {
        let len = len.min(self.len().saturating_sub(offset));
        match self {
            Self::File { file, .. } => {
                use std::io::{Read, Seek, SeekFrom};
                file.seek(SeekFrom::Start(offset))?;
                let mut buf = vec![0u8; len as usize];
                file.read_exact(&mut buf)?;
                Ok(buf)
            }
            Self::Disc { disc, entry } => Ok(disc.read_entry_range(entry, offset, len)?),
        }
    }
}

/// Splits `<image>:<path>`, or returns `None` for a plain path.
#[must_use]
pub fn split_disc_spec(spec: &str) -> Option<(&str, &str)> {
    let (image, inner) = spec.rsplit_once(':')?;
    // A bare Windows drive letter is not a disc spec.
    if image.len() < 2 || inner.is_empty() {
        return None;
    }
    Some((image, inner))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_disc_spec() {
        assert_eq!(
            split_disc_spec("data/images/pulse.chd:PSP_GAME/USRDIR/FE.wad"),
            Some(("data/images/pulse.chd", "PSP_GAME/USRDIR/FE.wad"))
        );
    }

    #[test]
    fn a_plain_path_is_not_a_disc_spec() {
        assert_eq!(split_disc_spec("FE.wad"), None);
        assert_eq!(split_disc_spec("C:"), None);
    }

    #[test]
    fn a_bare_word_is_rejected_rather_than_opened() {
        assert!(matches!(Archive::open("nonsense"), Err(Error::BadSpec(_))));
    }

    /// One stored (uncompressed) entry, 64-byte aligned, the way every PSP
    /// archive is laid out.
    fn tiny_wad(name_hash: u32, blob: &[u8]) -> Vec<u8> {
        let offset = 64u32;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1u32.to_le_bytes()); // version
        bytes.extend_from_slice(&1u32.to_le_bytes()); // entry_count
        bytes.extend_from_slice(&name_hash.to_le_bytes());
        bytes.extend_from_slice(&offset.to_le_bytes());
        bytes.extend_from_slice(&(blob.len() as u32).to_le_bytes()); // size_uncompressed
        bytes.extend_from_slice(&(blob.len() as u32).to_le_bytes()); // size
        bytes.resize(offset as usize, 0);
        bytes.extend_from_slice(blob);
        bytes
    }

    fn temp_file(name: &str, contents: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("oag-assets-test-{name}"));
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn len_reports_the_files_own_size() {
        let bytes = tiny_wad(0x1234_5678, b"hello world");
        let path = temp_file("len.wad", &bytes);

        let archive = Archive::open(path.to_str().unwrap()).unwrap();
        assert_eq!(archive.len(), bytes.len() as u64);
        assert!(!archive.is_empty());

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn read_raw_matches_read_for_a_stored_entry() {
        let blob = b"a stored entry is never decompressed";
        let bytes = tiny_wad(0xdead_beef, blob);
        let path = temp_file("raw.wad", &bytes);

        let mut archive = Archive::open(path.to_str().unwrap()).unwrap();
        assert_eq!(archive.read_raw(0).unwrap(), blob);
        assert_eq!(archive.read(0).unwrap(), blob);

        std::fs::remove_file(path).ok();
    }
}
