//! The named reasons this layer refuses a file.

use std::fmt;

/// Why an item, a track or a class could not be read.
#[derive(Debug)]
pub enum Error {
    /// The generic reader refused the bundle or an object.
    Unity(oag_unity::Error),
    /// The `.wtrk` wrapper is not the one layout this reads.
    Wrapper(&'static str),
    /// A `MonoBehaviour` class the track needs is absent from the bundle.
    MissingClass(&'static str),
    /// A class appears more often than a track can hold it.
    DuplicateClass(&'static str),
    /// A field a class needs is absent or of another type. Carries class and field.
    Field(&'static str, &'static str),
    /// A pointer or index leads nowhere.
    Dangling(&'static str),
    /// An enum value or structure this reader has no meaning for.
    Unsupported(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unity(e) => write!(f, "unity: {e}"),
            Self::Wrapper(why) => write!(f, "workshop track wrapper: {why}"),
            Self::MissingClass(c) => write!(f, "track has no {c}"),
            Self::DuplicateClass(c) => write!(f, "track has more than one {c}"),
            Self::Field(c, n) => write!(f, "{c}.{n} is missing or of another type"),
            Self::Dangling(what) => write!(f, "dangling reference: {what}"),
            Self::Unsupported(what) => write!(f, "unsupported: {what}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<oag_unity::Error> for Error {
    fn from(e: oag_unity::Error) -> Self {
        Self::Unity(e)
    }
}

/// This crate's result.
pub type Result<T> = std::result::Result<T, Error>;
