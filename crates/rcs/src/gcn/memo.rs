//! [`super::scan`] memoised by file content, for as long as a caller asks.
//!
//! One PS4 model build asks each material file several questions (its normal map, its `v`
//! flip, its sea, its crowd frame, ...), each through [`super::scan`], and a circuit's
//! materials share their files. Without the memo every question decodes every program of the
//! file again: the Omega circuit census scanned 895 distinct files 60,504 times.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use super::{Program, Result};

/// Every program of one file, as [`super::scan`] returns them.
pub type Scanned = Arc<[(usize, Result<Program>)]>;

/// Bucket key to the files in that bucket, each with its programs.
type Memo = HashMap<u64, Vec<(Arc<[u8]>, Scanned)>>;

thread_local! {
    static MEMO: RefCell<Option<Memo>> = const { RefCell::new(None) };
}

/// Runs `f` with [`super::scan`] memoised on this thread. The memo, and the copies of the
/// files it holds, are dropped when `f` returns; a nested call shares the outer memo.
pub fn memoised<R>(f: impl FnOnce() -> R) -> R {
    struct Close(bool);
    impl Drop for Close {
        fn drop(&mut self) {
            if self.0 {
                MEMO.with_borrow_mut(|memo| *memo = None);
            }
        }
    }
    let opened = MEMO.with_borrow_mut(|memo| {
        let open = memo.is_none();
        if open {
            *memo = Some(HashMap::new());
        }
        open
    });
    let _close = Close(opened);
    f()
}

/// The length and the first and last 64 bytes: cheap, and distinct enough to keep the buckets
/// short. Equality is decided on the whole file.
fn bucket(file: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    file.len().hash(&mut hasher);
    file[..file.len().min(64)].hash(&mut hasher);
    file[file.len().saturating_sub(64)..].hash(&mut hasher);
    hasher.finish()
}

/// `file`'s programs out of the memo, or `scan`'s, stored when a memo is open.
pub(super) fn get_or(file: &[u8], scan: impl FnOnce() -> Scanned) -> Scanned {
    let key = bucket(file);
    let hit = MEMO.with_borrow(|memo| {
        memo.as_ref().map(|memo| {
            memo.get(&key)?
                .iter()
                .find(|(bytes, _)| **bytes == *file)
                .map(|(_, scanned)| Arc::clone(scanned))
        })
    });
    match hit {
        None => scan(),
        Some(Some(scanned)) => scanned,
        Some(None) => {
            let scanned = scan();
            MEMO.with_borrow_mut(|memo| {
                if let Some(memo) = memo {
                    memo.entry(key)
                        .or_default()
                        .push((Arc::from(file), Arc::clone(&scanned)));
                }
            });
            scanned
        }
    }
}
