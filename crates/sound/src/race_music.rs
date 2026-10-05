//! [`MusicFetchWorker`]: locating and decoding one soundtrack track on a
//! thread of its own, so no caller waits on it - originally the race
//! hand-off, now the race-boundary prefetch and the `MUSIC SOURCE` row too.
//!
//! Split out of `audio.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change beyond what
//! landed with it in the same change (see [`MusicFetchWorker`]'s own doc).

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
/// [`fetch_track`] call it from a thread that does not own `Audio` at
/// all.
///
/// `pub(super)`: `Audio::fetch`/`Audio::fetch_indexed` call this too, and both
/// predate the split that moved it here alongside [`MusicFetchWorker`].
pub(super) fn locate(
    discs: &MusicDiscs,
    platform: Platform,
    source: &str,
    index: usize,
) -> Result<Option<Track>> {
    let Some(soundtrack) = Soundtrack::read(discs.library, source, platform)? else {
        return Ok(None);
    };
    if discs.booted() == Some(platform) {
        return Ok(soundtrack.tracks.get(index).copied());
    }

    let Some((booted_source, booted_platform)) = discs.pick(MusicSource::Auto) else {
        return Ok(soundtrack.tracks.get(index).copied());
    };
    let Some(booted) = Soundtrack::read(discs.library, booted_source, booted_platform)? else {
        return Ok(soundtrack.tracks.get(index).copied());
    };
    let Some(wanted) = booted.tracks.get(index) else {
        return Ok(None);
    };
    Ok(soundtrack.nearest(wanted.seconds))
}

/// One soundtrack track, being located and decoded on a thread of its own -
/// the same shape [`crate::race::LoadWorker`] and [`crate::boot::MediaWorker`]
/// already have, and for the same reason: fetching it synchronously blocked
/// whatever thread asked for however long a cold decode took - measured at
/// 2.83s for a full PS2 track. First built for the race hand-off, where that
/// was the gap between the loading screen's fade and the first race frame;
/// [`Audio::maybe_prefetch_next_race_track`] reuses it for a track boundary,
/// and [`Audio::set_music_source`] for the `MUSIC SOURCE` row - a switch
/// mid-menu or mid-race is exactly the same "decode one indexed track"
/// question, just asked from a different caller.
///
/// Holds no [`Audio`] and hands none back: [`fetch_track`] produces a plain
/// [`Loaded`], and the mixer only enters once a caller applies it on the
/// thread that owns it (`Audio::finish_race_music`, `Audio::advance_race_track`,
/// or `Audio::tick`'s poll of a pending `MUSIC SOURCE` switch). That is what
/// makes the split cheap rather than a rewrite - the same argument
/// [`crate::race::worker`] makes for [`crate::race::LoadWorker`].
#[derive(Debug)]
pub struct MusicFetchWorker {
    /// `None` once joined, which is what makes [`Self::join`] idempotent.
    handle: Option<std::thread::JoinHandle<(Result<Option<Loaded>>, usize)>>,
    /// The booted disc's soundtrack length, read on the same thread as the
    /// fetch and available once [`Self::join`] has returned - see
    /// [`Audio::race_soundtrack_len`].
    soundtrack_len: Option<usize>,
}

impl MusicFetchWorker {
    /// Starts the fetch in the background.
    ///
    /// `index` addresses the booted disc's own soundtrack order, the same
    /// way every caller of [`fetch_track`] does - [`Audio::MUSIC_TRACK`] for
    /// the menu, [`Audio::reserve_race_music_index`]'s return value or
    /// [`Audio::race_index`] for a race.
    ///
    /// `label` names the thread - `"race-music"`, `"race-source"` or
    /// `"menu-source"` today - so a debugger or `top` says which caller is
    /// waiting on it rather than a single name covering all of them; kept
    /// under 15 bytes, since Linux truncates a pthread name past that.
    #[must_use]
    pub fn spawn(
        discs: MusicDiscs,
        choice: MusicSource,
        cache_dir: PathBuf,
        index: usize,
        label: &str,
    ) -> Self {
        let handle = std::thread::Builder::new()
            .name(label.to_string())
            .spawn(move || {
                let fetched = fetch_track(&discs, choice, &cache_dir, index);
                (fetched, Audio::booted_soundtrack_len(&discs))
            })
            .ok();
        Self {
            handle,
            soundtrack_len: None,
        }
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
            Ok((result, len)) => {
                self.soundtrack_len = Some(len);
                result
            }
            // A panic on the fetch thread is reported as a failed fetch rather
            // than resumed here, which would take the window down with it.
            Err(_) => Err(anyhow::anyhow!("the music fetch panicked")),
        })
    }

    /// The booted disc's soundtrack length this worker read, once joined.
    #[must_use]
    pub fn soundtrack_len(&self) -> Option<usize> {
        self.soundtrack_len
    }
}

/// The locate-and-decode half of [`Audio::fetch`]/[`Audio::fetch_indexed`],
/// without either one's "already read" cache check - a [`MusicFetchWorker`]
/// is spawned before its caller knows the answer is not already cached (a
/// race launch has no [`Audio::race_cache`] yet to check; a `MUSIC SOURCE`
/// switch checks [`Audio::held`]/[`Audio::race_cache`] itself before ever
/// spawning one, and applies the cached `Sound` on the spot instead - see
/// [`Audio::set_music_source`]). Free-standing so it can run on a thread
/// that does not own `Audio`'s mixer - see [`MusicFetchWorker::spawn`].
fn fetch_track(
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
    let sound = Arc::new(load_track(
        discs.library,
        source,
        platform,
        track,
        cache_dir,
    )?);
    Ok(Some(Loaded {
        from: Some(platform),
        sound,
        what: format!(
            "{platform} soundtrack track {index}, {:.1} s as its own disc lists it",
            track.seconds
        ),
    }))
}

/// What [`Audio::source_switch`]'s worker is fetching, and what to check
/// before applying it once it lands - a switch can take several ticks, and
/// whatever it was answering may not be the live question any more by then.
#[derive(Debug)]
pub(super) struct PendingSwitch {
    /// The release this fetch is moving onto - checked against a fresh
    /// request before this lands, so nudging the row back before the fetch
    /// finishes drops it rather than applies it after the fact. `pub(super)`
    /// so `Audio`'s own `Debug` impl in `audio.rs`, the parent module, can
    /// report it.
    pub(super) wanted: Platform,
    /// Which voice this is for, and enough about it to tell whether that is
    /// still true once the fetch lands.
    target: SwitchTarget,
    worker: MusicFetchWorker,
}

/// Which voice a [`PendingSwitch`] targets.
#[derive(Debug)]
enum SwitchTarget {
    /// The menu voice - live only while [`Audio::race_voice`] is `None`. A
    /// race starting while this is in flight empties [`Audio::music`], which
    /// [`Audio::tick`]'s apply checks for rather than assumes away.
    Menu,
    /// The race voice, fetching soundtrack index `index` of the booted
    /// disc's own order - live only while [`Audio::race_index`] still names
    /// it. A track boundary while this is in flight moves the index on, and
    /// applying a result for the track that just ended would be exactly the
    /// wrong recording - the same staleness [`Audio::advance_race_track`]
    /// already checks for [`Audio::race_prefetch`]. `discs`/`choice` are
    /// carried alongside so a landed switch can update
    /// [`Audio::race_context`] the same way the old synchronous version did,
    /// without asking the caller that spawned this a second time.
    Race {
        index: usize,
        discs: MusicDiscs,
        choice: MusicSource,
    },
}

/// The race-playlist half of `impl Audio`, kept beside [`MusicFetchWorker`]
/// rather than in `audio.rs` itself: every method here either produces or
/// consumes a [`MusicFetchWorker`]'s result, so the two belong to the same
/// question - "how does the race's music get fetched" - even though nothing
/// stops an inherent `impl` block living in a different file from the type it
/// is on. The `MUSIC SOURCE` row's async switch (below, from
/// [`Audio::set_music_source`]) belongs here for the same reason: it is the
/// exact same "decode one indexed track off a [`MusicFetchWorker`]" question,
/// just asked mid-menu or mid-race instead of at a race's own hand-off.
impl Audio {
    /// The race-launch counterpart of [`Self::start_race_music`]: applies a
    /// track that a [`MusicFetchWorker`], spawned earlier, has already located
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
        mut worker: MusicFetchWorker,
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
        // Never actually waits: this is only called once the loading screen's
        // own `race_ready` has seen `worker.is_finished()` true - see
        // `LoadingStage::race_ready`.
        let fetched = worker.join().unwrap_or_else(|| {
            Err(anyhow::anyhow!(
                "the race music fetch thread would not start"
            ))
        });
        let len = worker.soundtrack_len().filter(|&len| len > 0);
        self.race_context = Some((discs.clone(), choice, cache_dir.to_path_buf(), len));
        self.apply_fetched_race_track(index, fetched, seek);
        if self.race_voice.is_some() {
            self.race_position = 0.0;
        }
    }

    /// Reserves which track index the next race's music means, without
    /// fetching it.
    ///
    /// The split that lets a caller spawn a [`MusicFetchWorker`] for that index
    /// before the mixer - or the disc - is touched at all:
    /// [`Self::start_race_music`] used to do this reservation and the fetch in
    /// the same breath, which was fine when both were synchronous and cheap
    /// enough to not notice; spawning a worker for the fetch half needs the
    /// index settled first, so `Session::launch_race` can hand it to
    /// [`MusicFetchWorker::spawn`] before the loading screen even goes up.
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
    /// [`MusicFetchWorker`] fetched instead of [`Self::fetch_indexed`]. Neither
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
                    Some(seek) => debug!(
                        "audio: race music {}, {seconds:.1} s, resuming from {seek:.1} s",
                        loaded.what
                    ),
                    None => debug!("audio: race music {}, {seconds:.1} s", loaded.what),
                }
            }
            Ok(None) => warn!("audio: this source carries no race music this can play"),
            Err(error) => warn!("audio: no race music ({error:#})"),
        }
    }

    /// The modulus the race playlist wraps at: [`Self::booted_soundtrack_len`],
    /// read once and kept.
    ///
    /// **Not read on the tick thread when it can be helped.** Reading it opens
    /// the disc image and parses its soundtrack table - 38 ms on the first race
    /// tick in a release build on a desktop, the one frame over budget left
    /// once the race scene's build moved off the frame thread (see
    /// `docs/architecture/race-load-transition.md`). The race load's own
    /// [`MusicFetchWorker`] reads it alongside the track it fetches, and
    /// [`Self::finish_race_music`] keeps that answer in
    /// [`Self::race_context`]; the synchronous
    /// read is only the fallback for a run that never had one.
    ///
    /// A `0` - no soundtrack, or a read that failed - is never kept, so a
    /// transient failure is retried the next time exactly as it was before
    /// this cache existed.
    fn race_soundtrack_len(&mut self) -> usize {
        let Some((discs, _, _, known)) = self.race_context.as_mut() else {
            return 0;
        };
        if let Some(len) = *known {
            return len;
        }
        let len = Self::booted_soundtrack_len(discs);
        *known = (len > 0).then_some(len);
        len
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
        let Some((discs, choice, cache_dir, _)) = self.race_context.clone() else {
            return;
        };
        let next = next_race_index(index, self.race_soundtrack_len());
        self.race_prefetch = Some((
            next,
            MusicFetchWorker::spawn(discs, choice, cache_dir, next, "race-music"),
        ));
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
    /// means [`MusicFetchWorker::join`] returns immediately here instead;
    /// falls back to the old synchronous fetch only when there is no
    /// prefetch to trust - none started in time, or one started for a track
    /// this is no longer the same question as (a source change since - the
    /// index check below is what catches that).
    pub(super) fn advance_race_track(&mut self) {
        let Some((discs, choice, cache_dir, _)) = self.race_context.clone() else {
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
        let next = next_race_index(index, self.race_soundtrack_len());
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

    /// [`Audio::set_music_source`]'s menu case.
    pub(super) fn set_menu_music_source(
        &mut self,
        discs: &MusicDiscs,
        choice: MusicSource,
        cache_dir: &Path,
    ) {
        let Some((_, wanted)) = discs.pick(choice) else {
            return;
        };
        let (Some(_), Some(from)) = (self.music, self.music_from) else {
            // Either nothing is playing, or what is playing is not one of the
            // sixteen. Neither is this row's business - see above.
            return;
        };

        // A pending fetch for anything other than `wanted` no longer
        // describes what this row is asking for - dropped up front rather
        // than left to land later and apply a platform nobody is asking for
        // any more. This runs even when `wanted` turns out to already be
        // playing below: a press that lands back on the current release
        // while a switch away from it is still in flight means "stay here",
        // not "let the switch finish anyway".
        let already_fetching_this = self.source_switch.as_ref().is_some_and(|pending| {
            pending.wanted == wanted && matches!(pending.target, SwitchTarget::Menu)
        });
        if !already_fetching_this {
            self.source_switch = None;
        }

        if from == wanted || already_fetching_this {
            return;
        }

        if let Some((_, sound)) = self.held.iter().find(|(held, _)| *held == wanted) {
            // Already read this session - no I/O, so there is nothing to
            // move off this thread. Applied on the spot, the same as before.
            let sound = Arc::clone(sound);
            self.apply_menu_source_result(
                wanted,
                Ok(Some(Loaded {
                    from: Some(wanted),
                    sound,
                    what: format!("{wanted} soundtrack track {MUSIC_TRACK}, already read"),
                })),
            );
            return;
        }

        self.source_switch = Some(PendingSwitch {
            wanted,
            target: SwitchTarget::Menu,
            worker: MusicFetchWorker::spawn(
                discs.clone(),
                choice,
                cache_dir.to_path_buf(),
                MUSIC_TRACK,
                "menu-source",
            ),
        });
    }

    /// [`Audio::set_music_source`]'s race case: the same seek-preserving
    /// swap, against [`Audio::race_voice`]/[`Audio::race_index`] instead of
    /// the menu's fields, and landing in the bounded [`Audio::race_cache`],
    /// not [`Audio::held`].
    pub(super) fn set_race_music_source(
        &mut self,
        discs: &MusicDiscs,
        choice: MusicSource,
        cache_dir: &Path,
    ) {
        let Some((_, wanted)) = discs.pick(choice) else {
            return;
        };
        let (Some(_), Some(from), Some(index)) = (self.race_voice, self.race_from, self.race_index)
        else {
            return;
        };

        // Same up-front drop as `Self::set_menu_music_source`: a pending
        // fetch for a different release, or a different track of this one,
        // no longer describes what this row is asking for.
        let already_fetching_this = self.source_switch.as_ref().is_some_and(|pending| {
            pending.wanted == wanted
                && matches!(pending.target, SwitchTarget::Race { index: i, .. } if i == index)
        });
        if !already_fetching_this {
            self.source_switch = None;
        }

        if from == wanted || already_fetching_this {
            return;
        }

        if let Some((cached_platform, cached_index, sound)) = &self.race_cache
            && *cached_platform == wanted
            && *cached_index == index
        {
            let sound = Arc::clone(sound);
            self.apply_race_source_result(
                wanted,
                index,
                discs.clone(),
                choice,
                Ok(Some(Loaded {
                    from: Some(wanted),
                    sound,
                    what: format!("{wanted} soundtrack track {index}, already read"),
                })),
            );
            return;
        }

        // Dropped rather than kept: it was fetching the release this row is
        // leaving, and a track fetched from the wrong release is exactly the
        // kind of wrong `Self::advance_race_track` cannot detect by index
        // alone. `Self::maybe_prefetch_next_race_track` starts a correct one
        // once the switch below actually lands.
        self.race_prefetch = None;

        self.source_switch = Some(PendingSwitch {
            wanted,
            target: SwitchTarget::Race {
                index,
                discs: discs.clone(),
                choice,
            },
            worker: MusicFetchWorker::spawn(
                discs.clone(),
                choice,
                cache_dir.to_path_buf(),
                index,
                "race-source",
            ),
        });
    }

    /// Applies a fetched menu-source switch, from
    /// [`Self::set_menu_music_source`]'s cache-hit fast path or a landed
    /// [`PendingSwitch`] worker polled by [`Self::poll_source_switch`].
    /// Either way the playhead is read **here**, at the moment of the swap,
    /// not when the fetch was first asked for - a worker can take several
    /// ticks to land, and the old voice keeps playing every one of them.
    fn apply_menu_source_result(&mut self, wanted: Platform, fetched: Result<Option<Loaded>>) {
        match fetched {
            Ok(Some(loaded)) => {
                // Banked for next time whether or not there is a menu voice
                // left to apply to below - the decode is not wasted just
                // because a race started while it was in flight.
                if let Some(platform) = loaded.from
                    && !self.held.iter().any(|(held, _)| *held == platform)
                {
                    self.held.push((platform, Arc::clone(&loaded.sound)));
                }
                let Some(playing) = self.music else {
                    return;
                };
                let at = self.playhead().unwrap_or(0.0);
                self.menu_sound = Some(Arc::clone(&loaded.sound));
                self.output.with_mixer(|mixer| {
                    mixer.stop(playing);
                    let started = mixer.play(Play::looping(Arc::clone(&loaded.sound), Bus::Music));
                    if let Some(id) = started {
                        mixer.seek(id, at);
                    }
                    self.music = started;
                });
                self.music_from = self.music.and(loaded.from);
                debug!("audio: music {}, from {at:.1} s", loaded.what);
            }
            Ok(None) => {
                warn!("audio: no soundtrack on the {wanted} release, so nothing changed")
            }
            Err(error) => warn!("audio: the music stays where it is ({error:#})"),
        }
    }

    /// [`Self::apply_menu_source_result`]'s race counterpart.
    ///
    /// `index` is checked against [`Audio::race_index`] before anything
    /// else: the playlist can move on to a new track - naturally, or a
    /// second switch - while this was in flight, and applying a result for
    /// the track that just ended would put the wrong recording under the
    /// current one. `discs`/`choice` land in [`Audio::race_context`] on
    /// success, the same bookkeeping the old synchronous version did, so a
    /// later advance-on-finish in [`Audio::tick`] fetches the next track
    /// from the release this switch actually moved to.
    fn apply_race_source_result(
        &mut self,
        wanted: Platform,
        index: usize,
        discs: MusicDiscs,
        choice: MusicSource,
        fetched: Result<Option<Loaded>>,
    ) {
        match fetched {
            Ok(Some(loaded)) => {
                if self.race_index != Some(index) {
                    return; // answers a question nobody is asking any more
                }
                let Some(playing) = self.race_voice else {
                    return; // the race ended while this was in flight
                };
                let at = self.playhead_race().unwrap_or(0.0);
                self.output.with_mixer(|mixer| {
                    mixer.stop(playing);
                    let started = mixer.play(Play::once(Arc::clone(&loaded.sound), Bus::Music));
                    if let Some(id) = started {
                        mixer.seek(id, at);
                    }
                    self.race_voice = started;
                });
                self.race_from = self.race_voice.and(loaded.from);
                self.race_cache = loaded.from.map(|platform| (platform, index, loaded.sound));
                if let Some((cached_discs, cached_choice, ..)) = &mut self.race_context {
                    *cached_discs = discs;
                    *cached_choice = choice;
                }
                // A prefetch still in flight was started for the release
                // this switch just left - see `Self::set_race_music_source`,
                // which already dropped the one that existed when this was
                // spawned; this catches one `Self::maybe_prefetch_next_race_track`
                // started for the old release in the meantime.
                self.race_prefetch = None;
                debug!("audio: race music {}, from {at:.1} s", loaded.what);
            }
            Ok(None) => {
                warn!("audio: no soundtrack on the {wanted} release, so nothing changed")
            }
            Err(error) => warn!("audio: the race music stays where it is ({error:#})"),
        }
    }

    /// Polls [`Audio::source_switch`], applying it once its worker lands.
    ///
    /// Called from [`Audio::tick`], the same way [`Self::advance_race_track`]
    /// and [`Self::maybe_prefetch_next_race_track`] are - this module's rule
    /// that everything here moves on the tick count, never a frame or a wall
    /// clock. A no-op on every tick but the one a fetch actually finishes on.
    pub(super) fn poll_source_switch(&mut self) {
        let Some(pending) = &self.source_switch else {
            return;
        };
        if !pending.worker.is_finished() {
            return;
        }
        let Some(PendingSwitch {
            wanted,
            target,
            mut worker,
        }) = self.source_switch.take()
        else {
            return;
        };
        let fetched = worker.join().unwrap_or_else(|| {
            Err(anyhow::anyhow!(
                "the music source fetch thread would not start"
            ))
        });
        match target {
            SwitchTarget::Menu => self.apply_menu_source_result(wanted, fetched),
            SwitchTarget::Race {
                index,
                discs,
                choice,
            } => self.apply_race_source_result(wanted, index, discs, choice, fetched),
        }
    }
}
