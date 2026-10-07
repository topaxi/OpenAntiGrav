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
mod blob_source;
pub mod container;
pub mod dlc;
pub mod psarc;
pub mod read_memo;
pub mod source;
#[cfg(test)]
mod testing;

pub use archive::Archive;
pub use container::Container;
pub use source::{Archives, Layout, Platform, read_loose_file};

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

    /// A PS3 disc image whose archives will not read because its encrypted
    /// regions are still encrypted.
    #[error(
        "{looked_in} is an encrypted PS3 disc image (its archives are encrypted on disc). \
         Decrypt it with your own disc key first: \
         `uv run --with cryptography python3 scripts/ps3iso.py decrypt <image.iso> <key> hdfury-ps3-eu-dec.iso` \
         (needs the Python package `cryptography`), then put the decrypted image in data/images/."
    )]
    EncryptedDisc {
        /// The disc image that would not read.
        looked_in: String,
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
    /// The advice this used to give - "support is tracked for M8" - was true
    /// when nothing but Pulse could open, and stopped being true on 2026-08-11:
    /// `oag_source::title::open_source` catches this error for Pure and opens the
    /// disc as Pure instead, which is how `--race` reaches a Pure circuit. So
    /// the error now means what it always literally said and nothing more - *you
    /// asked for one title and this disc is another* - and it points at the
    /// call that does the choosing rather than at a milestone.
    #[error(
        "{looked_in} identifies as {serial} ({title}), not Wipeout Pulse. \
         Open it through oag_source::title::open_source, which picks the title \
         from the serial; see docs/overview/roadmap.md for how far {title} gets."
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

    /// A `.fnt` did not decode.
    #[error("{name}: {source}")]
    Font {
        /// The entry name asked for.
        name: String,
        /// What the font parser said.
        source: oag_texture::fnt::Error,
    },

    /// A PS2 `.fnt`'s glyph atlas - the entry following it - did not decode.
    ///
    /// Separate from [`Font`](Error::Font) because the two say different
    /// things about what went wrong: this one means the directory-position
    /// rule found something that is not a texture.
    #[error("{name}: the entry after it is not a glyph atlas ({source})")]
    FontAtlas {
        /// The entry name asked for.
        name: String,
        /// What the texture parser said.
        source: oag_texture::ps2_texture::Error,
    },

    /// The archive's own directory did not parse.
    #[error("{archive}: {source}")]
    BadDirectory {
        /// Which archive.
        archive: String,
        /// What the WAD parser said.
        source: oag_formats::wad::Error,
    },

    /// A PSARC archive's own directory did not parse.
    ///
    /// Also what a `.psarc` inside an *encrypted* PS3 image produces: the
    /// header read finds noise and the magic check says so. See
    /// `docs/formats/ps3-disc.md`.
    #[error("{archive}: {source}")]
    BadPsarcDirectory {
        /// Which archive.
        archive: String,
        /// What the PSARC parser said.
        source: oag_formats::psarc::Error,
    },

    /// A PSARC archive has no entry with this path.
    ///
    /// Unlike [`Error::NoSuchEntry`], the archive knows every path it holds, so
    /// this really does mean the path is absent rather than unmined.
    #[error("{archive} has no entry at {path}")]
    NoSuchPath {
        /// Which archive.
        archive: String,
        /// The path asked for.
        path: String,
    },

    /// A caller asked a PSARC for something only a WAD can answer.
    ///
    /// Three things about a [WAD](crate::Archive) are not properties of an
    /// archive in general, and each is load-bearing somewhere:
    ///
    /// - **The name hash.** A WAD stores hashes and no names, so a caller with
    ///   an unmined entry addresses it as `hash:3d2c85f8`. A PSARC stores the
    ///   real paths, so it has nothing to compare a hash against - and, since a
    ///   PSARC's own per-entry MD5 is over the *path*, no hash to derive one
    ///   from either.
    /// - **Directory position.** The PS2 addresses a model's texture set as
    ///   "the entry before" and a font's atlas as "the entry after"; a PSARC's
    ///   order is the manifest's and means nothing.
    /// - **Ranged and raw reads** into an entry's stored bytes, which a WAD
    ///   serves from a flat blob and a PSARC only through its block table.
    ///
    /// Raised rather than approximated, because every one of those has a
    /// plausible-looking wrong answer available.
    #[error("{archive} is a PSARC, and {wanted} is a WAD-only way to address an entry")]
    NotAWad {
        /// Which archive.
        archive: String,
        /// What was asked for, as a person would name it.
        wanted: &'static str,
    },

    /// A ranged read asked for the middle of a compressed blob.
    ///
    /// Raised for both LZSS and zlib, and for the same reason: [`decompress`]
    /// (LZSS) and `miniz_oxide` (zlib) are decodable here, they are just not
    /// *seekable*. LZSS's back-references reach into the ring buffer the
    /// earlier bytes filled and zlib's deflate stream reaches into its own
    /// sliding window, so byte 300 MiB of either cannot be produced without
    /// decoding the 300 MiB before it, and the whole point of
    /// [`Archive::read_range`] is not doing that. Falling back to a full read
    /// the way [`Archive::peek`] does would silently turn a bounded read into
    /// an unbounded one, so the caller is told instead and can reach for
    /// [`Archive::read`] knowingly.
    ///
    /// [`decompress`]: oag_formats::lzss::decompress
    #[error(
        "{archive} entry {index} is stored {compression}-compressed, so it cannot be read at an offset; read the whole entry instead"
    )]
    RangeIntoCompressed {
        /// Which archive.
        archive: String,
        /// Which entry.
        index: usize,
        /// The compression the entry declared.
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
