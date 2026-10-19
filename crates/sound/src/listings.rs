//! The soundtrack listings a run has already read, shared by every clone of
//! one [`crate::MusicDiscs`].
//!
//! **Why it exists (measured on the Steam Deck, 2026-10-19).** Reading Wipeout
//! HD's listing measures every track, and a PSARC entry cannot be peeked, so
//! the read decompresses all fifteen tracks whole (about 75 MiB). A `--race`
//! run read it three times, and the third read ran on the frame thread on
//! the first race tick (`Audio::race_soundtrack_len`). That one frame took
//! 248 to 264 ms on the Deck, and the 120-frame rolling mean in `--frame-log`
//! turned it into the "28 fps worst second" HD was said to drop to. A race
//! resumed from the menus took the same path. With the listing kept here,
//! only the first read pays.
//!
//! Only a listing that was read is kept. An error or a source with no
//! soundtrack is asked again next time, so a transient failure is still
//! retried, as `race_soundtrack_len` never keeps a `0`. The lock is held for
//! the lookup and the insert only, never across the read, because on the
//! browser's page thread [`oag_thread::lock`] spins.

use std::sync::{Arc, Mutex};

use anyhow::Result;
use oag_disc::Platform;

use crate::Soundtrack;
use crate::library::Library;

/// Listings already read, by source and platform.
#[derive(Debug, Clone, Default)]
pub struct Listings(Arc<Mutex<Vec<(String, Platform, Soundtrack)>>>);

impl Listings {
    /// The soundtrack `source` carries, read once per run and then kept.
    ///
    /// # Errors
    ///
    /// As [`Soundtrack::read`]; an error is not kept.
    pub(crate) fn read(
        &self,
        library: &dyn Library,
        source: &str,
        platform: Platform,
    ) -> Result<Option<Soundtrack>> {
        if let Some(found) = self.find(source, platform) {
            return Ok(Some(found));
        }
        let read = Soundtrack::read(library, source, platform)?;
        if let Some(soundtrack) = &read
            && let Ok(mut kept) = oag_thread::lock(&self.0)
            && !kept.iter().any(|(s, p, _)| s == source && *p == platform)
        {
            kept.push((source.to_string(), platform, soundtrack.clone()));
        }
        Ok(read)
    }

    fn find(&self, source: &str, platform: Platform) -> Option<Soundtrack> {
        let kept = oag_thread::lock(&self.0).ok()?;
        kept.iter()
            .find(|(s, p, _)| s == source && *p == platform)
            .map(|(_, _, soundtrack)| soundtrack.clone())
    }
}
