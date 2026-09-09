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
    },
}

impl BlobSource {
    pub(crate) fn len(&self) -> u64 {
        match self {
            Self::File { len, .. } => *len,
            Self::Disc { entry, .. } => entry.size,
        }
    }

    pub(crate) fn read(&mut self, offset: u64, len: u64) -> Result<Vec<u8>> {
        let len = len.min(self.len().saturating_sub(offset));
        match self {
            Self::File { file, .. } => {
                use std::io::{Read, Seek, SeekFrom};
                file.seek(SeekFrom::Start(offset))?;
                let mut buf = vec![0u8; len as usize];
                file.read_exact(&mut buf)?;
                Ok(buf)
            }
            Self::Disc { disc, entry } => Ok(disc
                .lock()
                .expect(POISONED)
                .read_entry_range(entry, offset, len)?),
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
        let file = std::fs::File::open(&path)?;
        let len = file.metadata()?.len();
        Ok(Self::File { file, len })
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
        })
    }

    pub(crate) fn open_file(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();
        Ok(Self::File { file, len })
    }
}
