//! A bounded memo of [`crate::Archives::read_name`] results, for one load.
//!
//! A race load asks for the same small entries over and over: every material
//! slot of every model reads its `.rcsmaterial` once per pass that inspects it
//! (skin, roles, water, ice, refraction, light cone, mag wave, ...), and every
//! pad class repeats the lot. On HD Fury's Vineta K that was 13,828 reads of 99
//! distinct materials - 2.6 GB inflated for 5.8 MB of data, measured
//! 2026-10-07 in `docs/architecture/load-time.md`.
//!
//! Off unless asked for ([`crate::Archives::memoising_reads`]), because an
//! [`crate::Archives`] that lives for a whole session would otherwise hold the
//! memo for as long as it does. Only entries up to [`MAX_ENTRY`] bytes are
//! kept, and no more than [`BUDGET`] in all: the entries a load re-reads are
//! the small ones, and a texture or a music track read twice is cheaper to
//! read twice than to pin for the rest of the load.

use std::collections::HashMap;

/// The largest entry kept. Every `.rcsmaterial` on HD, 2048 and Omega is
/// under 0.6 MB.
pub const MAX_ENTRY: usize = 1 << 20;

/// The most bytes kept in all, over every entry.
pub const BUDGET: usize = 64 << 20;

/// What has been read, by the name it was asked for.
#[derive(Debug, Default)]
pub struct ReadMemo {
    entries: HashMap<String, Vec<u8>>,
    bytes: usize,
    /// Reads answered from the memo.
    pub hits: usize,
    /// Reads that went to an archive.
    pub misses: usize,
}

impl ReadMemo {
    /// A copy of `name`'s bytes if it was read before, counting the ask.
    pub fn get(&mut self, name: &str) -> Option<Vec<u8>> {
        let found = self.entries.get(name).cloned();
        if found.is_some() {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
        found
    }

    /// Keeps `blob` as `name`'s bytes, if it fits.
    pub fn keep(&mut self, name: &str, blob: &[u8]) {
        if blob.len() > MAX_ENTRY || self.bytes + blob.len() > BUDGET {
            return;
        }
        self.bytes += blob.len();
        self.entries.insert(name.to_string(), blob.to_vec());
    }

    /// One line for a load report.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "archive reads: {} of {} answered from the read memo ({} entries, {} KiB kept)",
            self.hits,
            self.hits + self.misses,
            self.entries.len(),
            self.bytes / 1024
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_read_is_a_hit_and_an_oversized_entry_is_never_kept() {
        let mut memo = ReadMemo::default();
        assert_eq!(memo.get("a"), None);
        memo.keep("a", b"bytes");
        assert_eq!(memo.get("a").as_deref(), Some(&b"bytes"[..]));
        memo.keep("big", &vec![0; MAX_ENTRY + 1]);
        assert_eq!(memo.get("big"), None);
        assert_eq!((memo.hits, memo.misses), (1, 2));
    }

    #[test]
    fn the_budget_bounds_what_is_kept() {
        let mut memo = ReadMemo::default();
        let entry = vec![0; MAX_ENTRY];
        for n in 0..(BUDGET / MAX_ENTRY + 2) {
            memo.keep(&n.to_string(), &entry);
        }
        assert_eq!(memo.bytes, BUDGET);
    }
}
