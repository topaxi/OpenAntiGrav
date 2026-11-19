//! Error types for the ATRAC9 decoder.

use std::fmt;

/// Decoder status / error codes (mirrors the original C library).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Operation completed successfully (not returned as Err).
    Success,

    /// Feature not yet implemented.
    NotImplemented,

    /// Invalid or unsupported config data.
    BadConfigData,

    /// Superframe flag was invalid.
    UnpackSuperframeFlagInvalid,

    /// Reuse band parameters flag invalid.
    UnpackReuseBandParamsInvalid,

    /// Band parameters invalid.
    UnpackBandParamsInvalid,

    /// Gradient boundary invalid.
    UnpackGradBoundaryInvalid,

    /// Gradient start unit out of bounds.
    UnpackGradStartUnitOob,

    /// Gradient end unit out of bounds.
    UnpackGradEndUnitOob,

    /// Gradient start value out of bounds.
    UnpackGradStartValueOob,

    /// Gradient end value out of bounds.
    UnpackGradEndValueOob,

    /// Gradient end unit < start unit.
    UnpackGradEndUnitInvalid,

    /// Scale-factor coding mode invalid.
    UnpackScaleFactorModeInvalid,

    /// Scale-factor value out of bounds.
    UnpackScaleFactorOob,

    /// Extension data invalid.
    UnpackExtensionDataInvalid,

    /// Buffer too small for output.
    BufferTooSmall,

    /// Input data truncated / incomplete frame.
    UnexpectedEof,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Success => write!(f, "success"),
            Error::NotImplemented => write!(f, "not implemented"),
            Error::BadConfigData => write!(f, "bad config data"),
            Error::UnpackSuperframeFlagInvalid => write!(f, "invalid superframe flag"),
            Error::UnpackReuseBandParamsInvalid => write!(f, "invalid reuse band params"),
            Error::UnpackBandParamsInvalid => write!(f, "invalid band params"),
            Error::UnpackGradBoundaryInvalid => write!(f, "invalid gradient boundary"),
            Error::UnpackGradStartUnitOob => write!(f, "gradient start unit out of bounds"),
            Error::UnpackGradEndUnitOob => write!(f, "gradient end unit out of bounds"),
            Error::UnpackGradStartValueOob => write!(f, "gradient start value out of bounds"),
            Error::UnpackGradEndValueOob => write!(f, "gradient end value out of bounds"),
            Error::UnpackGradEndUnitInvalid => write!(f, "gradient end unit < start unit"),
            Error::UnpackScaleFactorModeInvalid => write!(f, "invalid scale factor mode"),
            Error::UnpackScaleFactorOob => write!(f, "scale factor out of bounds"),
            Error::UnpackExtensionDataInvalid => write!(f, "invalid extension data"),
            Error::BufferTooSmall => write!(f, "output buffer too small"),
            Error::UnexpectedEof => write!(f, "unexpected end of input"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
