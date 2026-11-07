//! What can be wrong with a `.vex` file.

use std::fmt;

use super::FILE_HEADER_LEN;

/// Something wrong with a `.vex` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the file header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// A format version this project has no class table for.
    ///
    /// An error rather than a fall back to version 6: the numberings share no
    /// id, so the wrong table finds nothing and looks like an empty file.
    UnknownVersion {
        /// The version word the file declares.
        version: u32,
    },
    /// A structure points outside the file.
    OutOfBounds {
        /// What was being read.
        what: &'static str,
        /// Where it would end.
        end: usize,
        /// Size of the file.
        len: usize,
    },
    /// A vertex type this build does not handle: an unexplored asset class or a
    /// decoding error, either worth hearing about loudly.
    UnsupportedVertexType {
        /// The value found.
        vertex_type: u16,
    },
    /// A PS2 batch's VIF command stream did not walk.
    Vif {
        /// Offset of the batch header in the mesh payload.
        at: usize,
        /// What the walker said. Boxed to keep this enum small.
        error: Box<crate::vif::Error>,
    },
    /// A PS2 batch's packet framing did not add up.
    ///
    /// Separate from [`Self::Vif`]: these checks decide whether the packet was
    /// read as the right *kind* of thing (three lengths that must close, the
    /// attribute set agreeing with the vertex type).
    Packet {
        /// Which check failed.
        what: &'static str,
        /// Offset of the batch header in the mesh payload.
        at: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { got } => {
                write!(f, "need at least {FILE_HEADER_LEN} bytes, got {got}")
            }
            Self::UnknownVersion { version } => write!(
                f,
                "no class table for .vex version {version}; \
                 known versions are 6 (Pulse), 4 and 3"
            ),
            Self::OutOfBounds { what, end, len } => {
                write!(f, "{what} ends at {end} but the file is {len} bytes")
            }
            Self::UnsupportedVertexType { vertex_type } => {
                write!(f, "unsupported GU vertex type {vertex_type:#06x}")
            }
            Self::Vif { at, error } => write!(f, "batch at {at}: {error}"),
            Self::Packet { what, at } => write!(f, "batch at {at}: {what}"),
        }
    }
}

impl std::error::Error for Error {}
