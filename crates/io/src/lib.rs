//! The atomic write (ADR-0095): `path` holds the old content or the new, never a mix.

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Taken across a write's temp file and rename, so two threads writing one file never share
/// its temp file.
static WRITING: Mutex<()> = Mutex::new(());

/// Who may read the file once it is in place.
#[derive(Clone, Copy)]
enum Access {
    Shared,
    OwnerOnly,
}

#[cfg(any(test, feature = "crash-hook"))]
thread_local! {
    /// A test sets this to cut the next write short: that many bytes reach the temp file,
    /// then the write fails as a crash would, leaving the temp file where it is.
    static CUT_AFTER: std::cell::Cell<Option<usize>> =
        const { std::cell::Cell::new(None) };
    static CRASHED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// As [`std::fs::write`], but atomic: `path` holds the old content or the new, never a mix.
///
/// # Errors
/// The directory is missing or cannot be written. The old file is then as it was.
pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    replace(path.as_ref(), contents.as_ref(), Access::Shared)
}

/// [`write`] for a file that is not text; the web build keeps no such file.
///
/// # Errors
/// As [`write`].
pub fn write_bytes(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    replace(path.as_ref(), contents.as_ref(), Access::Shared)
}

/// [`write`] for a secret: where the system has owner-only files the file is one from its
/// first byte.
///
/// # Errors
/// As [`write`].
pub fn write_private(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    replace(path.as_ref(), contents.as_ref(), Access::OwnerOnly)
}

/// `path` with `.tmp` appended: `records.toml` stages in `records.toml.tmp`.
fn staging(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

fn replace(path: &Path, contents: &[u8], access: Access) -> io::Result<()> {
    let _writing = WRITING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let temp = staging(path);
    let staged = stage(&temp, contents, access).and_then(|()| commit(&temp, path));
    if let Err(why) = staged {
        // A crashed process leaves its temp file; a failed write cleans up after itself.
        #[cfg(any(test, feature = "crash-hook"))]
        if CRASHED.with(|crashed| crashed.replace(false)) {
            return Err(why);
        }
        let _ = std::fs::remove_file(&temp);
        return Err(why);
    }
    Ok(())
}

/// Writes `contents` to a fresh `temp` and flushes it to the disk. A temp file left by an
/// earlier crash is removed first, so its mode and content never leak into this write.
fn stage(temp: &Path, contents: &[u8], access: Access) -> io::Result<()> {
    let _ = std::fs::remove_file(temp);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    if matches!(access, Access::OwnerOnly) {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    #[cfg(not(unix))]
    let _ = access;
    let mut file = options.open(temp)?;
    #[cfg(any(test, feature = "crash-hook"))]
    if let Some(cut) = CUT_AFTER.with(std::cell::Cell::take) {
        file.write_all(&contents[..cut.min(contents.len())])?;
        CRASHED.with(|crashed| crashed.set(true));
        return Err(io::Error::other("simulated crash mid-write"));
    }
    file.write_all(contents)?;
    file.sync_all()
}

/// Renames `temp` over `path`. Windows refuses while another process holds `path` open
/// without sharing deletes (a virus scanner, a file indexer); that is usually over in
/// milliseconds, so it is tried a few times before the old file is kept.
fn commit(temp: &Path, path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        let mut tries = 0;
        loop {
            match std::fs::rename(temp, path) {
                Err(why) if why.kind() == io::ErrorKind::PermissionDenied && tries < 5 => {
                    tries += 1;
                    std::thread::sleep(std::time::Duration::from_millis(20 * tries));
                }
                done => return done,
            }
        }
    }
    #[cfg(not(windows))]
    std::fs::rename(temp, path)
}

/// Makes the next [`write`] on this thread die after `bytes` bytes of the new content, leaving its
/// temp file as a crashed process would.
#[cfg(any(test, feature = "crash-hook"))]
pub fn cut_the_next_write_after(bytes: usize) {
    CUT_AFTER.with(|cut| cut.set(Some(bytes)));
}

#[cfg(test)]
mod tests;
