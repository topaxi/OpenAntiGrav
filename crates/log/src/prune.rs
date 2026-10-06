//! Dropping what is older than the retention window, on startup.

use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::stamp;

/// How long an entry is kept.
pub const RETENTION: Duration = Duration::from_secs(7 * 24 * 3600);

/// `text` without the entries older than `cutoff` (a [`stamp::format`] string),
/// or `None` when nothing needs to change.
///
/// An entry is a line that starts with a stamp plus every line after it that
/// does not (a multi-line message, a panic backtrace). A line ahead of the
/// first stamp belongs to an entry that is already gone. A missing final
/// newline - a crash in mid-write - is put back, so the next run's first line
/// does not join the torn one.
#[must_use]
pub fn prune_text(text: &str, cutoff: &str) -> Option<String> {
    let mut kept = String::with_capacity(text.len());
    let mut keeping = false;
    for line in text.split_inclusive('\n') {
        if stamp::leads(line) {
            keeping = line[..stamp::WIDTH] >= *cutoff;
        }
        if keeping {
            kept.push_str(line);
        }
    }
    if !kept.is_empty() && !kept.ends_with('\n') {
        kept.push('\n');
    }
    (kept != text).then_some(kept)
}

/// Prunes the file at `path` as of `now`: read it, and when something is older
/// than [`RETENTION`], write the rest beside it and rename over it, so a crash
/// mid-prune leaves the old file, never half of one. A missing file is nothing
/// to do.
///
/// A line another process appends between the read and the rename is lost; two
/// games starting in the same second is the only way to meet that, and the
/// line lost is a startup line.
///
/// # Errors
/// The read, the temporary write or the rename failing.
pub fn prune_file(path: &Path, now: SystemTime) -> io::Result<()> {
    let text = match fs::read(path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    let cutoff = stamp::format(now.checked_sub(RETENTION).unwrap_or(SystemTime::UNIX_EPOCH));
    let Some(kept) = prune_text(&text, &cutoff) else {
        return Ok(());
    };
    let tmp = path.with_extension(format!("log.{}.tmp", std::process::id()));
    fs::write(&tmp, kept)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}
