//! Opened CHD handles waiting for the next [`ChdSource::open`] of the same file.
//!
//! Opening a CHD decodes its whole hunk map (a Huffman walk and a CRC-16 per hunk), about
//! 9 ms for a PSP disc and more for a PS2 one, and a running game opens the same few images
//! a hundred times: every `DiscImage::open`, each worker of a split read, each track's
//! length. A source that is dropped hands its handle here, and an open of the same file takes
//! it back with the hunk map already decoded. A handle belongs to one source at a time, so no
//! two threads share a read position.
//!
//! A handle is found by the identity of the file it was opened on, read from the open handle
//! (never from the path before the open): two spellings of one file share it, and a file
//! replaced under the same path does not.

use std::path::Path;
use std::sync::Mutex;

use super::{Chd, ChdSource, UnitLayout};

/// Which file a handle was opened on.
#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Identity {
    device: u64,
    inode: u64,
    len: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

#[cfg(unix)]
impl Identity {
    /// `None` when the open file cannot be stat-ed; such a file is never pooled.
    pub(super) fn of(file: &std::fs::File, _path: &Path) -> Option<Self> {
        use std::os::unix::fs::MetadataExt;
        let metadata = file.metadata().ok()?;
        Some(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            len: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        })
    }
}

/// Which file a handle was opened on: where there is no inode to read, the resolved path and
/// what the open file says of its size and modification time.
#[cfg(not(unix))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Identity {
    path: std::path::PathBuf,
    len: u64,
    modified: std::time::SystemTime,
}

#[cfg(not(unix))]
impl Identity {
    /// `None` when the file cannot be resolved or stat-ed; such a file is never pooled.
    pub(super) fn of(file: &std::fs::File, path: &Path) -> Option<Self> {
        let metadata = file.metadata().ok()?;
        Some(Self {
            path: std::fs::canonicalize(path).ok()?,
            len: metadata.len(),
            modified: metadata.modified().ok()?,
        })
    }
}

/// An idle source: everything [`ChdSource`] holds but its path.
struct Idle {
    chd: Chd<std::io::BufReader<crate::mount::ImageFile>>,
    layout: UnitLayout,
    sectors_per_hunk: u32,
    sector_count: u32,
    hunk_buf: Vec<u8>,
    compressed_buf: Vec<u8>,
    cached_hunk: Option<u32>,
}

/// The most idle handles kept in all: one per worker of a split read, and a few more for the
/// threads that open the other discs beside it. Each holds a file descriptor, a decoded hunk
/// map and a hunk buffer.
const MAX_IDLE: usize = 16;

/// Idle values by identity, the least recently returned first.
struct Shelf<I, T> {
    entries: Vec<(I, T)>,
}

impl<I: PartialEq, T> Shelf<I, T> {
    const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// The most recently returned value of `id`.
    fn take(&mut self, id: &I) -> Option<T> {
        let at = self.entries.iter().rposition(|(held, _)| held == id)?;
        Some(self.entries.remove(at).1)
    }

    /// Keeps `value`, dropping the least recently returned one when `cap` are held.
    fn give(&mut self, id: I, value: T, cap: usize) -> Option<T> {
        self.entries.push((id, value));
        (self.entries.len() > cap).then(|| self.entries.remove(0).1)
    }
}

static IDLE: Mutex<Shelf<Identity, Idle>> = Mutex::new(Shelf::new());

fn shelf() -> std::sync::MutexGuard<'static, Shelf<Identity, Idle>> {
    IDLE.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// An idle source opened on the file of this identity, now for `path`.
/// Borrows `id`: off Unix it holds a `PathBuf` and is not `Copy`, and the caller still needs it.
pub(super) fn take(path: &Path, id: &Identity) -> Option<ChdSource> {
    let idle = shelf().take(id)?;
    Some(ChdSource {
        chd: Some(idle.chd),
        path: path.to_path_buf(),
        layout: idle.layout,
        sectors_per_hunk: idle.sectors_per_hunk,
        sector_count: idle.sector_count,
        hunk_buf: idle.hunk_buf,
        compressed_buf: idle.compressed_buf,
        cached_hunk: idle.cached_hunk,
        // Off Unix `Identity` holds a `PathBuf`, so this clone is not a copy there.
        #[allow(clippy::clone_on_copy)]
        stamp: Some(id.clone()),
    })
}

/// Keeps the handle of a source being dropped, unless its file has no identity.
pub(super) fn give_back(source: &mut ChdSource) {
    let (Some(chd), Some(id)) = (source.chd.take(), source.stamp.take()) else {
        return;
    };
    let idle = Idle {
        chd,
        layout: source.layout,
        sectors_per_hunk: source.sectors_per_hunk,
        sector_count: source.sector_count,
        hunk_buf: std::mem::take(&mut source.hunk_buf),
        compressed_buf: std::mem::take(&mut source.compressed_buf),
        cached_hunk: source.cached_hunk,
    };
    // Dropped after the lock is released: closing a file is not done under it.
    let evicted = shelf().give(id, idle, MAX_IDLE);
    drop(evicted);
}

#[cfg(test)]
mod tests;
