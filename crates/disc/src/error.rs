//! Errors produced while reading disc images.

use std::path::PathBuf;

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Something went wrong reading a disc image.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An underlying I/O failure.
    #[error("i/o error on {path}")]
    Io {
        /// The file being read when the failure happened.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },

    /// An I/O failure with no useful path to attribute it to.
    #[error("i/o error")]
    BareIo(#[from] std::io::Error),

    /// The `chd` crate rejected the file.
    #[error("chd error")]
    Chd(#[from] chd::Error),

    /// The file is not a container we recognise.
    #[error("{path} is not a recognised disc image (expected CHD or raw ISO 9660)")]
    UnknownContainer {
        /// The file that was sniffed.
        path: PathBuf,
    },

    /// The CHD's units have no 2048-byte user-data area to read.
    ///
    /// Raw MODE1 and MODE2/FORM1 frames are handled. Audio tracks and
    /// MODE2/FORM2 are not, because they carry no 2048-byte user area at all.
    #[error(
        "{path} stores {unit_bytes}-byte units{}, which contain no 2048-byte user data area",
        .mode.as_ref().map(|m| format!(" of type {m}")).unwrap_or_default()
    )]
    CdImageUnsupported {
        /// The file that was opened.
        path: PathBuf,
        /// The unit size found in the CHD header.
        unit_bytes: u32,
        /// The track mode, when the image declares one.
        mode: Option<String>,
    },

    /// A hunk cannot hold whole units, so the sector mapping does not exist.
    ///
    /// `hunk_size` and `unit_bytes` both come from the CHD header, and the
    /// mapping from logical block address to hunk divides one by the other. A
    /// hunk smaller than a unit makes that division zero, and a hunk that is not
    /// a whole number of units puts the last unit's user data past the end of the
    /// decompressed buffer. Neither is producible by `chdman`; both are one
    /// edited header away.
    #[error(
        "{path} declares {hunk_bytes}-byte hunks of {unit_bytes}-byte units, \
         which cannot be divided into whole sectors"
    )]
    HunkGeometryUnsupported {
        /// The file that was opened.
        path: PathBuf,
        /// The hunk size found in the CHD header.
        hunk_bytes: u32,
        /// The unit size the layout was detected as.
        unit_bytes: u32,
    },

    /// The image has more than one track, or a non-zero pregap.
    ///
    /// Pregaps shift the mapping from logical block address to stored unit, so
    /// a flat mapping would read the wrong data without failing. No in-scope
    /// Wipeout title ships as a multi-track disc.
    #[error(
        "{path} has {tracks} track(s) with pregaps; only single-track data discs are supported"
    )]
    MultiTrackUnsupported {
        /// The file that was opened.
        path: PathBuf,
        /// How many tracks the image declares.
        tracks: usize,
    },

    /// No ISO 9660 primary volume descriptor was found.
    #[error("no ISO 9660 primary volume descriptor found (searched sectors 16..{searched})")]
    NoPrimaryVolumeDescriptor {
        /// The last sector index searched.
        searched: u32,
    },

    /// A structure on the disc was malformed.
    #[error("malformed ISO 9660 structure at sector {sector}: {reason}")]
    MalformedFilesystem {
        /// Sector the bad structure was read from.
        sector: u32,
        /// What was wrong with it.
        reason: String,
    },

    /// A read went past the end of the image.
    #[error("sector {sector} is out of range (image has {total} sectors)")]
    SectorOutOfRange {
        /// The requested sector.
        sector: u32,
        /// How many sectors the image actually has.
        total: u32,
    },

    /// A requested path is not on the disc.
    #[error("{path} not found on disc")]
    NotFound {
        /// The path that was looked up.
        path: String,
    },

    /// A package file (a `.vpk`) could not be read as one.
    #[error("{path}: {reason}")]
    Package {
        /// The file that was opened.
        path: PathBuf,
        /// What was wrong with it.
        reason: String,
    },

    /// Directory nesting exceeded the recursion limit.
    ///
    /// A malformed or hostile image can point a directory record at its own
    /// parent, and an unbounded walk would never terminate.
    #[error("directory nesting deeper than {limit} levels; image is probably corrupt")]
    TooDeep {
        /// The limit that was hit.
        limit: u32,
    },
}

impl Error {
    /// Attaches a path to an I/O error.
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
