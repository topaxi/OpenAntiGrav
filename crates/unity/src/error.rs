//! Every way a read can refuse. A reader either decodes or names why not; none
//! of these is a panic, because the bytes are untrusted user content.

use std::fmt;

/// A named refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A read ran past the end of the bytes it was given.
    Truncated {
        /// What was being read.
        what: &'static str,
        /// Offset the read started at.
        at: usize,
        /// Bytes it needed.
        need: usize,
        /// Bytes that were left.
        have: usize,
    },
    /// The bytes do not start with the `UnityFS` signature.
    BadSignature,
    /// An archive version this reader has no layout for.
    UnsupportedArchiveVersion(u32),
    /// A block compression this reader does not implement (LZHAM, say).
    UnsupportedCompression(u32),
    /// A flag bit in the bundle header that is not understood.
    UnsupportedFlags(u32),
    /// A decompressor failed or produced the wrong amount of data.
    Decompress {
        /// Which codec.
        codec: &'static str,
        /// What went wrong.
        reason: String,
    },
    /// A count or size from the file is beyond what the reader will allocate
    /// or believe.
    LimitExceeded {
        /// What the number counts.
        what: &'static str,
        /// The value read.
        value: u64,
        /// The most this reader accepts.
        limit: u64,
    },
    /// A structural rule the format guarantees did not hold.
    Malformed(&'static str),
    /// A SerializedFile version without a known layout.
    UnsupportedSerializedVersion(u32),
    /// A typetree string offset into the built-in table that is not in it.
    UnknownCommonString(u32),
    /// A typetree string offset past the end of the local string buffer.
    BadStringOffset(u32),
    /// A node type the value reader has no rule for.
    UnsupportedType(String),
    /// An object whose class has no typetree in this file.
    NoTypeTree {
        /// Its class id.
        class_id: i32,
    },
    /// A value read did not consume exactly the object's bytes.
    SizeMismatch {
        /// Bytes the object table gives.
        expected: usize,
        /// Bytes the typetree walk consumed.
        consumed: usize,
    },
    /// A typed class read found a field missing or of the wrong kind.
    Field {
        /// The class being read.
        class: &'static str,
        /// The field.
        field: &'static str,
    },
    /// A construct of a built-in class that is recognised but not implemented.
    Unsupported(&'static str),
    /// A texture format id with no size rule here.
    UnsupportedTextureFormat(i32),
    /// A value outside the range its meaning allows.
    OutOfRange(&'static str),
    /// A string that is not UTF-8.
    InvalidUtf8,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated {
                what,
                at,
                need,
                have,
            } => write!(f, "truncated {what} at {at}: need {need}, have {have}"),
            Self::BadSignature => write!(f, "not a UnityFS bundle"),
            Self::UnsupportedArchiveVersion(v) => write!(f, "unsupported UnityFS version {v}"),
            Self::UnsupportedCompression(c) => write!(f, "unsupported compression type {c}"),
            Self::UnsupportedFlags(x) => write!(f, "unsupported bundle flags {x:#x}"),
            Self::Decompress { codec, reason } => write!(f, "{codec}: {reason}"),
            Self::LimitExceeded { what, value, limit } => {
                write!(f, "{what} {value} exceeds limit {limit}")
            }
            Self::Malformed(m) => write!(f, "malformed: {m}"),
            Self::UnsupportedSerializedVersion(v) => {
                write!(f, "unsupported SerializedFile version {v}")
            }
            Self::UnknownCommonString(o) => write!(f, "unknown common string offset {o}"),
            Self::BadStringOffset(o) => write!(f, "string offset {o} outside the buffer"),
            Self::UnsupportedType(t) => write!(f, "no value rule for type {t:?}"),
            Self::NoTypeTree { class_id } => write!(f, "no typetree for class {class_id}"),
            Self::SizeMismatch { expected, consumed } => {
                write!(
                    f,
                    "object is {expected} bytes, typetree consumed {consumed}"
                )
            }
            Self::Field { class, field } => write!(f, "{class}.{field} missing or wrong kind"),
            Self::Unsupported(what) => write!(f, "unsupported: {what}"),
            Self::UnsupportedTextureFormat(id) => write!(f, "texture format {id} has no size rule"),
            Self::OutOfRange(what) => write!(f, "out of range: {what}"),
            Self::InvalidUtf8 => write!(f, "string is not UTF-8"),
        }
    }
}

impl std::error::Error for Error {}

/// This crate's result.
pub type Result<T> = std::result::Result<T, Error>;
