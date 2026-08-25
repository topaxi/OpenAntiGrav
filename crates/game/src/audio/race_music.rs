//! [`RaceMusicWorker`]: locating and decoding the race's music on a thread of
//! its own, so the race hand-off does not wait on it.
//!
//! Split out of `audio.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change beyond what
//! landed with it in the same change (see [`RaceMusicWorker`]'s own doc).

use super::*;

/// Which track of `platform`'s soundtrack `index` means, `index` being an
/// entry in the **booted** disc's own order.
///
/// On the booted release that is simply its own entry `index`. On the other
/// one it is whichever track is the same length, which is what makes the two
/// selections the same recording rather than two unrelated pieces of music.
///
/// **The index is not stable across boots**, and it is worth being plain
/// about that: an index means "entry N of whichever disc booted", and the two
/// archives are not in the same order. Entry 0 happens to name the same
/// recording on both - the PS2's first track is also the first of the PSP's
/// sixteen in `Data.wad` order - but that is coincidence, and at index 1 the
/// two boots start on different music. `MUSIC_TRACK` gets away with never
/// noticing because it is a constant; `Audio::race_index` is not, and this
/// is exactly why it is kept and read in the booted disc's own order rather
/// than in whichever platform happens to be playing at the time - a future
/// circuit-to-track map has to be built the same way.
///
/// A free function rather than an `Audio` method, though every caller so far
/// has one in hand: nothing here reads `self`, and that is what lets
/// [`fetch_race_track`] call it from a thread that does not own `Audio` at
/// all.
///
/// `pub(super)`: `Audio::fetch`/`Audio::fetch_indexed` call this too, and both
/// predate the split that moved it here alongside [`RaceMusicWorker`].
pub(super) fn locate(
    discs: &MusicDiscs,
    platform: Platform,
    source: &str,
    index: usize,
) -> Result<Option<Track>> {
    let Some(soundtrack) = Soundtrack::read(source, platform)? else {
        return Ok(None);
    };
    if discs.booted() == Some(platform) {
        return Ok(soundtrack.tracks.get(index).copied());
    }

    let Some((booted_source, booted_platform)) = discs.pick(MusicSource::Auto) else {
        return Ok(soundtrack.tracks.get(index).copied());
    };
    let Some(booted) = Soundtrack::read(booted_source, booted_platform)? else {
        return Ok(soundtrack.tracks.get(index).copied());
    };
    let Some(wanted) = booted.tracks.get(index) else {
        return Ok(None);
    };
    Ok(soundtrack.nearest(wanted.seconds))
}

/// The race's music, being located and decoded on a thread of its own - the
/// same shape [`crate::race::LoadWorker`] and [`crate::boot::MediaWorker`]
/// already have, and for the same reason: fetching it synchronously, at the
/// race hand-off, blocked the frame loop for however long a cold decode
/// took - measured at 2.83s for a full PS2 track, the whole of a gap that
/// otherwise opened up between the loading screen's fade and the first race
/// frame.
///
/// Holds no [`Audio`] and hands none back: [`fetch_race_track`] produces a
/// plain [`Loaded`], and the mixer only enters at [`Audio::finish_race_music`],
/// which runs on the thread that owns it. That is what makes the split cheap
/// rather than a rewrite - the same argument [`crate::race::worker`] makes
/// for [`crate::race::LoadWorker`].
#[derive(Debug)]
pub struct RaceMusicWorker {
    /// `None` once joined, which is what makes [`Self::join`] idempotent.
    handle: Option<std::thread::JoinHandle<Result<Option<Loaded>>>>,
}

impl RaceMusicWorker {
    /// Starts the fetch in the background.
    ///
    /// `index` is [`Audio::reserve_race_music_index`]'s return value,
    /// reserved before this is called so the worker and the `Audio` it will
    /// hand its result to agree on which track without either one asking
    /// `discs` again or the worker touching the mixer's own state.
    #[must_use]
    pub fn spawn(discs: MusicDiscs, choice: MusicSource, cache_dir: PathBuf, index: usize) -> Self {
        let handle = std::thread::Builder::new()
            // Named for the same reason `race-load` and `boot-media` are: it
            // should be obvious in a debugger and in `top` which thread the
            // window is waiting on.
            .name("race-music".to_string())
            .spawn(move || fetch_race_track(&discs, choice, &cache_dir, index))
            .ok();
        Self { handle }
    }

    /// Whether the fetch has returned.
    ///
    /// `true` for a worker whose thread failed to spawn at all, the same
    /// convention [`crate::race::LoadWorker::is_finished`] uses and for the
    /// same reason: a caller polling this cannot wait forever on a thread
    /// that never started - the error surfaces from [`Self::join`] instead.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.handle
            .as_ref()
            .is_none_or(std::thread::JoinHandle::is_finished)
    }

    /// Takes the result, waiting if it is not in yet.
    ///
    /// **Never actually waits in the frame loop**, which only calls this once
    /// [`Self::is_finished`] is true. `None` on the second call, and on a
    /// worker whose thread would not spawn.
    pub fn join(&mut self) -> Option<Result<Option<Loaded>>> {
        let handle = self.handle.take()?;
        Some(match handle.join() {
            Ok(result) => result,
            // A panic on the fetch thread is reported as a failed fetch rather
            // than resumed here, which would take the window down with it.
            Err(_) => Err(anyhow::anyhow!("the race music fetch panicked")),
        })
    }
}

/// The locate-and-decode half of [`Audio::fetch_indexed`], without its
/// "already read" cache check - a [`RaceMusicWorker`] is spawned before the
/// race it is fetching for exists, so there is no `Audio::race_cache` yet to
/// check against. Free-standing so it can run on a thread that does not own
/// `Audio`'s mixer - see [`RaceMusicWorker::spawn`].
fn fetch_race_track(
    discs: &MusicDiscs,
    choice: MusicSource,
    cache_dir: &Path,
    index: usize,
) -> Result<Option<Loaded>> {
    let Some((source, platform)) = discs.pick(choice) else {
        return Ok(None);
    };
    let Some(track) = locate(discs, platform, source, index)? else {
        return Ok(None);
    };
    let sound = Arc::new(load_track(source, platform, track, cache_dir)?);
    Ok(Some(Loaded {
        from: Some(platform),
        sound,
        what: format!(
            "{platform} soundtrack track {index}, {:.1} s as its own disc lists it",
            track.seconds
        ),
    }))
}
