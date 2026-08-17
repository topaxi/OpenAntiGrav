//! Where an archive's bytes come from, whichever container they are in.
//!
//! Both the [WAD](crate::Archive) and the [PSARC](crate::psarc) reader want the
//! same thing underneath: a lazy `(offset, len)` read into a file on disc or
//! into one entry of a disc image. Neither container may be slurped - a
//! `Data.wad` is 315 MiB, a `DATA02.PSARC` more - and on a CHD every read costs
//! decompression, so this is the layer that keeps both of them seeking.

use std::path::{Path, PathBuf};

use oag_disc::DiscImage;

use crate::archive::split_disc_spec;
use crate::{Error, Result};

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
        disc: Box<DiscImage>,
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
            Self::Disc { disc, entry } => Ok(disc.read_entry_range(entry, offset, len)?),
        }
    }

    /// Opens `spec`, either a path to a container file or
    /// `<image>:<path-on-disc>`.
    ///
    /// Splitting on the last colon rather than the first keeps Windows drive
    /// letters working.
    pub(crate) fn open(spec: &str) -> Result<Self> {
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
            return Ok(Self::Disc {
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
        Ok(Self::File { file, len })
    }

    /// Opens a container file that is already known to be one, bypassing
    /// spec parsing. See [`crate::Archive::open_file`].
    pub(crate) fn open_file(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();
        Ok(Self::File { file, len })
    }
}
