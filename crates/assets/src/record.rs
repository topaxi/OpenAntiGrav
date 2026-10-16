//! Recording which byte ranges of which archives a run reads.
//!
//! Every archive reader sits on a [`crate::blob_source::BlobSource`], so noting
//! its `(offset, len)` reads there sees everything a race load asks of the
//! disc: table-of-contents parses, presence probes (those read the table too)
//! and entry bytes, whichever path reached them. Off unless [`start`] was
//! called, which only the server extractor does; a normal run pays one relaxed
//! atomic load per read.
//!
//! The recording is process-wide because the readers that notice it are built
//! deep inside a load, far from any value the extractor could hand them.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static RECORDER: Mutex<Option<Recorder>> = Mutex::new(None);
static MISSES: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct Recorder {
    root: String,
    files: BTreeMap<String, Recorded>,
    loose: Vec<String>,
}

struct Recorded {
    len: u64,
    /// Start to end, merged, never touching.
    ranges: BTreeMap<u64, u64>,
}

/// One archive's recorded reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// The archive's path under the source: relative to a folder source's root, the path
    /// on the disc for an image.
    pub path: String,
    /// The archive's length.
    pub len: u64,
    /// `(offset, len)` ascending, merged, never touching.
    pub ranges: Vec<(u64, u64)>,
}

/// What a recording saw.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Recording {
    /// Archives read, with the ranges of each.
    pub files: Vec<File>,
    /// Loose files read whole (an identity file, a movie), by the same kind of path.
    pub loose: Vec<String>,
}

/// Starts recording. `root` is the folder source being read, so its archives are
/// named relative to it; pass the image path for an image (disc entries carry their own).
pub fn start(root: &Path) {
    let mut recorder = RECORDER.lock().expect("recorder lock");
    *recorder = Some(Recorder {
        root: root.to_string_lossy().into_owned(),
        files: BTreeMap::new(),
        loose: Vec::new(),
    });
    ACTIVE.store(true, Ordering::Release);
}

/// Stops recording and hands back what was seen; `None` when none was running.
pub fn stop() -> Option<Recording> {
    ACTIVE.store(false, Ordering::Release);
    let recorder = RECORDER.lock().expect("recorder lock").take()?;
    let files = recorder
        .files
        .into_iter()
        .map(|(path, recorded)| File {
            path,
            len: recorded.len,
            ranges: recorded
                .ranges
                .into_iter()
                .map(|(a, b)| (a, b - a))
                .collect(),
        })
        .collect();
    let mut loose = recorder.loose;
    loose.sort();
    loose.dedup();
    Some(Recording { files, loose })
}

/// The largest single range recorded so far, with its archive: a load that reads most of an
/// archive cannot be extracted usefully, and the extractor says which race did it.
#[must_use]
pub fn largest_range() -> Option<(String, u64)> {
    let guard = RECORDER.lock().expect("recorder lock");
    let recorder = guard.as_ref()?;
    recorder
        .files
        .iter()
        .flat_map(|(path, recorded)| {
            recorded
                .ranges
                .iter()
                .map(move |(start, end)| (path.clone(), end - start))
        })
        .max_by_key(|(_, len)| *len)
}

/// Whether a recording is running.
#[must_use]
pub fn active() -> bool {
    ACTIVE.load(Ordering::Acquire)
}

/// Notes a read of `len` bytes at `offset` of the archive `path` (`total` long).
pub(crate) fn note(path: &str, total: u64, offset: u64, len: u64) {
    if !active() || len == 0 {
        return;
    }
    let mut guard = RECORDER.lock().expect("recorder lock");
    let Some(recorder) = guard.as_mut() else {
        return;
    };
    let key = path
        .strip_prefix(recorder.root.as_str())
        .map_or(path, |rest| rest.trim_start_matches(['/', '\\']))
        .replace('\\', "/");
    let recorded = recorder.files.entry(key).or_insert_with(|| Recorded {
        len: total,
        ranges: BTreeMap::new(),
    });
    let (mut start, mut end) = (offset, offset + len);
    // Swallow every range this one touches or overlaps.
    let touching: Vec<u64> = recorded
        .ranges
        .range(..=end)
        .rev()
        .take_while(|(_, other_end)| **other_end >= start)
        .map(|(other_start, _)| *other_start)
        .collect();
    for other_start in touching {
        if let Some(other_end) = recorded.ranges.remove(&other_start) {
            start = start.min(other_start);
            end = end.max(other_end);
        }
    }
    recorded.ranges.insert(start, end);
}

/// Notes that the loose file `path` (not inside an archive) was read.
pub(crate) fn note_loose(path: &str) {
    if !active() {
        return;
    }
    if let Some(recorder) = RECORDER.lock().expect("recorder lock").as_mut() {
        let key = path
            .strip_prefix(recorder.root.as_str())
            .map_or(path, |rest| rest.trim_start_matches(['/', '\\']))
            .replace('\\', "/");
        recorder.loose.push(key);
    }
}

/// Notes a read a server extraction could not answer. A reader may turn the error into an
/// absent optional asset, so the error alone is not proof nothing was missed: callers that
/// check an extraction ask [`take_misses`] after a race and require it empty.
pub(crate) fn note_miss(what: String) {
    let mut misses = MISSES.lock().expect("misses lock");
    if misses.len() < 64 && !misses.contains(&what) {
        misses.push(what);
    }
}

/// The reads an extraction could not answer since the last call, each once; drains them.
#[must_use]
pub fn take_misses() -> Vec<String> {
    std::mem::take(&mut *MISSES.lock().expect("misses lock"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The recorder is process-wide, so the tests that use it take turns.
    static TURN: Mutex<()> = Mutex::new(());

    /// One test owns the process-wide recorder, so the cases run in sequence.
    #[test]
    fn overlapping_and_touching_reads_merge_and_a_gap_stays() {
        let _turn = TURN
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        start(Path::new("/src"));
        note("/src/a/Data.wad", 1000, 100, 10);
        note("/src/a/Data.wad", 1000, 110, 10);
        note("/src/a/Data.wad", 1000, 105, 3);
        note("/src/a/Data.wad", 1000, 300, 5);
        note("/src/a/Data.wad", 1000, 90, 10);
        note("/src/a/Data.wad", 1000, 0, 0);
        note_loose("/src/UMD_DATA.BIN");
        let recording = stop().expect("was recording");
        assert_eq!(
            recording.files,
            [File {
                path: "a/Data.wad".into(),
                len: 1000,
                ranges: vec![(90, 30), (300, 5)],
            }]
        );
        assert_eq!(recording.loose, ["UMD_DATA.BIN"]);
        assert!(!active());
    }

    /// Reads in any order merge to exactly the bytes they cover: compared with a bitmap.
    #[test]
    fn merging_matches_a_bitmap_for_reads_in_any_order() {
        let _turn = TURN
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = move |n: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % n
        };
        start(Path::new("/src"));
        let mut covered = vec![false; 600];
        for _ in 0..400 {
            let (offset, len) = (next(560), 1 + next(30));
            note("/src/x.wad", 600, offset, len);
            for at in offset..(offset + len).min(600) {
                covered[at as usize] = true;
            }
        }
        let recording = stop().expect("was recording");
        let mut from_ranges = vec![false; 600];
        let mut last_end = None;
        for &(offset, len) in &recording.files[0].ranges {
            assert!(
                last_end.is_none_or(|end| offset > end),
                "ranges touch or overlap"
            );
            last_end = Some(offset + len);
            for at in offset..offset + len {
                from_ranges[at as usize] = true;
            }
        }
        assert_eq!(from_ranges, covered);
    }
}
