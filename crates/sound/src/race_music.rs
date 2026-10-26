//! [`MusicFetchWorker`]: locating and decoding one soundtrack track on its own
//! thread so no caller waits on it: every race start and resume, the
//! race-boundary prefetch and the `MUSIC SOURCE` row.

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
/// that does not own `Audio`.
fn locate(
    discs: &MusicDiscs,
    platform: Platform,
    source: &str,
    index: usize,
) -> Result<Option<Track>> {
    let Some(soundtrack) = discs.listings.read(discs.library, source, platform)? else {
        return Ok(None);
    };
    if discs.booted() == Some(platform) {
        return Ok(soundtrack.tracks.get(index).copied());
    }

    let Some((booted_source, booted_platform)) = discs.pick(MusicSource::Auto) else {
        return Ok(soundtrack.tracks.get(index).copied());
    };
    let Some(booted) = discs
        .listings
        .read(discs.library, booted_source, booted_platform)?
    else {
        return Ok(soundtrack.tracks.get(index).copied());
    };
    let Some(wanted) = booted.tracks.get(index) else {
        return Ok(None);
    };
    Ok(soundtrack.nearest(wanted.seconds))
}

/// Which track a race fetch is for, as far as the frame thread can say without
/// a listing.
///
/// "One past" and "the saved place, if it still matches" both need the booted
/// soundtrack's listing, which on HD means reading every track's header off a
/// PSARC. So the frame thread hands over the rule and the worker applies it
/// ([`RacePick::resolve`]); [`MusicFetchWorker::index`] says what it chose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RacePick {
    index: Pick,
    /// Where to start the track, for a resume of a known index.
    seek: Option<f64>,
    /// A saved place still to be checked against the listing
    /// ([`Audio::restore_race_music`] when the listing was not yet read).
    saved: Option<RaceMusicState>,
}

/// The index half of a [`RacePick`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pick {
    /// This track of the booted order.
    Exact(usize),
    /// The track after this one, wrapping at the listing's length.
    After(usize),
    /// A run's first race: one past the menu's [`MUSIC_TRACK`] when the menu
    /// plays a soundtrack track (a PS2 boot), so the race does not restart the
    /// recording the menu was playing; else [`MUSIC_TRACK`]. Decided by the
    /// booted platform, not by whether the menu's loop has landed yet.
    FirstRace,
}

impl RacePick {
    /// Track `index` of the booted order, from its start.
    #[must_use]
    pub fn exact(index: usize) -> Self {
        Self {
            index: Pick::Exact(index),
            seek: None,
            saved: None,
        }
    }

    /// The track after `index`, from its start.
    pub(super) fn after(index: usize) -> Self {
        Self {
            index: Pick::After(index),
            seek: None,
            saved: None,
        }
    }

    /// The index this names when it needs no listing, which is what lets an
    /// already decoded track start on the spot. Never reads or waits.
    pub(super) fn known(&self, discs: &MusicDiscs) -> Option<usize> {
        match (self.index, self.saved) {
            (Pick::Exact(index), None) => Some(index),
            (Pick::FirstRace, None) => match discs.booted()? {
                Platform::Ps2 => None,
                _ => Some(MUSIC_TRACK),
            },
            _ => None,
        }
    }

    /// [`Self::resolve`]'s index, for a unit test.
    #[cfg(test)]
    pub(super) fn resolve_for_test(self, discs: &MusicDiscs, len: usize) -> usize {
        self.resolve(discs, len).index
    }

    /// The track, the seek and the restore verdict, on the worker's thread.
    fn resolve(self, discs: &MusicDiscs, len: usize) -> Resolved {
        let normal = match self.index {
            Pick::Exact(index) => index,
            Pick::FirstRace if discs.booted() != Some(Platform::Ps2) => MUSIC_TRACK,
            Pick::After(_) | Pick::FirstRace => {
                let index = match self.index {
                    Pick::After(index) => index,
                    _ => MUSIC_TRACK,
                };
                if len == 0 {
                    warn!(
                        "audio: could not list the booted disc's soundtrack, so the race \
                         playlist stays on track {index} rather than the one after it"
                    );
                }
                next_race_index(index, len)
            }
        };
        let Some(saved) = self.saved else {
            return Resolved {
                index: normal,
                seek: self.seek,
                restore: None,
            };
        };
        match saved.check(&Audio::booted_track_seconds(discs)) {
            Ok(()) => Resolved {
                index: saved.index,
                seek: (saved.second > 0).then_some(f64::from(saved.second)),
                restore: Some(Ok(saved)),
            },
            Err(why) => Resolved {
                index: normal,
                seek: None,
                restore: Some(Err(why)),
            },
        }
    }
}

/// What a [`RacePick`] came to.
#[derive(Debug, Clone, PartialEq)]
struct Resolved {
    index: usize,
    seek: Option<f64>,
    /// The verdict on a saved place the frame thread could not check.
    restore: Option<std::result::Result<RaceMusicState, Unusable>>,
}

/// Everything a fetch hands back.
struct Landed {
    fetched: Result<Option<Loaded>>,
    resolved: Resolved,
    /// The booted soundtrack's length, `0` when it could not be listed.
    len: usize,
}

/// One soundtrack track being located and decoded on its own thread, the shape
/// of `oag_raceplay::worker::LoadWorker` and `oag_game::boot::MediaWorker`.
///
/// **Every race track goes through one**, so the frame thread never reads a
/// listing or decodes a track: the race hand-off (`Session::launch_race`),
/// [`Audio::start_race_music`] when the track is not already decoded, the
/// race-boundary prefetch ([`Audio::maybe_prefetch_next_race_track`]) and the
/// `MUSIC SOURCE` row ([`Audio::set_music_source`]). The frame thread only
/// polls ([`Audio::tick`]); a fetch that lands late starts the music late.
///
/// Holds no [`Audio`]: the fetch produces a plain [`Loaded`], and the mixer is
/// only entered when [`Audio`] applies it on its own thread.
pub struct MusicFetchWorker {
    /// The fetch, on a thread natively and on a Web Worker in the browser,
    /// where the page's thread polls it and never waits (`oag_thread::Task`).
    task: oag_thread::Task<Landed>,
    /// What the fetch was asked for, so a stale prefetch can be told apart.
    pick: RacePick,
}

impl std::fmt::Debug for MusicFetchWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MusicFetchWorker")
            .field("pick", &self.pick)
            .field("finished", &self.task.is_finished())
            .finish()
    }
}

impl MusicFetchWorker {
    /// Starts the fetch in the background.
    ///
    /// `label` names the thread (`"race-music"`, `"race-source"`,
    /// `"menu-source"`) so a debugger says which caller waits; under 15 bytes,
    /// since Linux truncates a pthread name past that.
    #[must_use]
    pub fn spawn(
        discs: MusicDiscs,
        choice: MusicSource,
        cache_dir: PathBuf,
        pick: RacePick,
        label: &str,
    ) -> Self {
        let task = oag_thread::Task::spawn(label, move || {
            discs.wait_surveyed();
            let len = Audio::booted_soundtrack_len(&discs);
            let resolved = pick.resolve(&discs, len);
            let fetched = fetch_track(&discs, choice, &cache_dir, resolved.index);
            Landed {
                fetched,
                resolved,
                len,
            }
        });
        Self { task, pick }
    }

    /// Whether the fetch has returned; `true` if the thread failed to spawn (as
    /// `oag_raceplay::worker::LoadWorker::is_finished`), so a poller never waits on a
    /// thread that never started and the error surfaces when it is applied.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }

    /// Takes the result. Natively this waits for an unfinished fetch, which
    /// only [`Audio::settle_music`] and the source switch's poll (after
    /// [`Self::is_finished`]) ever ask for.
    fn land(&mut self) -> Landed {
        let failed = |why: &str| Landed {
            fetched: Err(anyhow::anyhow!("{why}")),
            resolved: self.pick.resolve_without_listing(),
            len: 0,
        };
        match self.task.join() {
            Some(Ok(landed)) => landed,
            // A fetch-thread panic is a failed fetch, not resumed (it would take
            // the window down).
            Some(Err(_)) => failed("the music fetch panicked"),
            None => failed("the music fetch thread would not start"),
        }
    }
}

impl RacePick {
    /// What a pick means when the fetch never ran: its known index, or the
    /// track it counts from.
    fn resolve_without_listing(&self) -> Resolved {
        let index = match self.index {
            Pick::Exact(index) | Pick::After(index) => index,
            Pick::FirstRace => MUSIC_TRACK,
        };
        Resolved {
            index,
            seek: self.seek,
            restore: None,
        }
    }
}

/// Locates and decodes one soundtrack track, on a worker's thread. No "already
/// read" check: callers look in [`Audio::held`]/[`Audio::race_cache`] before
/// they spawn one. Free so it runs on a thread without `Audio`'s mixer.
pub(super) fn fetch_track(
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
    /// The race-launch counterpart of [`Self::start_race_music`]: takes the
    /// [`MusicFetchWorker`] `Session::launch_race` spawned with
    /// [`Self::race_music_pick`], and starts its track the moment it lands.
    ///
    /// Never waits: a finished fetch (the loading screen's `race_ready` waits
    /// for one) applies here, an unfinished one on a later [`Self::tick`]. A
    /// synchronous cold PS2 decode here once measured 2.83 s on the frame thread.
    pub fn finish_race_music(
        &mut self,
        worker: MusicFetchWorker,
        discs: &MusicDiscs,
        choice: MusicSource,
        cache_dir: &Path,
    ) {
        self.stop_music_voices();
        self.race_context = Some((discs.clone(), choice, cache_dir.to_path_buf()));
        self.race_job = Some(worker);
        self.poll_race_job();
        self.settle_if_dumping();
    }

    /// Which track the next race's music means, without reading anything.
    ///
    /// The track the playlist is on once one has played (with the place
    /// [`Self::pause_race_music`] saved). Before that, one past the menu's
    /// track on a PS2 boot, whose menu plays one of the sixteen, so the first
    /// race does not restart the recording the menu was playing, else
    /// [`MUSIC_TRACK`]; plus a saved place [`Self::restore_race_music`] could
    /// not check yet, which the worker checks first. A saved place is handed
    /// out once.
    pub fn race_music_pick(&mut self) -> RacePick {
        if let Some(index) = self.race_index {
            return RacePick {
                seek: (self.race_position > 0.0).then_some(self.race_position),
                ..RacePick::exact(index)
            };
        }
        RacePick {
            index: Pick::FirstRace,
            seek: None,
            saved: self.pending_restore.take(),
        }
    }

    /// Stops the menu voice and any race voice, before a race track starts.
    pub(super) fn stop_music_voices(&mut self) {
        self.song_hold.drop_parked();
        if let Some(id) = self.music.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }
        // Defensive: nothing starts a race track with a race voice sounding,
        // but a wrong assumption would leak a voice.
        if let Some(id) = self.race_voice.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }
    }

    /// The race track `pick` names, if it is already decoded: what makes a
    /// resume from the menus instant.
    pub(super) fn cached_race_track(
        &self,
        discs: &MusicDiscs,
        choice: MusicSource,
        pick: &RacePick,
    ) -> Option<(usize, Loaded)> {
        let index = pick.known(discs)?;
        let (_, platform) = discs.pick(choice)?;
        let (cached_platform, cached_index, sound) = self.race_cache.as_ref()?;
        (*cached_platform == platform && *cached_index == index).then(|| {
            (
                index,
                Loaded {
                    from: Some(platform),
                    sound: Arc::clone(sound),
                    what: format!("{platform} soundtrack track {index}, already read"),
                },
            )
        })
    }

    /// Applies [`Self::race_job`] once its fetch has landed. Called from
    /// [`Self::tick`]; a no-op while it is in flight.
    pub(super) fn poll_race_job(&mut self) {
        if !self
            .race_job
            .as_ref()
            .is_some_and(MusicFetchWorker::is_finished)
        {
            return;
        }
        if let Some(mut job) = self.race_job.take() {
            let landed = job.land();
            self.apply_race_landed(landed);
        }
    }

    /// Starts the race voice on a landed fetch.
    ///
    /// A race left while the fetch was in flight banks the track and its place
    /// instead, so the next [`Self::start_race_music`] resumes it at once.
    fn apply_race_landed(&mut self, landed: Landed) {
        let Landed {
            fetched,
            resolved,
            len,
        } = landed;
        if len > 0 {
            self.race_tracks = len;
        }
        match resolved.restore {
            Some(Ok(saved)) => info!(
                "audio: race music resumes at track {} and {} s, as saved",
                saved.index, saved.second
            ),
            Some(Err(why)) => warn!(
                "audio: not resuming the saved race music ({why}); the playlist starts at its \
                 normal place"
            ),
            None => {}
        }
        let index = resolved.index;
        self.race_index = Some(index);
        if self.race_context.is_none() {
            if let Ok(Some(loaded)) = fetched
                && let Some(platform) = loaded.from
            {
                self.race_cache = Some((platform, index, loaded.sound));
            }
            self.race_position = resolved.seek.unwrap_or(0.0);
            return;
        }
        self.apply_fetched_race_track(index, fetched, resolved.seek);
        // Consumed only once applied to a sounding voice, so a failed attempt
        // keeps the saved position for a later call.
        if self.race_voice.is_some() {
            self.race_position = 0.0;
        }
    }

    /// Waits for every music start in flight and applies it, natively; a
    /// no-op in the browser, whose page thread may not wait.
    ///
    /// For a caller that is not a frame thread and needs the music to start on
    /// a known tick: a `--dump-audio` capture is compared sample for sample at
    /// a tick count, so every start calls this while [`Self::dump`] is set
    /// ([`Self::settle_if_dumping`]). Waiting does no music work on this
    /// thread, it only blocks it. A `MUSIC SOURCE` switch is not waited for:
    /// it lands on [`Self::tick`] as before.
    pub fn settle_music(&mut self) {
        if cfg!(target_arch = "wasm32") {
            return;
        }
        if let Some(mut job) = self.menu_job.take() {
            self.apply_menu_landed(job.land());
        }
        if let Some(mut job) = self.race_job.take() {
            let landed = job.land();
            self.apply_race_landed(landed);
        }
    }

    /// Whether a music start or a `MUSIC SOURCE` switch is still being read.
    /// A prefetch of the next race track is not counted: nothing waits on it.
    #[must_use]
    pub fn music_in_flight(&self) -> bool {
        self.menu_job.is_some() || self.race_job.is_some() || self.source_switch.is_some()
    }

    /// [`Self::settle_music`] when a dump is being written.
    pub(super) fn settle_if_dumping(&mut self) {
        if self.dump.is_some() {
            self.settle_music();
        }
    }

    /// The "start a voice on what was fetched" half of every race start.
    /// Touches only the mixer and the race-playlist state.
    ///
    /// A track that lands while [`Audio::hold_race_song`] holds the song is banked and parked
    /// instead, and starts when the race leaves its countdown ([`Audio::release_race_song`]).
    pub(super) fn apply_fetched_race_track(
        &mut self,
        index: usize,
        fetched: Result<Option<Loaded>>,
        seek: Option<f64>,
    ) {
        match fetched {
            Ok(Some(loaded)) if self.song_hold.holding() => {
                self.race_cache = loaded
                    .from
                    .map(|platform| (platform, index, Arc::clone(&loaded.sound)));
                debug!("audio: race music {} waits for the green light", loaded.what);
                self.song_hold.park(index, loaded, seek);
            }
            Ok(Some(loaded)) => self.start_race_voice(index, loaded, seek),
            Ok(None) => warn!("audio: this source carries no race music this can play"),
            Err(error) => warn_no_music("no race music", &error),
        }
    }

    /// Starts the race voice on a decoded track: `seek` resumes it part way in, which is not a
    /// new song for the now-playing display.
    pub(super) fn start_race_voice(&mut self, index: usize, loaded: Loaded, seek: Option<f64>) {
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
        if self.race_voice.is_some() && seek.is_none() {
            self.song_serial = self.song_serial.wrapping_add(1);
        }
        match seek {
            Some(seek) => debug!(
                "audio: race music {}, {seconds:.1} s, resuming from {seek:.1} s",
                loaded.what
            ),
            None => debug!("audio: race music {}, {seconds:.1} s", loaded.what),
        }
    }

    /// Starts fetching the race playlist's next track as soon as the current one
    /// is playing (see [`Audio::race_prefetch`]).
    ///
    /// Immediately, not "a few seconds before the end": a remaining-time
    /// threshold covers the natural end but not a skip (a `MUSIC SOURCE`-style
    /// row or future skip control can end a track anywhere). The cost is one
    /// extra track's PCM (tens of MiB) alongside [`Self::race_cache`]'s slot.
    /// The worker works out which track "next" is, so this reads nothing.
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
        let Some((discs, choice, cache_dir)) = self.race_context.clone() else {
            return;
        };
        self.race_prefetch = Some(MusicFetchWorker::spawn(
            discs,
            choice,
            cache_dir,
            RacePick::after(index),
            "race-music",
        ));
    }

    /// Moves the race playlist on once the current track has finished: tracks
    /// play [`Play::once`], so "no longer playing" means "reached its end".
    ///
    /// Hands [`Self::race_prefetch`] over as [`Self::race_job`], which starts
    /// the next track the tick it lands: at once when the prefetch is in, a
    /// little late when the boundary beat it, and never by waiting here (this
    /// used to decode the next track, 0.4 to 2.8 s, inside the fixed-timestep
    /// loop). A prefetch for another track (the playlist moved since) is
    /// replaced.
    pub(super) fn advance_race_track(&mut self) {
        let Some((discs, choice, cache_dir)) = self.race_context.clone() else {
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
        let wanted = RacePick::after(index);
        self.race_job = Some(match self.race_prefetch.take() {
            Some(prefetch) if prefetch.pick == wanted => prefetch,
            _ => MusicFetchWorker::spawn(discs, choice, cache_dir, wanted, "race-music"),
        });
        self.poll_race_job();
        self.settle_if_dumping();
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
                RacePick::exact(MUSIC_TRACK),
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
                RacePick::exact(index),
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
            Err(error) => warn_no_music("the music stays where it is", &error),
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
            Err(error) => warn_no_music("the race music stays where it is", &error),
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
        let fetched = worker.land().fetched;
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

/// Logs a music fetch that failed, as `audio: <what> (<why>)`.
///
/// A codec this platform cannot decode at all (`oag_music::at3::NoDecoderHere`:
/// Pulse's and Pure's ATRAC3+ in a browser) is said once, at warn, and every
/// later track it refuses goes to debug: the absence is the same one each time.
pub(crate) fn warn_no_music(what: &str, error: &anyhow::Error) {
    static SAID: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if error
        .chain()
        .any(|cause| cause.is::<oag_music::at3::NoDecoderHere>())
    {
        if SAID.swap(true, std::sync::atomic::Ordering::Relaxed) {
            debug!("audio: {what} ({error:#})");
        } else {
            warn!("audio: {what} ({error:#}); this music stays absent");
        }
        return;
    }
    warn!("audio: {what} ({error:#})");
}
