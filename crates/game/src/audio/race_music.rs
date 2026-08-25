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

/// The race-playlist half of `impl Audio`, kept beside [`RaceMusicWorker`]
/// rather than in `audio.rs` itself: every method here either produces or
/// consumes a [`RaceMusicWorker`]'s result, so the two belong to the same
/// question - "how does the race's music get fetched" - even though nothing
/// stops an inherent `impl` block living in a different file from the type it
/// is on.
impl Audio {
    /// The race-launch counterpart of [`Self::start_race_music`]: applies a
    /// track that a [`RaceMusicWorker`], spawned earlier, has already located
    /// and decoded - so nothing here reads off the disc, and this never
    /// blocks the caller the way [`Self::start_race_music`] can on a cold
    /// cache.
    ///
    /// This is the fix for the gap that otherwise opens up between the
    /// loading screen's fade and the first race frame: decoding a full track
    /// synchronously, at the hand-off, measured at 2.83 seconds for a cold PS2
    /// track - all of it spent before this method ever existed, on the frame
    /// thread, with the fade already faded out.
    ///
    /// `index` was reserved by [`Self::reserve_race_music_index`] before
    /// `worker` was spawned - see `Session::launch_race` - so this reads
    /// [`Self::race_index`] back rather than taking it as a parameter: the
    /// two are guaranteed to agree without asking `discs` a second time.
    pub fn finish_race_music(
        &mut self,
        mut worker: RaceMusicWorker,
        discs: &MusicDiscs,
        choice: MusicSource,
        cache_dir: &Path,
    ) {
        if let Some(id) = self.music.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }
        // Defensive rather than load-bearing, the same as the identical guard
        // in `start_race_music`: nothing calls this while a race voice is
        // already sounding, but a wrong assumption here would otherwise leak
        // a voice rather than fail loudly.
        if let Some(id) = self.race_voice.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }
        let index = self.race_index.expect(
            "Session::launch_race reserves this via reserve_race_music_index before spawning \
             the worker this method takes",
        );
        let seek = (self.race_position > 0.0).then_some(self.race_position);
        self.race_context = Some((discs.clone(), choice, cache_dir.to_path_buf()));
        // Never actually waits: this is only called once the loading screen's
        // own `race_ready` has seen `worker.is_finished()` true - see
        // `LoadingStage::race_ready`.
        let fetched = worker.join().unwrap_or_else(|| {
            Err(anyhow::anyhow!(
                "the race music fetch thread would not start"
            ))
        });
        self.apply_fetched_race_track(index, fetched, seek);
        if self.race_voice.is_some() {
            self.race_position = 0.0;
        }
    }

    /// Reserves which track index the next race's music means, without
    /// fetching it.
    ///
    /// The split that lets a caller spawn a [`RaceMusicWorker`] for that index
    /// before the mixer - or the disc - is touched at all:
    /// [`Self::start_race_music`] used to do this reservation and the fetch in
    /// the same breath, which was fine when both were synchronous and cheap
    /// enough to not notice; spawning a worker for the fetch half needs the
    /// index settled first, so `Session::launch_race` can hand it to
    /// [`RaceMusicWorker::spawn`] before the loading screen even goes up.
    pub fn reserve_race_music_index(&mut self, discs: &MusicDiscs) -> usize {
        if self.race_index.is_none() {
            self.race_index = Some(Self::initial_race_index(&self.music_from, discs));
        }
        self.race_index.expect("set immediately above")
    }

    /// Loads and plays one race-playlist track, reporting what happened.
    ///
    /// Shared by [`Self::start_race_music`] and the advance-on-finish check in
    /// [`Self::tick`], which is the only other place `race_index` moves.
    /// `seek` is `Some` for a resume and `None` for a fresh start (index 0 of
    /// the track, which is also where a freshly-advanced track begins).
    ///
    /// `pub(super)`: `Audio::start_race_music` calls this too, and stayed in
    /// `audio.rs` rather than moving here - it is the general entry point,
    /// not a race-music-fetch specific one.
    pub(super) fn play_race_track(
        &mut self,
        discs: &MusicDiscs,
        choice: MusicSource,
        cache_dir: &Path,
        index: usize,
        seek: Option<f64>,
    ) {
        let fetched = self.fetch_indexed(discs, choice, cache_dir, index);
        self.apply_fetched_race_track(index, fetched, seek);
    }

    /// The "start a voice on what was fetched" half of [`Self::play_race_track`],
    /// split out so [`Self::finish_race_music`] can share it over a track
    /// [`RaceMusicWorker`] fetched instead of [`Self::fetch_indexed`]. Neither
    /// caller does anything to the disc from here on - this only ever touches
    /// the mixer and this struct's own race-playlist state.
    fn apply_fetched_race_track(
        &mut self,
        index: usize,
        fetched: Result<Option<Loaded>>,
        seek: Option<f64>,
    ) {
        match fetched {
            Ok(Some(loaded)) => {
                let seconds = loaded.sound.seconds();
                self.race_cache = loaded
                    .from
                    .map(|platform| (platform, index, Arc::clone(&loaded.sound)));
                self.race_voice = self.output.with_mixer(|mixer| {
                    let id = mixer.play(Play::once(loaded.sound, Bus::Music))?;
                    if let Some(seek) = seek {
                        mixer.seek(id, seek);
                    }
                    Some(id)
                });
                self.race_from = self.race_voice.and(loaded.from);
                match seek {
                    Some(seek) => info!(
                        "audio: race music {}, {seconds:.1} s, resuming from {seek:.1} s",
                        loaded.what
                    ),
                    None => info!("audio: race music {}, {seconds:.1} s", loaded.what),
                }
            }
            Ok(None) => info!("audio: this source carries no race music this can play"),
            Err(error) => warn!("audio: no race music ({error:#})"),
        }
    }

    /// Starts fetching the race playlist's next track as soon as the current
    /// one is playing, rather than waiting until it is close to ending - see
    /// [`Audio::race_prefetch`] for why this exists and what it replaces.
    ///
    /// **Immediately, not "a few seconds before the end".** A remaining-time
    /// threshold was tried first and rejected: it answers "will the natural
    /// end of this track hitch" but not "will *skipping* it hitch", and
    /// skipping is not remotely rare enough to leave unanswered - a `MUSIC
    /// SOURCE`-style row, or any future skip control, can end a track at any
    /// point in it, and the fetch has to already exist for that to be free.
    /// The cost is holding one extra track's PCM (tens of MiB) alongside
    /// [`Self::race_cache`]'s own single slot for most of a track's length
    /// instead of its last few seconds - proportionate, next to what a pause
    /// resume already costs by the same measure.
    ///
    /// A no-op on every tick but the one right after a track starts: outside
    /// a race, or once [`Self::race_prefetch`] is already `Some`.
    pub(super) fn maybe_prefetch_next_race_track(&mut self) {
        if self.race_prefetch.is_some() || self.race_voice.is_none() {
            return;
        }
        let Some(index) = self.race_index else {
            return;
        };
        let Some((discs, choice, cache_dir)) = self.race_context.clone() else {
            return;
        };
        let next = next_race_index(index, Self::booted_soundtrack_len(&discs));
        self.race_prefetch = Some((next, RaceMusicWorker::spawn(discs, choice, cache_dir, next)));
    }

    /// Moves the race playlist on to the next track once the current one has
    /// finished naturally - race tracks play [`Play::once`], never looping,
    /// which is what makes "no longer playing" mean "reached its end" rather
    /// than "was stopped".
    ///
    /// **Joins [`Self::race_prefetch`] rather than fetching from nothing**,
    /// which is what this used to do - a disc read and, on the PSP, a possible
    /// `ffmpeg` decode, 0.4-2.8 s by this module's own measurements, paid
    /// inside the fixed-timestep loop on every track boundary. A prefetch
    /// [`Self::maybe_prefetch_next_race_track`] started early almost always
    /// means [`RaceMusicWorker::join`] returns immediately here instead;
    /// falls back to the old synchronous fetch only when there is no
    /// prefetch to trust - none started in time, or one started for a track
    /// this is no longer the same question as (a source change since - the
    /// index check below is what catches that).
    pub(super) fn advance_race_track(&mut self) {
        let Some((discs, choice, cache_dir)) = self.race_context.clone() else {
            return;
        };
        let Some(index) = self.race_index else {
            return;
        };
        // Stopped explicitly rather than assumed already silent: `Audio::tick`'s
        // own call only ever reaches here once the voice has already ended on
        // its own, but nothing about this method's name or signature says a
        // caller may not reach it with one still sounding - a skip control
        // would - and `mixer.stop` on an id whose voice already finished is a
        // no-op (voice ids carry a generation), so this costs nothing on the
        // path that exists today.
        if let Some(id) = self.race_voice.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }
        let next = next_race_index(index, Self::booted_soundtrack_len(&discs));
        self.race_index = Some(next);

        if let Some((prefetch_index, mut worker)) = self.race_prefetch.take()
            && prefetch_index == next
        {
            // Never actually waits in the ordinary case - see this method's
            // own doc - but can, if the boundary arrived before the fetch
            // did; still strictly no worse than the synchronous path below,
            // since that decode is exactly what this worker is already most
            // of the way through.
            let fetched = worker.join().unwrap_or_else(|| {
                Err(anyhow::anyhow!(
                    "the race music prefetch thread would not start"
                ))
            });
            self.apply_fetched_race_track(next, fetched, None);
            return;
        }
        self.play_race_track(&discs, choice, &cache_dir, next, None);
    }
}
