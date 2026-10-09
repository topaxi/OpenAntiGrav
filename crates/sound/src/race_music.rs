//! [`MusicFetchWorker`]: locating and decoding one soundtrack track on its own
//! thread so no caller waits on it: the race hand-off, the race-boundary
//! prefetch and the `MUSIC SOURCE` row. Split out of `lib.rs` under the
//! 1,000-line rule.

use super::*;

/// Which track of `platform`'s soundtrack `index` means, `index` being an entry
/// in the **booted** disc's own order.
///
/// On the booted release that is its own entry `index`; on the other it is the
/// track of the same length, so both selections are the same recording.
///
/// The index is not stable across boots: the two archives are in different
/// orders. Entry 0 names the same recording on both (coincidence), at index 1
/// the two boots start on different music. `MUSIC_TRACK` is a constant so never
/// notices; `Audio::race_index` is read in the booted disc's order for exactly
/// this reason, and a future circuit-to-track map must be built the same way.
///
/// Free, not an `Audio` method, so [`fetch_track`] can call it from a thread
/// that does not own `Audio`. `pub(super)` for `Audio::fetch`/`fetch_indexed`.
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

/// One soundtrack track being located and decoded on its own thread, the shape
/// of `oag_raceplay::worker::LoadWorker` and `oag_game::boot::MediaWorker`: a
/// synchronous fetch blocked the caller for a cold decode (2.83 s for a full
/// PS2 track), the gap between the loading screen's fade and the first race
/// frame. Also reused by [`Audio::maybe_prefetch_next_race_track`] and
/// [`Audio::set_music_source`].
///
/// Holds no [`Audio`]: [`fetch_track`] produces a plain [`Loaded`], and the
/// mixer is only entered when a caller applies it on its own thread
/// (`Audio::finish_race_music`, `Audio::advance_race_track`, `Audio::tick`'s
/// poll), as `oag_raceplay::worker` argues for `LoadWorker`.
#[derive(Debug)]
pub struct MusicFetchWorker {
    /// The fetch, on a thread natively and on a Web Worker in the browser,
    /// where the page's thread polls it and never waits (`oag_thread::Task`).
    task: oag_thread::Task<(Result<Option<Loaded>>, usize)>,
    /// The booted disc's soundtrack length, read on the fetch's thread and
    /// available after [`Self::join`] ([`Audio::race_soundtrack_len`]).
    soundtrack_len: Option<usize>,
}

impl MusicFetchWorker {
    /// Starts the fetch in the background.
    ///
    /// `index` addresses the booted disc's soundtrack order, as for every
    /// [`fetch_track`] caller.
    ///
    /// `label` names the thread (`"race-music"`, `"race-source"`,
    /// `"menu-source"`) so a debugger says which caller waits; under 15 bytes,
    /// since Linux truncates a pthread name past that.
    #[must_use]
    pub fn spawn(
        discs: MusicDiscs,
        choice: MusicSource,
        cache_dir: PathBuf,
        index: usize,
        label: &str,
    ) -> Self {
        let task = oag_thread::Task::spawn(label, move || {
            let fetched = fetch_track(&discs, choice, &cache_dir, index);
            (fetched, Audio::booted_soundtrack_len(&discs))
        });
        Self {
            task,
            soundtrack_len: None,
        }
    }

    /// Whether the fetch has returned; `true` if the thread failed to spawn (as
    /// `oag_raceplay::worker::LoadWorker::is_finished`), so a poller never waits on a
    /// thread that never started and the error surfaces from [`Self::join`].
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }

    /// Takes the result, waiting if it is not in yet (the frame loop only calls
    /// it once [`Self::is_finished`]). `None` on a second call or a thread that
    /// would not spawn.
    pub fn join(&mut self) -> Option<Result<Option<Loaded>>> {
        Some(match self.task.join()? {
            Ok((result, len)) => {
                self.soundtrack_len = Some(len);
                result
            }
            // A fetch-thread panic is a failed fetch, not resumed (it would take
            // the window down).
            Err(_) => Err(anyhow::anyhow!("the music fetch panicked")),
        })
    }

    /// The booted disc's soundtrack length this worker read, once joined.
    #[must_use]
    pub fn soundtrack_len(&self) -> Option<usize> {
        self.soundtrack_len
    }
}

/// The locate-and-decode half of [`Audio::fetch`]/[`Audio::fetch_indexed`]
/// without their "already read" cache check: a worker is spawned before the
/// caller knows the answer is uncached (a `MUSIC SOURCE` switch checks
/// [`Audio::held`]/[`Audio::race_cache`] first, see [`Audio::set_music_source`]).
/// Free so it runs on a thread without `Audio`'s mixer.
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

/// What [`Audio::source_switch`]'s worker is fetching, and what to check before
/// applying it: a switch spans several ticks and may be stale when it lands.
#[derive(Debug)]
pub(super) struct PendingSwitch {
    /// The release this fetch moves onto, checked against a fresh request so
    /// nudging the row back drops it. `pub(super)` for `Audio`'s `Debug` impl.
    pub(super) wanted: Platform,
    /// Which voice this is for, and enough to tell whether that still holds.
    target: SwitchTarget,
    worker: MusicFetchWorker,
}

/// Which voice a [`PendingSwitch`] targets.
#[derive(Debug)]
enum SwitchTarget {
    /// The menu voice, live only while [`Audio::race_voice`] is `None`; a race
    /// starting meanwhile empties [`Audio::music`], which [`Audio::tick`] checks.
    Menu,
    /// The race voice, fetching soundtrack index `index` of the booted order,
    /// live only while [`Audio::race_index`] still names it: applying a result
    /// for a track that just ended would be the wrong recording (the staleness
    /// [`Audio::advance_race_track`] checks for [`Audio::race_prefetch`]).
    /// `discs`/`choice` let a landed switch update [`Audio::race_context`].
    Race {
        index: usize,
        discs: MusicDiscs,
        choice: MusicSource,
    },
}

/// The race-playlist half of `impl Audio`, beside [`MusicFetchWorker`]: every
/// method here produces or consumes a worker's result, including the
/// `MUSIC SOURCE` row's async switch ([`Audio::set_music_source`]).
impl Audio {
    /// The race-launch counterpart of [`Self::start_race_music`]: applies a track
    /// a [`MusicFetchWorker`] spawned earlier has located and decoded, so it
    /// never reads the disc or blocks. Fixes the gap between the loading
    /// screen's fade and the first race frame: a synchronous cold PS2 decode
    /// measured 2.83 s on the frame thread.
    ///
    /// `index` was reserved by [`Self::reserve_race_music_index`] before the
    /// worker spawned (`Session::launch_race`), so [`Self::race_index`] is read
    /// back instead of taken as a parameter.
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
        // Defensive, as in `start_race_music`: nothing calls this with a race
        // voice sounding, but a wrong assumption would leak a voice.
        if let Some(id) = self.race_voice.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }
        let index = self.race_index.expect(
            "Session::launch_race reserves this via reserve_race_music_index before spawning \
             the worker this method takes",
        );
        let seek = (self.race_position > 0.0).then_some(self.race_position);
        // Never waits: only called once the loading screen's `race_ready` saw
        // `worker.is_finished()` (`LoadingStage::race_ready`).
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

    /// Reserves which track index the next race's music means, without fetching.
    ///
    /// Lets `Session::launch_race` spawn a [`MusicFetchWorker`] for that index
    /// before the loading screen goes up; [`Self::start_race_music`] used to
    /// reserve and fetch together.
    pub fn reserve_race_music_index(&mut self, discs: &MusicDiscs) -> usize {
        if self.race_index.is_none() {
            self.race_index = Some(Self::initial_race_index(&self.music_from, discs));
        }
        self.race_index.expect("set immediately above")
    }

    /// Loads and plays one race-playlist track, reporting what happened.
    ///
    /// Shared by [`Self::start_race_music`] and the advance-on-finish check in
    /// [`Self::tick`]. `seek` is `Some` for a resume, `None` for a fresh start.
    /// `pub(super)`: `Audio::start_race_music` is the general entry point and
    /// stayed in `lib.rs`.
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
    /// shared with [`Self::finish_race_music`]. Touches only the mixer and the
    /// race-playlist state.
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
            Err(error) => crate::warn_no_music("no race music", &error),
        }
    }

    /// The modulus the race playlist wraps at: [`Self::booted_soundtrack_len`],
    /// read once and kept.
    ///
    /// Not read on the tick thread when it can be helped: it opens the disc image
    /// and parses the soundtrack table, 38 ms on the first race tick (release,
    /// desktop), the one frame over budget once the scene build left the frame
    /// thread (`docs/architecture/race-load-transition.md`). The race load's
    /// [`MusicFetchWorker`] reads it, [`Self::finish_race_music`] keeps it in
    /// [`Self::race_context`], and the synchronous read is the fallback. A `0`
    /// is never kept, so a transient failure is retried.
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

    /// Starts fetching the race playlist's next track as soon as the current one
    /// is playing (see [`Audio::race_prefetch`]).
    ///
    /// Immediately, not "a few seconds before the end": a remaining-time
    /// threshold covers the natural end but not a skip (a `MUSIC SOURCE`-style
    /// row or future skip control can end a track anywhere). The cost is one
    /// extra track's PCM (tens of MiB) alongside [`Self::race_cache`]'s slot.
    ///
    /// A no-op except on the tick after a track starts: outside a race, or once
    /// [`Self::race_prefetch`] is `Some`.
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

    /// Moves the race playlist on once the current track has finished: tracks
    /// play [`Play::once`], so "no longer playing" means "reached its end".
    ///
    /// Joins [`Self::race_prefetch`] rather than fetching from nothing, which
    /// paid a disc read and possibly an `ffmpeg` decode (0.4-2.8 s measured)
    /// inside the fixed-timestep loop at every boundary. Falls back to the
    /// synchronous fetch only with no trustworthy prefetch (none in time, or for
    /// a track since changed by a source change; the index check catches that).
    pub(super) fn advance_race_track(&mut self) {
        let Some((discs, choice, cache_dir, _)) = self.race_context.clone() else {
            return;
        };
        let Some(index) = self.race_index else {
            return;
        };
        // Stopped explicitly: `Audio::tick` only gets here after the voice ended,
        // but a skip control would not, and `mixer.stop` on a finished id is a
        // no-op (ids carry a generation).
        if let Some(id) = self.race_voice.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }
        let next = next_race_index(index, self.race_soundtrack_len());
        self.race_index = Some(next);

        // In the browser `join` cannot wait for an unfinished fetch, so one
        // the boundary beat is dropped and the track fetched the synchronous
        // way below, as it would be with no prefetch at all.
        if let Some((prefetch_index, mut worker)) = self.race_prefetch.take()
            && prefetch_index == next
            && (cfg!(not(target_arch = "wasm32")) || worker.is_finished())
        {
            // Rarely waits (see the doc), but can if the boundary beat the fetch;
            // no worse than the synchronous path, which this worker is mostly
            // through.
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
            // Nothing playing, or not one of the sixteen: not this row's business.
            return;
        };

        // A pending fetch for anything but `wanted` is dropped up front rather
        // than landing later. Runs even when `wanted` is already playing: a press
        // back onto the current release during a switch away means "stay here".
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
            // Already read this session: no I/O, applied on the spot.
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

    /// [`Audio::set_music_source`]'s race case: the same seek-preserving swap
    /// against [`Audio::race_voice`]/[`Audio::race_index`], landing in the
    /// bounded [`Audio::race_cache`], not [`Audio::held`].
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

        // Same up-front drop as `Self::set_menu_music_source`.
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

        // Dropped: it was fetching the release this row is leaving, and a track
        // from the wrong release is wrong in a way `Self::advance_race_track`
        // cannot detect by index. `Self::maybe_prefetch_next_race_track` starts
        // a correct one once the switch lands.
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

    /// Applies a fetched menu-source switch, from the cache-hit fast path of
    /// [`Self::set_menu_music_source`] or a landed [`PendingSwitch`] polled by
    /// [`Self::poll_source_switch`]. The playhead is read here, at the swap, not
    /// when the fetch was asked for: the old voice plays on for every tick the
    /// worker takes.
    fn apply_menu_source_result(&mut self, wanted: Platform, fetched: Result<Option<Loaded>>) {
        match fetched {
            Ok(Some(loaded)) => {
                // Banked whether or not a menu voice is left to apply to: a race
                // starting mid-fetch does not waste the decode.
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
            Err(error) => crate::warn_no_music("the music stays where it is", &error),
        }
    }

    /// [`Self::apply_menu_source_result`]'s race counterpart.
    ///
    /// `index` is checked against [`Audio::race_index`] first: the playlist can
    /// move on mid-flight, and a result for the track that just ended would put
    /// the wrong recording under the current one. `discs`/`choice` land in
    /// [`Audio::race_context`] on success so the next advance fetches from the
    /// release this switch moved to.
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
                // A prefetch started for the release this switch just left (after
                // `Self::set_race_music_source` dropped the earlier one).
                self.race_prefetch = None;
                debug!("audio: race music {}, from {at:.1} s", loaded.what);
            }
            Ok(None) => {
                warn!("audio: no soundtrack on the {wanted} release, so nothing changed")
            }
            Err(error) => crate::warn_no_music("the race music stays where it is", &error),
        }
    }

    /// Polls [`Audio::source_switch`], applying it once its worker lands. Called
    /// from [`Audio::tick`] like [`Self::advance_race_track`]: everything here
    /// moves on the tick count. A no-op except on the tick a fetch finishes.
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
