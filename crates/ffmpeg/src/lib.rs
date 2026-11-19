//! Finding `ffmpeg` and `ffprobe`, and fetching a pinned build of them.
//!
//! Every call site that starts one of the two goes through [`command`] (or
//! [`resolve`]): `PATH` first, then the downloaded copy, then nothing. There is
//! no latched "missing" answer anywhere, so a copy that arrives mid-session is
//! used by the very next decode.
//!
//! A system tool on `PATH` wins unless the config file's `use_pinned` is on, and
//! nothing here ever downloads on its own: [`install`] is called by the
//! composition root after the player said yes
//! (`docs/architecture/ffmpeg.md`).

mod pins;
pub use pins::{Archive, Pin};

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
mod install;
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub use install::{Progress, install, install_from};

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::RwLock;

/// One of the two executables the game starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// Decodes music and transcodes movies.
    Ffmpeg,
    /// Reads a raw MPEG-2 program stream's size and rate, and the prefetch check.
    Ffprobe,
}

impl Tool {
    /// The executable's file name on this OS.
    #[must_use]
    pub const fn file_name(self) -> &'static str {
        match (self, cfg!(windows)) {
            (Self::Ffmpeg, false) => "ffmpeg",
            (Self::Ffmpeg, true) => "ffmpeg.exe",
            (Self::Ffprobe, false) => "ffprobe",
            (Self::Ffprobe, true) => "ffprobe.exe",
        }
    }
}

/// What the composition root tells the resolver about this run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Policy {
    /// Prefer the pinned copy over one on `PATH` (the config file's
    /// `[ffmpeg] use_pinned`).
    pub use_pinned: bool,
    /// The directory pinned builds are installed under; `None` means no
    /// downloaded copy is looked for.
    pub root: Option<PathBuf>,
}

static POLICY: RwLock<Policy> = RwLock::new(Policy {
    use_pinned: false,
    root: None,
});

/// Sets the policy for the rest of the process.
pub fn configure(policy: Policy) {
    *POLICY
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = policy;
}

fn policy() -> Policy {
    POLICY
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// The directory this build's pinned copy lives in under `root`, if this OS has
/// a pin. The pin's version is in the name, so a pin bump never reuses an old
/// binary.
#[must_use]
pub fn install_dir(root: &Path) -> Option<PathBuf> {
    Pin::current().map(|pin| root.join(pin.dir_name()))
}

/// The pinned copy of `tool` under `root`, when it is installed.
#[must_use]
pub fn pinned(root: &Path, tool: Tool) -> Option<PathBuf> {
    let path = install_dir(root)?.join(tool.file_name());
    path.is_file().then_some(path)
}

/// Where `tool` is on `PATH`, searched the way a shell would.
#[must_use]
pub fn on_path(tool: Tool) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    on_path_in(&paths, tool)
}

fn on_path_in(paths: &std::ffi::OsStr, tool: Tool) -> Option<PathBuf> {
    std::env::split_paths(paths)
        .map(|dir| dir.join(tool.file_name()))
        .find(|candidate| candidate.is_file())
}

/// The order of preference, as a pure function of what was found.
///
/// A system tool wins unless `use_pinned` asks for ours; either falls back to
/// the other. `None` means the tool is missing.
#[must_use]
pub fn pick(
    on_path: Option<PathBuf>,
    pinned: Option<PathBuf>,
    use_pinned: bool,
) -> Option<PathBuf> {
    if use_pinned {
        pinned.or(on_path)
    } else {
        on_path.or(pinned)
    }
}

/// Where the tool to run is, or `None` when there is none.
#[must_use]
pub fn resolve(tool: Tool) -> Option<PathBuf> {
    let policy = policy();
    let found = policy.root.as_deref().and_then(|root| pinned(root, tool));
    pick(on_path(tool), found, policy.use_pinned)
}

/// Whether the player has no `ffmpeg` at all, system or downloaded.
#[must_use]
pub fn is_missing() -> bool {
    resolve(Tool::Ffmpeg).is_none()
}

/// A [`Command`] for `tool`, or `None` when it is missing. Never latched.
#[must_use]
pub fn command(tool: Tool) -> Option<Command> {
    resolve(tool).map(Command::new)
}

/// Whether to offer the download: no `ffmpeg` anywhere, this OS has a pin, or
/// `use_pinned` is set and the pinned copy is not there yet.
#[must_use]
pub fn wants_download() -> bool {
    let policy = policy();
    let Some(root) = policy.root.as_deref() else {
        return false;
    };
    if Pin::current().is_none() {
        return false;
    }
    if policy.use_pinned {
        return pinned(root, Tool::Ffmpeg).is_none();
    }
    on_path(Tool::Ffmpeg).is_none() && pinned(root, Tool::Ffmpeg).is_none()
}

#[cfg(test)]
mod tests;
