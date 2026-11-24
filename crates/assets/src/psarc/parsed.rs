//! The parsed half of a PSARC archive, shared by every open of the same bytes.
//!
//! A launch opens the same archives on several threads (the soundtrack scan, the
//! title's source, the launcher's survey), and each open inflated the manifest,
//! hashed every manifest path to match it to its entry and built the path index.
//! That work depends on nothing but the archive's table of contents and its stored
//! manifest, so it is kept by those bytes: the first open of an archive does it,
//! the others wait for it and take the result.
//!
//! Keyed by content, not by file identity, so every [`super::BlobSource`] (a file,
//! a server extraction's sparse stand-in, an image on a disc) shares through one
//! rule and a file replaced under its path cannot return the old parse.

use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use oag_formats::psarc::Directory;

/// What opening an archive derives from its table of contents and manifest.
pub(super) struct Parsed {
    pub(super) directory: Directory,
    pub(super) paths: Vec<String>,
    /// See [`super::Archive`]'s old field of this name: the directory index storing `paths[n]`.
    pub(super) entry_of_path: Vec<usize>,
    /// [`super::normalise`]d path to its position in `paths`.
    pub(super) index: HashMap<String, usize>,
}

/// How many archives are kept. A launch opens about fifty (every title's archives on the
/// disk); the oldest goes first.
const KEPT: usize = 128;

/// The table of contents and the stored manifest, as lengths and two 64-bit digests.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    toc: (usize, u64),
    manifest: (usize, u64),
}

impl Key {
    fn of(toc: &[u8], stored_manifest: &[u8]) -> Self {
        let digest = |bytes: &[u8]| {
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            (bytes.len(), hasher.finish())
        };
        Self {
            toc: digest(toc),
            manifest: digest(stored_manifest),
        }
    }
}

/// One archive's slot: empty while its first opener parses, so a second opener of the same
/// bytes waits on the lock instead of parsing again.
type Slot = Arc<Mutex<Option<Arc<Parsed>>>>;

#[derive(Default)]
struct Shelf {
    slots: HashMap<Key, Slot>,
    oldest_first: VecDeque<Key>,
}

impl Shelf {
    fn slot(&mut self, key: Key) -> Slot {
        if let Some(slot) = self.slots.get(&key) {
            return slot.clone();
        }
        if self.oldest_first.len() >= KEPT
            && let Some(oldest) = self.oldest_first.pop_front()
        {
            self.slots.remove(&oldest);
        }
        self.oldest_first.push_back(key);
        self.slots.entry(key).or_default().clone()
    }
}

static SHELF: LazyLock<Mutex<Shelf>> = LazyLock::new(Mutex::default);

/// The parse of the archive with this table of contents and stored manifest: kept from an
/// earlier open of the same bytes, else `parse`'s result, which is kept unless it failed.
pub(super) fn shared<E>(
    toc: &[u8],
    stored_manifest: &[u8],
    parse: impl FnOnce() -> Result<Parsed, E>,
) -> Result<Arc<Parsed>, E> {
    let slot = SHELF
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .slot(Key::of(toc, stored_manifest));
    let mut kept = slot.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(parsed) = kept.as_ref() {
        return Ok(parsed.clone());
    }
    let parsed = Arc::new(parse()?);
    *kept = Some(parsed.clone());
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    fn parsed(entries: usize) -> Parsed {
        Parsed {
            directory: Directory {
                header: oag_formats::psarc::Header {
                    version_major: 1,
                    version_minor: 4,
                    compression: *b"zlib",
                    toc_len: 32,
                    entry_len: 30,
                    entry_count: 0,
                    block_size: 65536,
                    flags: 3,
                },
                entries: Vec::new(),
                blocks: Vec::new(),
                block_width: 2,
            },
            paths: vec!["/a".to_string(); entries],
            entry_of_path: Vec::new(),
            index: HashMap::new(),
        }
    }

    #[test]
    fn the_same_bytes_parse_once() {
        let runs = AtomicUsize::new(0);
        let open = |toc: &[u8], manifest: &[u8]| {
            shared::<()>(toc, manifest, || {
                runs.fetch_add(1, Ordering::SeqCst);
                Ok(parsed(3))
            })
            .expect("parses")
        };
        let first = open(b"toc-once", b"manifest-once");
        let second = open(b"toc-once", b"manifest-once");
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(runs.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_different_table_or_manifest_parses_again() {
        let runs = AtomicUsize::new(0);
        let open = |toc: &[u8], manifest: &[u8]| {
            shared::<()>(toc, manifest, || {
                runs.fetch_add(1, Ordering::SeqCst);
                Ok(parsed(1))
            })
            .expect("parses")
        };
        let base = open(b"toc-differs", b"manifest-differs");
        let other_toc = open(b"toc-differs!", b"manifest-differs");
        let other_manifest = open(b"toc-differs", b"manifest-differs!");
        assert!(!Arc::ptr_eq(&base, &other_toc));
        assert!(!Arc::ptr_eq(&base, &other_manifest));
        assert_eq!(runs.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn a_failed_parse_is_tried_again_not_kept() {
        let runs = AtomicUsize::new(0);
        let failed = shared(b"toc-fails", b"manifest-fails", || {
            runs.fetch_add(1, Ordering::SeqCst);
            Err::<Parsed, _>("bad manifest")
        });
        assert_eq!(failed.err(), Some("bad manifest"));
        let retried = shared::<&str>(b"toc-fails", b"manifest-fails", || {
            runs.fetch_add(1, Ordering::SeqCst);
            Ok(parsed(2))
        })
        .expect("the second open parses");
        assert_eq!(retried.paths.len(), 2);
        assert_eq!(runs.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn threads_opening_the_same_bytes_at_once_parse_once() {
        let runs = AtomicUsize::new(0);
        let results: Vec<Arc<Parsed>> = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..6)
                .map(|_| {
                    scope.spawn(|| {
                        shared::<()>(b"toc-racing", b"manifest-racing", || {
                            runs.fetch_add(1, Ordering::SeqCst);
                            std::thread::sleep(std::time::Duration::from_millis(30));
                            Ok(parsed(4))
                        })
                        .expect("parses")
                    })
                })
                .collect();
            threads
                .into_iter()
                .map(|thread| thread.join().expect("no panic"))
                .collect()
        });
        assert_eq!(runs.load(Ordering::SeqCst), 1);
        assert!(
            results
                .windows(2)
                .all(|pair| Arc::ptr_eq(&pair[0], &pair[1]))
        );
    }

    #[test]
    fn the_oldest_archive_is_dropped_past_the_limit() {
        let mut shelf = Shelf::default();
        let key = |n: usize| Key::of(&n.to_le_bytes(), b"manifest-evicts");
        for n in 0..KEPT + 5 {
            shelf.slot(key(n));
        }
        assert_eq!(shelf.slots.len(), KEPT);
        assert!(!shelf.slots.contains_key(&key(0)));
        assert!(shelf.slots.contains_key(&key(KEPT + 4)));
    }
}
