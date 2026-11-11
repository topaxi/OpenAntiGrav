//! Where an archive's bytes come from, whichever container they are in.
//!
//! Both the [WAD](crate::Archive) and the [PSARC](crate::psarc) reader want the
//! same thing underneath: a lazy `(offset, len)` read into a file on disc or
//! into one entry of a disc image. Neither container may be slurped - a
//! `Data.wad` is 315 MiB, a `DATA02.PSARC` more - and on a CHD every read costs
//! decompression, so this is the layer that keeps both of them seeking.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use oag_disc::DiscImage;

use crate::archive::split_disc_spec;
use crate::{Error, Result};

/// What a poisoned lock means here: a *previous* read of this image panicked
/// while holding it. The image is not corrupt - it is a read-only file handle -
/// but the panic that got us here is the real failure and this is not the place
/// to paper over it.
pub(crate) const POISONED: &str = "a previous read of this disc image panicked";

/// Where an archive's bytes come from.
///
/// Both variants read lazily; see the module docs for why that is not an
/// optimisation.
pub(crate) enum BlobSource {
    File {
        file: std::fs::File,
        len: u64,
        path: PathBuf,
    },
    /// A server extraction's stand-in for an archive: only the ranges a load read.
    /// Chosen by the file's first bytes, never by its name (`crate::sparse`).
    Sparse {
        sparse: crate::sparse::Sparse,
        path: PathBuf,
    },
    Disc {
        /// **Shared, so the containers of one disc are one open image.**
        ///
        /// `Archives::open` mounts two archives off a Pulse disc and eight off
        /// a Wipeout HD one, and each used to own a `DiscImage` of its own: a
        /// separate file handle, a separate ISO 9660 walk, and - the part that
        /// costs - a separate single-hunk CHD cache, so a caller alternating
        /// `Data.wad` and `FE.wad` thrashed two caches that never saw each
        /// other's hunks. One image behind them all fixes both.
        ///
        /// Measured on `Archives::open`, five opens, best of, 2026-09-09:
        ///
        /// | image | before | after |
        /// | --- | --- | --- |
        /// | `pulse-ps2-eu.chd`, 3.7 GB | 47.2 ms | **17.9 ms** |
        /// | `pulse-psp-eu.chd` | 27.5 ms | **10.0 ms** |
        ///
        /// Both on a CHD with no extract beside it, which is where this shows:
        /// `oag_testdata::image` hands a test a raw `.iso` when one exists, and
        /// a raw walk is 0.05 ms either way.
        ///
        /// `Arc<Mutex<_>>` rather than `Rc<RefCell<_>>`, and not by preference:
        /// `oag_game::boot` sends an `Archives` into the thread it loads media
        /// on, so this has to be `Send`. An uncontended lock is tens of
        /// nanoseconds against a CHD hunk decompression, and the containers
        /// sharing this are read one at a time from the one thread that owns
        /// them, so it is uncontended in practice.
        disc: Arc<Mutex<DiscImage>>,
        entry: oag_disc::Entry,
        /// The image `disc` was opened from, for a read on the browser page's
        /// thread that finds `disc` held by a worker: it opens the image
        /// afresh rather than wait ([`read_disc`]).
        image: PathBuf,
    },
}

/// Reads `entry`'s range from the shared image, or, on the browser page's
/// thread when a worker holds it, from a private open of the same image: that
/// thread may not wait on a lock (Firefox freezes the page there, Chromium
/// traps; docs/tools/web.md, "The page's thread may not wait"). A private open
/// costs an ISO walk and a cold hunk cache, which only a contended read pays.
fn read_disc(
    disc: &Mutex<DiscImage>,
    image: &Path,
    entry: &oag_disc::Entry,
    offset: u64,
    len: u64,
) -> Result<Vec<u8>> {
    read_disc_on(disc, image, entry, offset, len, oag_thread::may_wait())
}

/// [`read_disc`] on a thread that `may_wait` or may not.
fn read_disc_on(
    disc: &Mutex<DiscImage>,
    image: &Path,
    entry: &oag_disc::Entry,
    offset: u64,
    len: u64,
    may_wait: bool,
) -> Result<Vec<u8>> {
    if may_wait {
        return Ok(disc
            .lock()
            .expect(POISONED)
            .read_entry_range(entry, offset, len)?);
    }
    match disc.try_lock() {
        Ok(mut shared) => Ok(shared.read_entry_range(entry, offset, len)?),
        Err(std::sync::TryLockError::Poisoned(_)) => panic!("{POISONED}"),
        Err(std::sync::TryLockError::WouldBlock) => {
            Ok(DiscImage::open(image)?.read_entry_range(entry, offset, len)?)
        }
    }
}

impl BlobSource {
    pub(crate) fn len(&self) -> u64 {
        match self {
            Self::File { len, .. } => *len,
            Self::Sparse { sparse, .. } => sparse.len(),
            Self::Disc { entry, .. } => entry.size,
        }
    }

    pub(crate) fn read(&mut self, offset: u64, len: u64) -> Result<Vec<u8>> {
        let len = len.min(self.len().saturating_sub(offset));
        if crate::record::active() {
            let total = self.len();
            match &*self {
                Self::File { path, .. } | Self::Sparse { path, .. } => {
                    crate::record::note(&path.to_string_lossy(), total, offset, len);
                }
                Self::Disc { entry, .. } => {
                    crate::record::note(&entry.path, total, offset, len);
                }
            }
        }
        match self {
            Self::Sparse { sparse, path } => {
                sparse.read(offset, len).map(<[u8]>::to_vec).ok_or_else(|| {
                    crate::record::note_miss(format!(
                        "{} {offset}..{}",
                        path.display(),
                        offset + len
                    ));
                    Error::NotExtracted {
                        archive: path.display().to_string(),
                        offset,
                        len,
                    }
                })
            }
            Self::File { file, .. } => {
                use std::io::{Read, Seek, SeekFrom};
                file.seek(SeekFrom::Start(offset))?;
                let mut buf = vec![0u8; len as usize];
                file.read_exact(&mut buf)?;
                Ok(buf)
            }
            Self::Disc { disc, entry, image } => read_disc(disc, image, entry, offset, len),
        }
    }

    /// Opens `spec`, either a path to a container file or
    /// `<image>:<path-on-disc>`.
    ///
    /// Splitting on the last colon rather than the first keeps Windows drive
    /// letters working.
    pub(crate) fn open(spec: &str) -> Result<Self> {
        if let Some((image, inner)) = split_disc_spec(spec) {
            let disc = Arc::new(Mutex::new(DiscImage::open(image)?));
            return Self::on_disc(&disc, image, inner);
        }

        let path = PathBuf::from(spec);
        if path.extension().is_none() {
            // A bare word is far more likely to be a mistyped disc spec than a
            // real file, and the error is much more useful this way.
            return Err(Error::BadSpec(spec.to_string()));
        }
        Self::open_file(&path)
    }

    /// Opens a container file that is already known to be one, bypassing
    /// spec parsing. See [`crate::Archive::open_file`].
    /// One entry of an image that is **already open**, sharing it.
    ///
    /// `image` is carried only to name the disc in [`Error::NotOnDisc`]; the
    /// bytes all come from `disc`. See [`Self::Disc`]'s own docs for why the
    /// sharing is the point.
    pub(crate) fn on_disc(disc: &Arc<Mutex<DiscImage>>, image: &str, inner: &str) -> Result<Self> {
        let entry = disc
            .lock()
            .expect(POISONED)
            .entries()?
            .iter()
            .find(|e| !e.is_directory && e.path.eq_ignore_ascii_case(inner))
            .cloned()
            .ok_or_else(|| Error::NotOnDisc {
                image: image.to_string(),
                path: inner.to_string(),
            })?;
        Ok(Self::Disc {
            disc: Arc::clone(disc),
            entry,
            image: PathBuf::from(image),
        })
    }

    pub(crate) fn open_file(path: &Path) -> Result<Self> {
        use std::io::Read;
        let mut file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();
        let mut head = [0u8; 4];
        if file.read_exact(&mut head).is_ok() && crate::sparse::Sparse::is_sparse(&head) {
            let sparse =
                crate::sparse::Sparse::parse(&path.display().to_string(), std::fs::read(path)?)?;
            return Ok(Self::Sparse {
                sparse,
                path: path.to_path_buf(),
            });
        }
        Ok(Self::File {
            file,
            len,
            path: path.to_path_buf(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A read on a thread that may not wait, of an image another holder has
    /// locked, returns the same bytes through a private open instead of
    /// waiting (here the holder is this very thread, so a wait would never end).
    #[test]
    #[ignore = "needs data/images/pulse-psp-eu.chd"]
    fn a_read_that_may_not_wait_reads_past_a_held_disc() {
        let Some(image) = oag_testdata::image("pulse-psp-eu.chd") else {
            return;
        };
        let disc = Arc::new(Mutex::new(
            DiscImage::open(&image).expect("the image opens"),
        ));
        let entry = disc
            .lock()
            .expect(POISONED)
            .entries()
            .expect("the image lists its files")
            .iter()
            .find(|e| !e.is_directory && e.size > 70_000)
            .cloned()
            .expect("a file of some size");
        let want = read_disc_on(&disc, &image, &entry, 65_536, 4_096, true).expect("a shared read");
        let held = disc.lock().expect(POISONED);
        let got =
            read_disc_on(&disc, &image, &entry, 65_536, 4_096, false).expect("a private read");
        drop(held);
        assert_eq!(got.len(), 4_096, "{}", entry.path);
        assert_eq!(
            got, want,
            "{}: the private open read other bytes",
            entry.path
        );
    }
}
