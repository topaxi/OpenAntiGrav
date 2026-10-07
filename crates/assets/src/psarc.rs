//! One PSARC archive, on the filesystem or inside a PS3 disc image.
//!
//! The runtime half of [`oag_formats::psarc`], which is the format and does no
//! I/O. This holds the [`BlobSource`] and turns an entry's declared block range
//! into bytes.
//!
//! # What it is not
//!
//! Not [`crate::Archive`], and deliberately a separate type rather than a
//! variant of it. A WAD entry is found by a **hash of its name**, is LZSS or
//! stored, and its directory row is a fixed 16 bytes; a PSARC entry is found by
//! its **real path**, is a run of deflated 64 KiB blocks, and its row is 30.
//! The two share their lazy reads and nothing else, so the shared part is
//! [`BlobSource`] and the rest is written twice on purpose.
//!
//! # The image has to be decrypted first
//!
//! A `.psarc` on a retail PS3 disc lies inside an encrypted region, so the
//! header read finds noise and [`oag_formats::psarc::Error::BadMagic`] says so
//! rather than guessing. See `docs/formats/ps3-disc.md`.

use std::collections::HashMap;

use oag_formats::psarc::{self, Directory};

use crate::blob_source::BlobSource;
use crate::{Error, Result};

/// A PSARC archive with its directory and manifest parsed.
pub struct Archive {
    source: BlobSource,
    label: String,
    directory: Directory,
    paths: Vec<String>,
    /// Directory index storing `paths[n]`. `n + 1` on a version-1.3 archive,
    /// where entry order is manifest order - not on a version-1.4 one, where
    /// it never is. See `oag_formats::psarc`'s "Entry 0 is the manifest, not
    /// a file" section.
    entry_of_path: Vec<usize>,
    /// [`normalise`]d path to its position in `paths`, the first spelling
    /// winning a collision as a scan in order would. A race load asks this
    /// archive thousands of times, and a scan that normalised every stored
    /// path per ask was a quarter of an HD race load.
    index: HashMap<String, usize>,
}

impl std::fmt::Debug for Archive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("psarc::Archive")
            .field("label", &self.label)
            .field("entries", &self.paths.len())
            .finish()
    }
}

impl Archive {
    /// Opens `spec`, either a path to a `.psarc` or `<image>:<path-on-disc>`.
    ///
    /// # Errors
    ///
    /// The source not opening, or the archive's own table of contents not
    /// parsing - which includes the header landing on noise because the image
    /// was never decrypted.
    pub fn open(spec: &str) -> Result<Self> {
        Self::from_source(BlobSource::open(spec)?, spec.to_string())
    }

    /// Opens a file that is already known to be one, bypassing spec parsing.
    ///
    /// See [`crate::Archive::open_file`] for why a spec and a path are not the
    /// same thing.
    ///
    /// # Errors
    ///
    /// As [`Archive::open`], less the spec parsing.
    pub fn open_file(path: &std::path::Path) -> Result<Self> {
        Self::from_source(BlobSource::open_file(path)?, path.display().to_string())
    }

    pub(crate) fn from_source(mut source: BlobSource, label: String) -> Result<Self> {
        let bad = |source: psarc::Error| Error::BadPsarcDirectory {
            archive: label.clone(),
            source,
        };

        let head = source.read(0, psarc::HEADER_LEN as u64)?;
        let header = psarc::Header::parse(&head).map_err(bad)?;
        let toc = source.read(0, u64::from(header.toc_len))?;
        let directory = Directory::parse(&toc).map_err(bad)?;

        // Entry 0 is the manifest. Reading it here is what makes the archive
        // addressable by name at all; `match_paths_to_entries` is what ties
        // its paths back to the entries that actually store them, by digest -
        // the header's declared version does not say which archives need
        // that rather than a positional reading, so every archive is read
        // the same way. See `oag_formats::psarc`'s module docs.
        let (offset, len) = directory.entry_range(MANIFEST).map_err(bad)?;
        let stored = source.read(offset, len)?;
        let manifest_bytes = directory.read_entry(MANIFEST, &stored).map_err(bad)?;
        let manifest_paths = psarc::parse_manifest(&manifest_bytes);
        let matches = psarc::match_paths_to_entries(&directory.entries, &manifest_paths);
        let mut paths = Vec::with_capacity(matches.len());
        let mut entry_of_path = Vec::with_capacity(matches.len());
        for psarc::PathEntry { index, path } in matches {
            paths.push(path);
            entry_of_path.push(index);
        }

        let index = path_index(&paths);
        Ok(Self {
            source,
            label,
            directory,
            paths,
            entry_of_path,
            index,
        })
    }

    /// How this archive was named, for error messages.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Every entry this archive can both name and locate in the directory,
    /// the manifest and any unbacked or placeholder entry excluded.
    ///
    /// Paths are stored lowercase and absolute, like
    /// `/data/environments/talons_junction/track.vex`. This is the one
    /// structural way PSARC is *easier* than the WAD it replaces: a WAD
    /// stores only a name hash, so most of its names still have to be mined.
    ///
    /// On a whole extraction of `omega-ps4-eu{,-patch}` every listed path is
    /// also readable and carries its extension's real content - 46,005 of
    /// 46,005 across the nine archives, `docs/formats/psarc.md`. A copy whose
    /// counts fall far short of that (`data00` listing 1,492 paths, not
    /// 10,926), or whose entries read as zeros, is a short-read extraction,
    /// not a property of the archives.
    #[must_use]
    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    /// The parsed directory. Entry order carries no promised relationship to
    /// [`Archive::paths`]'s order - it happens to, on a well-behaved
    /// archive, but nothing here relies on that - so
    /// [`Archive::index_of_path`] is always what recovers the
    /// correspondence, by digest. See `oag_formats::psarc`'s "Entry 0 is the
    /// manifest, not a file" section.
    #[must_use]
    pub fn directory(&self) -> &Directory {
        &self.directory
    }

    /// Index of the entry with this path, matched case-insensitively and with a
    /// leading `/` optional on either side.
    #[must_use]
    pub fn index_of_path(&self, path: &str) -> Option<usize> {
        self.index
            .get(&normalise(path))
            .map(|&n| self.entry_of_path[n])
    }

    /// Whether an entry with this path exists.
    #[must_use]
    pub fn contains(&self, path: &str) -> bool {
        self.index_of_path(path).is_some()
    }

    /// Reads and inflates entry `index`.
    ///
    /// # Errors
    ///
    /// [`Error::BadPsarcDirectory`] for an index the archive does not have or
    /// a block that does not inflate, and I/O for the read itself.
    pub fn read(&mut self, index: usize) -> Result<Vec<u8>> {
        let bad = |source: psarc::Error| Error::BadPsarcDirectory {
            archive: self.label.clone(),
            source,
        };
        let (offset, len) = self.directory.entry_range(index).map_err(bad)?;
        let stored = self.source.read(offset, len)?;
        self.directory.read_entry(index, &stored).map_err(bad)
    }

    /// Reads and inflates the entry with this path.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchPath`] naming the archive when nothing matches, plus
    /// everything [`Archive::read`] can raise.
    pub fn read_path(&mut self, path: &str) -> Result<Vec<u8>> {
        let index = self.index_of_path(path).ok_or_else(|| Error::NoSuchPath {
            archive: self.label.clone(),
            path: path.to_string(),
        })?;
        self.read(index)
    }
}

/// Entry 0 of every archive: the manifest, not a file.
const MANIFEST: usize = 0;

/// Every path's [`normalise`]d spelling to its position, first one kept.
fn path_index(paths: &[String]) -> HashMap<String, usize> {
    let mut index = HashMap::with_capacity(paths.len());
    for (n, path) in paths.iter().enumerate() {
        index.entry(normalise(path)).or_insert(n);
    }
    index
}

/// One spelling of a path, so `Data\...`, `/data/...` and `data/...` all match.
///
/// Backslashes are folded because that is how every *other* archive in this
/// project spells a path, and a caller that has one in hand should not have to
/// know which container it came out of.
fn normalise(path: &str) -> String {
    path.trim_start_matches('/')
        .replace('\\', "/")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{normalise, path_index};

    #[test]
    fn a_path_matches_however_the_caller_spells_it() {
        let stored = "/data/environments/talons_junction/track.vex";
        for spelling in [
            "/data/environments/talons_junction/track.vex",
            "data/environments/talons_junction/track.vex",
            "Data\\Environments\\Talons_Junction\\Track.vex",
            "DATA/ENVIRONMENTS/TALONS_JUNCTION/TRACK.VEX",
        ] {
            assert_eq!(normalise(spelling), normalise(stored), "{spelling}");
        }
    }

    #[test]
    fn two_different_paths_still_differ() {
        assert_ne!(
            normalise("/data/environments/talons_junction/track.vex"),
            normalise("/data/environments/talons_junction/track_reversed.vex")
        );
    }

    #[test]
    fn two_spellings_of_one_path_resolve_to_the_first_like_a_scan() {
        let paths = [
            "/data/a.vex".to_string(),
            "/Data/B.vex".to_string(),
            "data/b.vex".to_string(),
        ];
        let index = path_index(&paths);
        assert_eq!(index.get(&normalise("DATA\\b.VEX")), Some(&1));
        assert_eq!(index.get(&normalise("a.vex")), None);
        assert_eq!(index.get(&normalise("/data/a.vex")), Some(&0));
    }
}
