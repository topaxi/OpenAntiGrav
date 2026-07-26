//! Runtime asset access: WAD archives read straight out of a disc image.
//!
//! This crate exists to stop the same twenty lines being written a fourth time.
//! [`oag-tools`'s `oag-wad`][wad], [`oag-view`'s asset loader][view] and its
//! mesh loader each grew their own copy of "open a disc image, find a `.wad`,
//! read its directory, decompress a blob". That is this crate's whole job.
//!
//! [wad]: ../../oag_tools/index.html
//! [view]: ../../oag_view/index.html
//!
//! ```no_run
//! use oag_assets::Archive;
//!
//! let mut archive = Archive::open("data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad")?;
//! let xml = archive.read_name(r"Data\Plugins\PI001\GUI\Skin.xml")?;
//! # Ok::<(), oag_assets::Error>(())
//! ```
//!
//! # Names are hashes
//!
//! A WAD directory stores only a CRC-32 of each entry name, so
//! [`Archive::read_name`] hashes the name and looks that up. It cannot list
//! names, only resolve ones you already know. See `docs/formats/wad.md`.
//!
//! # Nothing here knows about rendering
//!
//! Blobs come out as bytes. Turning them into textures, meshes or screens is
//! the caller's problem, which is what keeps this testable without a GPU.

pub mod archive;
pub mod pulse;

pub use archive::Archive;

/// Something that went wrong reaching an asset.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The archive specifier was not a path or `<image>:<path-on-disc>`.
    #[error("expected a .wad path or <image>:<path-on-disc>, got {0:?}")]
    BadSpec(String),

    /// The named file is not on the disc image.
    #[error("{path} is not on {image}")]
    NotOnDisc {
        /// The disc image opened.
        image: String,
        /// The path looked for.
        path: String,
    },

    /// No entry in the archive has that name hash.
    #[error("{archive} has no entry hashing to {hash:08x}{}", name.as_deref().map(|n| format!(" ({n})")).unwrap_or_default())]
    NoSuchEntry {
        /// Which archive was searched.
        archive: String,
        /// The hash looked for.
        hash: u32,
        /// The name it came from, when the caller supplied one.
        name: Option<String>,
    },

    /// An entry index was past the end of the directory.
    #[error("{archive} has {count} entries, asked for {index}")]
    NoSuchIndex {
        /// Which archive was searched.
        archive: String,
        /// Entry count.
        count: usize,
        /// Index asked for.
        index: usize,
    },

    /// The archive's own directory did not parse.
    #[error("{archive}: {source}")]
    BadDirectory {
        /// Which archive.
        archive: String,
        /// What the WAD parser said.
        source: oag_formats::wad::Error,
    },

    /// A blob is stored with a compression this build cannot undo.
    ///
    /// Zlib appears in the game but in no shipped archive, so there is nothing
    /// to validate an implementation against. Refusing beats guessing.
    #[error("{archive} entry {index} uses {compression}, which is not supported")]
    UnsupportedCompression {
        /// Which archive.
        archive: String,
        /// Which entry.
        index: usize,
        /// The compression named.
        compression: String,
    },

    /// LZSS decompression failed or produced the wrong length.
    #[error("{archive} entry {index}: {message}")]
    BadBlob {
        /// Which archive.
        archive: String,
        /// Which entry.
        index: usize,
        /// What went wrong.
        message: String,
    },

    /// Reading the disc image failed.
    #[error(transparent)]
    Disc(#[from] oag_disc::Error),

    /// Reading a `.wad` off the filesystem failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;
