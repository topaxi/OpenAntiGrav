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

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use oag_disc::Platform;

use crate::Soundtrack;
use crate::library::Library;

/// Listings already read, by source and platform.
#[derive(Debug, Clone, Default)]
pub struct Listings {
    read: Arc<Mutex<Vec<(String, Platform, Soundtrack)>>>,
    /// A bank-driven title's playlist and bank, by source: what Omega's
    /// `load_entry` used to read and parse again for every track.
    prepared: Arc<Mutex<Vec<(String, Arc<oag_music::omega::Prepared>)>>>,
    /// How many times a bank was read and parsed, for a test to count.
    prepares: Arc<AtomicU32>,
}

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
            && let Ok(mut kept) = oag_thread::lock(&self.read)
            && !kept.iter().any(|(s, p, _)| s == source && *p == platform)
        {
            kept.push((source.to_string(), platform, soundtrack.clone()));
        }
        Ok(read)
    }

    /// The listing for `source` if it was already read; never reads.
    pub(crate) fn known(&self, source: &str, platform: Platform) -> Option<Soundtrack> {
        let kept = oag_thread::lock(&self.read).ok()?;
        kept.iter()
            .find(|(s, p, _)| s == source && *p == platform)
            .map(|(_, _, soundtrack)| soundtrack.clone())
    }

    /// The bank-driven playlist for `source`, built by `build` the first time
    /// and kept; `Ok(None)` (a title with no bank) is asked again.
    ///
    /// `build` runs outside the lock, so two racing workers may both build and
    /// the first insert wins.
    ///
    /// # Errors
    ///
    /// As `build`; an error is not kept.
    pub(crate) fn prepared(
        &self,
        source: &str,
        build: impl FnOnce() -> Result<Option<oag_music::omega::Prepared>>,
    ) -> Result<Option<Arc<oag_music::omega::Prepared>>> {
        let find = |kept: &Vec<(String, Arc<oag_music::omega::Prepared>)>| {
            kept.iter()
                .find(|(s, _)| s == source)
                .map(|(_, p)| Arc::clone(p))
        };
        if let Ok(kept) = oag_thread::lock(&self.prepared)
            && let Some(found) = find(&kept)
        {
            return Ok(Some(found));
        }
        let Some(built) = build()? else {
            return Ok(None);
        };
        self.prepares.fetch_add(1, Ordering::Relaxed);
        let built = Arc::new(built);
        if let Ok(mut kept) = oag_thread::lock(&self.prepared) {
            if let Some(found) = find(&kept) {
                return Ok(Some(found));
            }
            kept.push((source.to_string(), Arc::clone(&built)));
        }
        Ok(Some(built))
    }

    /// How many banks were read and parsed so far.
    pub(crate) fn bank_reads(&self) -> u32 {
        self.prepares.load(Ordering::Relaxed)
    }
}
