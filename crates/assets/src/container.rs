//! Opening whichever archive container a spec points at.
//!
//! Two containers reach this project: the [WAD](crate::Archive) Pure and Pulse
//! ship on PSP and PS2, and the [PSARC](crate::psarc) Wipeout HD ships on PS3.
//! A caller that wants one named entry out of one archive should not have to
//! know which, so [`read_entry`] is the call and this module is the dispatch.
//!
//! # Chosen by the data, never by the console
//!
//! The first four bytes decide: `PSAR` is a PSARC and anything else is tried as
//! a WAD, whose own header check then has the last word. Not the file
//! extension - `oag_assets::source::Layout` states the rule and names the single
//! existing exception to it, and a container that announces itself in its own
//! magic is not one.

use crate::blob_source::BlobSource;
use crate::{Archive, Result, psarc};

/// One open archive, of whichever kind the bytes turned out to be.
#[derive(Debug)]
pub enum Container {
    /// Pure and Pulse's archive, on PSP and PS2.
    Wad(Box<Archive>),
    /// Wipeout HD's archive, on PS3.
    Psarc(Box<psarc::Archive>),
}

impl Container {
    /// Opens `spec`, either a path to an archive file or
    /// `<image>:<path-on-disc>`.
    ///
    /// # Errors
    ///
    /// The source not opening, or the archive's own directory not parsing.
    pub fn open(spec: &str) -> Result<Self> {
        Self::from_source(BlobSource::open(spec)?, spec.to_string())
    }

    /// Opens a file that is already known to be one, bypassing spec parsing.
    ///
    /// See [`Archive::open_file`] for why a spec and a path are not the same
    /// thing.
    ///
    /// # Errors
    ///
    /// As [`Container::open`], less the spec parsing.
    pub fn open_file(path: &std::path::Path) -> Result<Self> {
        Self::from_source(BlobSource::open_file(path)?, path.display().to_string())
    }

    fn from_source(mut source: BlobSource, label: String) -> Result<Self> {
        let magic = source.read(0, 4)?;
        if magic == oag_formats::psarc::MAGIC {
            return Ok(Self::Psarc(Box::new(psarc::Archive::from_source(
                source, label,
            )?)));
        }
        Ok(Self::Wad(Box::new(Archive::from_source(source, label)?)))
    }

    /// How this archive was named, for error messages.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Wad(a) => a.label(),
            Self::Psarc(a) => a.label(),
        }
    }

    /// Reads one entry, by whatever this container calls a name.
    ///
    /// A WAD takes the game's own spelling with backslashes,
    /// `Data\Environments\16_Track\track.vex`; a PSARC takes the real path it
    /// stores, `/data/environments/talons_junction/track.vex`, and accepts
    /// either separator and any case. **They are not interchangeable**: a WAD
    /// stores only a hash of the name, so a differently spelled name hashes to
    /// nothing rather than being found.
    ///
    /// # Errors
    ///
    /// Nothing matching the name, or the entry not decompressing.
    pub fn read_entry(&mut self, name: &str) -> Result<Vec<u8>> {
        match self {
            Self::Wad(a) => a.read_name(name),
            Self::Psarc(a) => a.read_path(name),
        }
    }
}

/// Reads one named entry out of the archive `spec` points at.
///
/// The one-shot form of [`Container::open`] plus [`Container::read_entry`], for
/// a caller that wants a single blob and holds no archive.
///
/// # Errors
///
/// Everything [`Container::open`] and [`Container::read_entry`] can raise.
pub fn read_entry(spec: &str, name: &str) -> Result<Vec<u8>> {
    Container::open(spec)?.read_entry(name)
}
