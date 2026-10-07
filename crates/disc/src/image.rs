//! The front door: open an image, list it, read files out of it.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::chd_source::ChdSource;
use crate::error::{Error, Result};
use crate::iso9660::{self, Entry, VolumeDescriptor};
use crate::package::PackageSource;
use crate::platform::{self, Platform, TitleInfo};
use crate::ps3_crypt::{self, DiscKey, Unlocked};
use crate::raw_source::RawSource;
use crate::source::{SECTOR_SIZE, SectorSource};

/// CHD files begin with this tag.
const CHD_MAGIC: &[u8; 8] = b"MComprHD";

/// Which container an image turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    /// MAME Compressed Hunks of Data.
    Chd,
    /// A flat sequence of 2048-byte sectors.
    RawIso,
    /// A ZIP of a game's folder: a Vita `.vpk`, read in place.
    Vpk,
}

impl std::fmt::Display for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Chd => "CHD",
            Self::RawIso => "raw ISO",
            Self::Vpk => "VPK",
        })
    }
}

/// How a PS3 image's encrypted regions are being read; see [`crate::ps3_crypt`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ps3State {
    /// Not a PS3 image with encrypted regions (or not a raw ISO at all).
    NotEncrypted,
    /// Declares encrypted regions that already read as plaintext (a dump
    /// decrypted by other means).
    AlreadyPlain,
    /// Decrypting in place, with a key the oracle accepted.
    Decrypting,
    /// Encrypted, and no key found passes the oracle: the archives read as
    /// noise until the player supplies one.
    Locked,
}

/// An opened disc image.
#[derive(Debug)]
pub struct DiscImage {
    source: Box<dyn SectorSource>,
    path: PathBuf,
    container: Container,
    ps3: Ps3State,
    entries: Option<Vec<Entry>>,
    /// Set for a package image (`.vpk`), where [`Self::source`] is empty and
    /// every read goes here instead.
    package: Option<Box<dyn PackageSource>>,
}

/// The sector source of an image that has no sectors.
#[derive(Debug)]
struct NoSectors;

impl SectorSource for NoSectors {
    fn sector_count(&self) -> u32 {
        0
    }

    fn read_sector(&mut self, lba: u32, _buf: &mut [u8]) -> Result<()> {
        Err(Error::SectorOutOfRange {
            sector: lba,
            total: 0,
        })
    }
}

impl DiscImage {
    /// Opens an image, detecting the container by magic rather than extension.
    ///
    /// Extension-based detection would mislabel a `.iso` that is really a CHD,
    /// which is a common result of renaming a download.
    ///
    /// A raw ISO that is an encrypted PS3 disc is opened with every key
    /// [`ps3_crypt::find_keys`] finds (beside the image, then the app's keys
    /// directory); [`Self::ps3_state`] says whether one worked.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let keys = if sniff_container(path)? == Container::RawIso {
            ps3_crypt::find_keys(path)
        } else {
            Vec::new()
        };
        Self::open_with_keys(path, &keys)
    }

    /// As [`Self::open`], with the keys the caller already holds instead of a
    /// search.
    pub fn open_with_keys(path: impl AsRef<Path>, keys: &[DiscKey]) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let container = sniff_container(&path)?;

        let mut package: Option<Box<dyn PackageSource>> = None;
        let (source, ps3): (Box<dyn SectorSource>, Ps3State) = match container {
            Container::Vpk => {
                package = Some(Box::new(crate::vpk::VpkSet::open(&path)?));
                (Box::new(NoSectors), Ps3State::NotEncrypted)
            }
            Container::Chd => (Box::new(ChdSource::open(&path)?), Ps3State::NotEncrypted),
            Container::RawIso => {
                let raw = Box::new(RawSource::open(&path)?);
                match ps3_crypt::unlock(raw, keys)? {
                    Unlocked::NotEncrypted(s) => (s, Ps3State::NotEncrypted),
                    Unlocked::AlreadyPlain(s) => (s, Ps3State::AlreadyPlain),
                    Unlocked::Decrypting(s) => (s, Ps3State::Decrypting),
                    Unlocked::Locked(s) => (s, Ps3State::Locked),
                }
            }
        };

        Ok(Self {
            source,
            path,
            container,
            ps3,
            entries: None,
            package,
        })
    }

    /// How this image's PS3 encryption is being handled.
    #[must_use]
    pub fn ps3_state(&self) -> Ps3State {
        self.ps3
    }

    /// The path the image was opened from.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Which container the image uses.
    #[must_use]
    pub fn container(&self) -> Container {
        self.container
    }

    /// Total logical sectors.
    #[must_use]
    pub fn sector_count(&self) -> u32 {
        self.source.sector_count()
    }

    /// Reads the primary volume descriptor.
    pub fn volume_descriptor(&mut self) -> Result<VolumeDescriptor> {
        if self.package.is_some() {
            return Err(Error::NoPrimaryVolumeDescriptor { searched: 0 });
        }
        iso9660::read_volume_descriptor(self.source.as_mut())
    }

    /// Every file and directory on the disc, depth-first.
    ///
    /// Walking the tree costs a lot of decompression on a CHD, so the result is
    /// cached for the lifetime of the image.
    pub fn entries(&mut self) -> Result<&[Entry]> {
        if self.entries.is_none() {
            self.entries = Some(match &self.package {
                // A package's `lba` is the file's index, see `crate::package`.
                Some(package) => package
                    .files()
                    .iter()
                    .enumerate()
                    .map(|(index, file)| Entry {
                        path: file.path.clone(),
                        lba: u32::try_from(index).unwrap_or(u32::MAX),
                        size: file.size,
                        is_directory: false,
                    })
                    .collect(),
                None => iso9660::walk(self.source.as_mut())?,
            });
        }
        Ok(self.entries.as_ref().expect("just populated"))
    }

    /// Identifies the console and title.
    pub fn identify(&mut self) -> Result<TitleInfo> {
        // Cloned so the borrow of `self.entries` ends before `identify` needs
        // `self.source` mutably. The listing is small next to the image.
        let entries = self.entries()?.to_vec();
        if self.package.is_some() {
            return self.identify_package(&entries);
        }
        platform::identify(self.source.as_mut(), &entries)
    }

    /// A package names its release through its own `sce_sys/param.sfo`.
    fn identify_package(&mut self, entries: &[Entry]) -> Result<TitleInfo> {
        let sfo = entries
            .iter()
            .find(|e| e.path.to_ascii_lowercase().ends_with("sce_sys/param.sfo"));
        let serial = match sfo {
            Some(entry) => crate::sfo::title_id(&self.read_entry(entry)?),
            None => None,
        };
        Ok(TitleInfo {
            platform: Platform::Vita,
            serial,
            boot_path: None,
            raw: None,
        })
    }

    /// Whether this image's sector 0 declares PS3 encrypted regions; see
    /// [`platform::ps3_declares_encrypted_regions`].
    pub fn ps3_declares_encrypted_regions(&mut self) -> Result<bool> {
        let mut sector = vec![0u8; crate::source::SECTOR_SIZE];
        self.source.read_sector(0, &mut sector)?;
        Ok(platform::ps3_declares_encrypted_regions(
            &sector,
            self.source.sector_count(),
        ))
    }

    /// Reads one file by path, case-insensitively.
    pub fn read_file(&mut self, path: &str) -> Result<Vec<u8>> {
        let entry = self
            .entries()?
            .iter()
            .find(|e| !e.is_directory && e.path.eq_ignore_ascii_case(path))
            .cloned()
            .ok_or_else(|| Error::NotFound {
                path: path.to_string(),
            })?;

        self.read_entry(&entry)
    }

    /// Reads the contents of an entry returned by [`Self::entries`].
    pub fn read_entry(&mut self, entry: &Entry) -> Result<Vec<u8>> {
        if let Some(package) = &mut self.package {
            return package.read(entry.lba as usize, 0, entry.size);
        }
        self.source.read_range(entry.lba, entry.size)
    }

    /// Reads at most `max_len` bytes from the start of an entry.
    ///
    /// Triage only needs a header and an entropy sample, and reading a whole
    /// multi-hundred-megabyte archive to look at its first four bytes would
    /// make sniffing a disc take minutes instead of seconds.
    pub fn read_entry_head(&mut self, entry: &Entry, max_len: u64) -> Result<Vec<u8>> {
        if let Some(package) = &mut self.package {
            return package.read(entry.lba as usize, 0, entry.size.min(max_len));
        }
        self.source.read_range(entry.lba, entry.size.min(max_len))
    }

    /// Reads `len` bytes from `offset` within an entry.
    ///
    /// Seeks to the containing sector rather than reading from the start of the
    /// file, so poking at a structure 300 MB into an archive costs one sector
    /// read instead of 300 MB of decompression.
    ///
    /// Returns fewer bytes than asked for if the range runs past the end of the
    /// entry, and an empty vector if `offset` is past the end.
    pub fn read_entry_range(&mut self, entry: &Entry, offset: u64, len: u64) -> Result<Vec<u8>> {
        if offset >= entry.size {
            return Ok(Vec::new());
        }
        if let Some(package) = &mut self.package {
            return package.read(entry.lba as usize, offset, len);
        }
        let len = len.min(entry.size - offset);

        // Reads are sector-granular, so start at the sector containing `offset`
        // and trim the bytes before it.
        let sector_size = SECTOR_SIZE as u64;
        let skip = offset % sector_size;
        // **A checked add, and the `unwrap_or` under it is now unreachable
        // rather than load-bearing.** `entry.lba + offset/2048` overflowed u32
        // for the same hostile multi-extent entries `SectorSource::read_range`
        // names - a debug panic, or in release a wrap to a wrong sector that
        // reads whatever is there. Finding F4 of the 2026-08-18 review; one
        // checked add covers both halves of the pair.
        let start_lba = u32::try_from(offset / sector_size)
            .ok()
            .and_then(|sectors| entry.lba.checked_add(sectors))
            .ok_or(Error::SectorOutOfRange {
                sector: u32::MAX,
                total: self.source.sector_count(),
            })?;

        let mut data = self.source.read_range(start_lba, skip + len)?;
        data.drain(..(skip as usize).min(data.len()));
        Ok(data)
    }
}

/// Peeks at the first bytes of a file to decide what container it is.
fn sniff_container(path: &Path) -> Result<Container> {
    let mut file = File::open(path).map_err(|e| Error::io(path, e))?;
    let mut magic = [0u8; 8];

    // A file shorter than 8 bytes is not any container we support.
    let read = file.read(&mut magic).map_err(|e| Error::io(path, e))?;
    if read < 8 {
        return Err(Error::UnknownContainer {
            path: path.to_path_buf(),
        });
    }

    if &magic == CHD_MAGIC {
        return Ok(Container::Chd);
    }
    if magic[..4] == *crate::vpk::MAGIC {
        return Ok(Container::Vpk);
    }
    // Named refusals, not guesses: both are packages this engine cannot read in
    // place, and a raw-ISO assumption would only fail later as "no ISO 9660
    // primary volume descriptor".
    if magic[..4] == *b"\x7fPKG" {
        return Err(Error::Package {
            path: path.to_path_buf(),
            reason: "this is a Vita .pkg. Its game files are PFS-encrypted with a key the console \
                     derives in hardware from the licence (not a public constant), so it is not \
                     read in place. Use a NoNpDrm .vpk or folder of the same game, which carry \
                     plain files (docs/formats/vita-package.md)"
                .to_string(),
        });
    }
    if magic[..4] == *b"\x7fCNT" {
        return Err(Error::Package {
            path: path.to_path_buf(),
            reason: "this is a PS4 .pkg. Reading one in place is not implemented yet; extract it \
                     into a folder first (docs/overview/installing.md, \"Omega Collection\")"
                .to_string(),
        });
    }

    // Anything else is assumed to be a raw ISO. The assumption is verified
    // immediately: RawSource::open plus the PVD search will fail clearly if
    // there is no ISO 9660 volume here.
    Ok(Container::RawIso)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_file(name: &str, contents: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("oag-disc-test-{name}"));
        let mut f = File::create(&path).unwrap();
        f.write_all(contents).unwrap();
        path
    }

    #[test]
    fn recognises_a_chd_by_magic() {
        let path = temp_file("magic.iso", b"MComprHD\x00\x00\x00\x7c");
        // Named `.iso` but actually a CHD: the extension must not win.
        assert_eq!(sniff_container(&path).unwrap(), Container::Chd);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn falls_back_to_raw_iso() {
        let path = temp_file("plain.chd", &vec![0u8; 4096]);
        assert_eq!(sniff_container(&path).unwrap(), Container::RawIso);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn rejects_a_file_too_short_to_identify() {
        let path = temp_file("tiny.bin", b"abc");
        assert!(matches!(
            sniff_container(&path),
            Err(Error::UnknownContainer { .. })
        ));
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn a_vita_or_ps4_pkg_is_refused_by_name_not_read_as_an_iso() {
        let vita = temp_file("vita.pkg", b"\x7fPKG\x80\x00\x00\x01 more header");
        let error = sniff_container(&vita).unwrap_err().to_string();
        assert!(
            error.contains("Vita .pkg") && error.contains("NoNpDrm"),
            "{error}"
        );
        let ps4 = temp_file("ps4.pkg", b"\x7fCNT\x00\x00\x00\x01 more header");
        let error = sniff_container(&ps4).unwrap_err().to_string();
        assert!(error.contains("PS4 .pkg"), "{error}");
        std::fs::remove_file(vita).ok();
        std::fs::remove_file(ps4).ok();
    }

    #[test]
    fn a_zip_is_a_vpk() {
        let path = temp_file("a.vpk", b"PK\x03\x04 and the rest");
        assert_eq!(sniff_container(&path).unwrap(), Container::Vpk);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn container_displays_readably() {
        assert_eq!(Container::Chd.to_string(), "CHD");
        assert_eq!(Container::RawIso.to_string(), "raw ISO");
        assert_eq!(Container::Vpk.to_string(), "VPK");
    }
}
