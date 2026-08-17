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

    /// Whether this is a WAD, and so whether the hash-addressed reads are
    /// available.
    ///
    /// Asked by a caller that has a *second* route for the other container
    /// rather than one that is about to fail - [`Self::as_wad_mut`] is still how
    /// a WAD-only path says so, and its error names what it wanted. The boot
    /// movie is the case this exists for: a WAD addresses it by name hash and a
    /// PSARC by path, and both work, so "which container is this?" is a real
    /// question there rather than a prelude to an apology.
    #[must_use]
    pub fn is_wad(&self) -> bool {
        matches!(self, Self::Wad(_))
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

    /// How many entries this archive holds, for a load report.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        match self {
            Self::Wad(a) => a.directory().entries.len(),
            Self::Psarc(a) => a.paths().len(),
        }
    }

    /// Whether this archive holds `name`, by whatever it calls a name.
    ///
    /// The question [`crate::Archives`] asks to decide which of its mounted
    /// archives serves a read, and the reason it is a method here: a WAD
    /// answers by hashing the name, a PSARC by normalising it to the path
    /// spelling it stored. Neither answer is available to the other.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        match self {
            Self::Wad(a) => a.contains(name),
            Self::Psarc(a) => a.contains(name),
        }
    }

    /// This container as a WAD, or [`Error::NotAWad`](crate::Error::NotAWad).
    ///
    /// For the reads that are WAD-shaped rather than archive-shaped - by hash,
    /// by directory position, into the middle of an entry. See that error for
    /// the three of them and why none is approximated on a PSARC.
    ///
    /// # Errors
    ///
    /// [`Error::NotAWad`](crate::Error::NotAWad) when this is a PSARC, naming
    /// `wanted` so the message says which of the three was asked for.
    pub fn as_wad_mut(&mut self, wanted: &'static str) -> Result<&mut Archive> {
        match self {
            Self::Wad(a) => Ok(a),
            Self::Psarc(a) => Err(crate::Error::NotAWad {
                archive: a.label().to_string(),
                wanted,
            }),
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
