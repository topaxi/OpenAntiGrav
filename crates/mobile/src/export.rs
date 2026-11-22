//! Handing a finished file to the phone's own shared storage: a shot into the gallery, a replay
//! into Downloads, where the player's file manager, the photos app and a share sheet reach it.
//!
//! The app's files directory is where the game writes (`docs/tools/android.md`), and on Android
//! 11 and later it is hidden from the phone's file manager. Android 10 and later let an app add
//! files of its own to `MediaStore` with no permission, which is all this does: it never reads
//! another app's media and never asks for storage access. Older Androids (the APK's floor is 8)
//! need a storage permission the manifest does not carry, so there [`export`] reports
//! [`Error::Unsupported`] and the file stays where it was written.
//!
//! The decisions (where a kind of file goes, what it is called, its MIME type) are pure and
//! tested here; the `MediaStore` calls are `android::export` and run on a phone only.

use std::fmt;
use std::path::Path;

/// What is being handed over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A photo-mode shot (PNG), into the gallery.
    Picture,
    /// A replay (`.oagr`), into Downloads.
    Replay,
}

/// The `MediaStore` collection a file is inserted into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Collection {
    /// `MediaStore.Images`: the gallery.
    Images,
    /// `MediaStore.Downloads`.
    Downloads,
}

/// The folder every export lands in, under `Pictures` or `Download`.
pub const FOLDER: &str = "OpenAntiGrav";

/// Where a file goes and how it is described to the system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The collection to insert into.
    pub collection: Collection,
    /// The file's name as the gallery or file manager shows it.
    pub display_name: String,
    /// The MIME type; the system files by it.
    pub mime: &'static str,
    /// The folder under the shared storage root, such as `Pictures/OpenAntiGrav`.
    pub relative_dir: String,
}

/// Where `file_name` of `kind` goes. A path separator or control character in the name is
/// replaced, so a name can never reach out of the folder.
#[must_use]
pub fn plan(kind: Kind, file_name: &str) -> Plan {
    let display_name: String = file_name
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    match kind {
        Kind::Picture => Plan {
            collection: Collection::Images,
            display_name,
            mime: "image/png",
            relative_dir: format!("Pictures/{FOLDER}"),
        },
        Kind::Replay => Plan {
            collection: Collection::Downloads,
            display_name,
            mime: "application/octet-stream",
            relative_dir: format!("Download/{FOLDER}"),
        },
    }
}

/// Why a file was not exported.
#[derive(Debug)]
pub enum Error {
    /// This build or this Android has no shared storage to hand a file to.
    Unsupported(&'static str),
    /// The file could not be read.
    Read(std::io::Error),
    /// The system refused: the message is what it said.
    System(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(why) => write!(f, "not supported: {why}"),
            Self::Read(e) => write!(f, "cannot read the file: {e}"),
            Self::System(why) => write!(f, "the system refused: {why}"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(target_os = "android")]
impl From<jni::errors::Error> for Error {
    fn from(e: jni::errors::Error) -> Self {
        Self::System(e.to_string())
    }
}

/// Whether this build can export at all: Android only. A given phone may still refuse
/// ([`Error::Unsupported`] below Android 10).
#[must_use]
pub const fn available() -> bool {
    cfg!(target_os = "android")
}

/// Hands the file at `path` to shared storage, named as it is, and returns the system's address
/// for the copy (a `content://` URI).
///
/// # Errors
///
/// [`Error::Unsupported`] off Android and below Android 10, [`Error::Read`] when `path` cannot
/// be read, [`Error::System`] when the system refuses.
pub fn export(path: &Path, kind: Kind) -> Result<String, Error> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or(Error::Unsupported("the path has no file name"))?;
    let bytes = std::fs::read(path).map_err(Error::Read)?;
    insert(&plan(kind, &name), &bytes)
}

#[cfg(target_os = "android")]
fn insert(plan: &Plan, bytes: &[u8]) -> Result<String, Error> {
    crate::android::export(plan, bytes)
}

#[cfg(not(target_os = "android"))]
fn insert(_: &Plan, _: &[u8]) -> Result<String, Error> {
    Err(Error::Unsupported("shared storage export is Android's"))
}

#[cfg(test)]
mod tests;
