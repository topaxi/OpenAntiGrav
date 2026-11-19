//! The installer's names on a target with no download (wasm, Android), so the composition root
//! compiles unchanged. [`Pin::current`](crate::Pin::current) is `None` there and nothing calls
//! these.

use crate::Pin;
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64};

/// See the native `Progress`.
#[derive(Debug, Default)]
pub struct Progress {
    /// Bytes received.
    pub done: AtomicU64,
    /// Bytes expected.
    pub total: AtomicU64,
    /// Stop request.
    pub cancel: AtomicBool,
}

/// See the native `Cancelled`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the download was cancelled")
    }
}

impl std::error::Error for Cancelled {}

/// Always an error: nothing is downloaded on this target.
///
/// # Errors
/// Always.
pub fn install(_pin: &Pin, _root: &Path, _progress: &Progress) -> Result<PathBuf> {
    bail!("ffmpeg is not downloaded on this platform")
}

/// Always an error: nothing is downloaded on this target.
///
/// # Errors
/// Always.
pub fn install_from(_url: &str, pin: &Pin, root: &Path, progress: &Progress) -> Result<PathBuf> {
    install(pin, root, progress)
}
