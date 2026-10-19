//! The soundtrack listings a run has already read, shared by every clone of
//! one [`crate::MusicDiscs`].
//!
//! **Why it exists (measured on the Steam Deck, 2026-10-19).** Reading Wipeout
//! HD's listing measures every track off a PSARC, 305 ms on the Deck. It used
//! to run up to three times a run, once on the frame thread on the first race
//! tick: a 248 to 264 ms frame. Every read now runs on a music worker, and only
//! the first one pays; [`Listings::known`] answers the frame thread without
//! reading at all.
//!
//! Only a listing that was read is kept. An error or a source with no
//! soundtrack is asked again next time, so a transient failure is still
//! retried. The lock is held for the lookup and the insert only, never across
//! the read, because on the browser's page thread [`oag_thread::lock`] spins.

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
        if let Some(found) = self.known(source, platform) {
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

    /// The listing for `source` if it was already read; never reads.
    pub(crate) fn known(&self, source: &str, platform: Platform) -> Option<Soundtrack> {
        let kept = oag_thread::lock(&self.0).ok()?;
        kept.iter()
            .find(|(s, p, _)| s == source && *p == platform)
            .map(|(_, _, soundtrack)| soundtrack.clone())
    }
}
