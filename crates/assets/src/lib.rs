//! Runtime asset access: WAD archives read straight out of a disc image.
//!
//! This crate exists to stop the same twenty lines being written a fourth time.
//! [`oag-tools`'s `oag-wad`][wad] and [`oag-view`'s asset and mesh loaders][view]
//! used to each carry their own copy of "open a disc image, find a `.wad`, read
//! its directory, decompress a blob"; all three now go through [`Archive`].
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

    /// Nothing in the source is an archive any known layout names.
    ///
    /// Lists every candidate rather than only the platform's own, since the
    /// commonest cause of an *unidentified* source is one that is not a Pulse
    /// disc at all, and a message naming one file makes that look like a
    /// missing file instead. A source that positively identifies as a
    /// *different, known* title gets [`WrongTitle`](Error::WrongTitle)
    /// instead, before archive matching ever runs.
    #[error("{looked_in} ({platform}) holds none of: {}", looked_for.join(", "))]
    NoArchive {
        /// The disc image or directory looked in.
        ///
        /// Not called `source`: `thiserror` reserves that name for a nested
        /// error, and this is a path.
        looked_in: String,
        /// What the source identified itself as.
        platform: String,
        /// Every archive name tried.
        looked_for: Vec<String>,
    },

    /// The disc identifies as a Studio Liverpool title this project does not
    /// play.
    ///
    /// Wipeout Pure ships PSP archives byte-identical in name to Pulse's
    /// (`Data.wad`, `FE.wad`), so without this check
    /// [`pulse::Layout::resolve`] would open a Pure disc exactly as if it
    /// were Pulse - archive-name matching alone cannot tell the two apart.
    /// Raised only when the disc's own serial is positively known to belong
    /// to another title; an extracted directory with no header (ADR-0004),
    /// or an uncatalogued serial, gets no verdict here and falls through to
    /// [`NoArchive`](Error::NoArchive) if nothing then matches.
    #[error(
        "{looked_in} identifies as {serial} ({title}), not Wipeout Pulse. \
         {title} support is tracked for M8 - Beyond Pulse; see docs/overview/roadmap.md."
    )]
    WrongTitle {
        /// The disc image or directory looked in.
        looked_in: String,
        /// The serial read off the disc.
        serial: String,
        /// The title that serial is known to belong to.
        title: String,
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
