//! The kinds of failure the web build tells its page about.
//!
//! The page (`web/fault.js`) owns the words a player reads; this module owns
//! which kind a failure is. A kind's tag is the key of a row in that file's
//! `KINDS` table, and a test below fails for a tag the page has no row for.
//! Compiled on every target so the classification runs under `cargo nextest`;
//! the calls into the page are in `gpu::web`. See `docs/tools/web.md`,
//! "Troubleshooting".

use std::fmt;

/// Why the game stopped or what it is telling the page, for the kinds the
/// module itself knows. The page tells the rest apart (no WebGPU, a missing
/// header, a trap) before or without the module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A Rust panic.
    Panic,
    /// A WebGPU validation error: the picture may be wrong, the game runs on.
    GpuValidation,
    GpuOutOfMemory,
    GpuInternal,
    /// The device was lost for a reason other than the game ending it.
    GpuLost,
    /// The browser has WebGPU and gave no adapter.
    NoAdapter,
    /// The adapter would not give the device the game asks for.
    DeviceRequest,
    /// The file opened but is no disc image the game plays.
    DiscUnsupported,
    /// The file is cut short, damaged or no longer readable.
    DiscUnreadable,
    /// Anything else that stopped the start-up.
    Startup,
}

impl Kind {
    /// Every kind, for the test that reads the page's table.
    pub const ALL: [Kind; 10] = [
        Kind::Panic,
        Kind::GpuValidation,
        Kind::GpuOutOfMemory,
        Kind::GpuInternal,
        Kind::GpuLost,
        Kind::NoAdapter,
        Kind::DeviceRequest,
        Kind::DiscUnsupported,
        Kind::DiscUnreadable,
        Kind::Startup,
    ];

    /// The row of `web/fault.js`'s `KINDS` this kind is.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Kind::Panic => "panic",
            Kind::GpuValidation => "gpu-validation",
            Kind::GpuOutOfMemory => "gpu-out-of-memory",
            Kind::GpuInternal => "gpu-internal",
            Kind::GpuLost => "gpu-lost",
            Kind::NoAdapter => "no-adapter",
            Kind::DeviceRequest => "device-request",
            Kind::DiscUnsupported => "disc-unsupported",
            Kind::DiscUnreadable => "disc-unreadable",
            Kind::Startup => "startup",
        }
    }
}

/// The kind of an error WebGPU reported without being asked.
#[must_use]
pub fn gpu_error(error: &wgpu::Error) -> Kind {
    match error {
        wgpu::Error::OutOfMemory { .. } => Kind::GpuOutOfMemory,
        wgpu::Error::Validation { .. } => Kind::GpuValidation,
        wgpu::Error::Internal { .. } => Kind::GpuInternal,
    }
}

/// The kind of a lost device; `None` when the game destroyed it itself, which
/// is how it lets go of it and not a failure.
#[must_use]
pub fn device_lost(reason: wgpu::DeviceLostReason) -> Option<Kind> {
    match reason {
        wgpu::DeviceLostReason::Destroyed => None,
        wgpu::DeviceLostReason::Unknown => Some(Kind::GpuLost),
    }
}

/// An error that carries its kind through `anyhow`, as a context layer, so the
/// entry point can hand the page a tag and not a sentence to parse.
#[derive(Debug)]
pub struct Failure {
    pub kind: Kind,
    message: String,
}

impl Failure {
    #[must_use]
    pub fn new(kind: Kind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Failure {}

/// The kind `error` was tagged with, or [`Kind::Startup`].
#[must_use]
pub fn kind_of(error: &anyhow::Error) -> Kind {
    error.downcast_ref::<Failure>().map_or(Kind::Startup, |f| f.kind)
}

/// Whether `error` is a disc image that is cut short or cannot be read, as
/// against a file that read fine and is not a disc the game plays.
#[must_use]
pub fn disc_error(error: &anyhow::Error) -> Kind {
    let short = error.chain().any(|cause| {
        cause.downcast_ref::<std::io::Error>().is_some_and(|io| {
            matches!(
                io.kind(),
                std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::NotFound
            )
        })
    });
    if short {
        Kind::DiscUnreadable
    } else {
        Kind::DiscUnsupported
    }
}

#[cfg(test)]
mod tests;
