//! The front end's music, read and decoded on its own thread.
//!
//! [`Audio::start_music`] used to read and decode the loop on the frame
//! thread, from the front end's tick: measured on the Steam Deck at 231 ms for
//! HD (13 ms for its mix tables, 218 ms to decode the MP3) and 94 ms for Pulse,
//! on a frame the 40 ms warning does not see because the stage change marks it
//! a load stall. Now the frame thread starts a [`MenuJob`] and [`Audio::tick`]
//! starts the voice the tick it lands.

use super::*;

/// What the menu-music worker hands back.
pub(super) struct MenuLanded {
    /// HD's authored mix, when the booted title has one.
    maps: Option<hd_mix::Maps>,
    loaded: Result<Option<Loaded>>,
}

/// The menu music being read, polled by [`Audio::tick`].
pub(super) struct MenuJob(oag_thread::Task<MenuLanded>);

impl std::fmt::Debug for MenuJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuJob")
            .field("finished", &self.0.is_finished())
            .finish()
    }
}

impl MenuJob {
    /// Reads the mix tables and the menu's music for `discs` in the background.
    fn spawn(discs: MusicDiscs, choice: MusicSource, cache_dir: PathBuf) -> Self {
        Self(oag_thread::Task::spawn("menu-music", move || {
            let maps = match discs.booted() {
                Some(Platform::Ps3) => discs
                    .pick(MusicSource::Auto)
                    .and_then(|(source, _)| hd_mix::read_maps(discs.library, source)),
                _ => None,
            };
            // The PS2's front-end music is a soundtrack track, so it goes through
            // `fetch_track` and is stamped with its release; the PSP's is loaded
            // unstamped, and `music_from` staying `None` is the scoping rule.
            let loaded = match discs.booted() {
                Some(Platform::Ps2) => {
                    race_music::fetch_track(&discs, choice, &cache_dir, MUSIC_TRACK)
                }
                _ => front_end_music(&discs, &cache_dir),
            };
            MenuLanded { maps, loaded }
        }))
    }

    /// Takes the result, waiting natively if it is not in yet.
    pub(super) fn land(&mut self) -> MenuLanded {
        match self.0.join() {
            Some(Ok(landed)) => landed,
            Some(Err(_)) => MenuLanded {
                maps: None,
                loaded: Err(anyhow::anyhow!("the menu music read panicked")),
            },
            None => MenuLanded {
                maps: None,
                loaded: Err(anyhow::anyhow!("the menu music thread would not start")),
            },
        }
    }
}

impl Audio {
    /// Starts the music this source plays under the front end, looping.
    ///
    /// Not at boot: the intro reel (`LogoFMV`) carries its own sound on
    /// [`Bus::Music`], so the front end's tick loops call this once the
    /// sequence has left a movie state (`Frontend::is_playing_movie`). Callers
    /// with no sequence to wait on start it outright.
    ///
    /// The releases play different things, by the disc's doing: the PSP its own
    /// 28-second loop, the PS2 [`PS2_MUSIC_PATH`] track [`MUSIC_TRACK`], one of
    /// the sixteen. [`MusicSource`] cannot reach the PSP's;
    /// [`Self::set_music_source`] enforces it.
    ///
    /// **Reads nothing here**: a [`MenuJob`] reads and decodes it, and
    /// [`Self::tick`] starts the voice once it lands, unless a race has started
    /// meanwhile, in which case the sound is kept for
    /// [`Self::pause_race_music`].
    ///
    /// `cache_dir` is where a decoded ATRAC track lands
    /// (`oag_source::cache::default_audio_cache_dir`); the PS2 path ignores it.
    ///
    /// Never fatal: a source with no decodable music (or no `ffmpeg` on `PATH`
    /// for a PSP disc) says so in the log and plays nothing.
    pub fn start_music(&mut self, discs: &MusicDiscs, choice: MusicSource, cache_dir: &Path) {
        // Idempotent: the front end's tick loops call this every tick and track
        // no edge. See [`Self::music_attempted`].
        if self.music_attempted {
            return;
        }
        self.music_attempted = true;
        // A PS2 soundtrack track already read this run starts on the spot.
        if discs.booted() == Some(Platform::Ps2)
            && let Some((_, platform)) = discs.pick(choice)
            && let Some((_, sound)) = self.held.iter().find(|(held, _)| *held == platform)
        {
            let loaded = Loaded {
                from: Some(platform),
                sound: Arc::clone(sound),
                what: format!("{platform} soundtrack track {MUSIC_TRACK}, already read"),
            };
            self.apply_menu_landed(MenuLanded {
                maps: None,
                loaded: Ok(Some(loaded)),
            });
            return;
        }
        self.menu_job = Some(MenuJob::spawn(
            discs.clone(),
            choice,
            cache_dir.to_path_buf(),
        ));
        self.poll_menu_job();
        self.settle_if_dumping();
    }

    /// Applies [`Self::menu_job`] once it has landed; a no-op while in flight.
    pub(super) fn poll_menu_job(&mut self) {
        if !self
            .menu_job
            .as_ref()
            .is_some_and(|job| job.0.is_finished())
        {
            return;
        }
        if let Some(mut job) = self.menu_job.take() {
            self.apply_menu_landed(job.land());
        }
    }

    /// Installs HD's mix and starts the menu voice on a landed read.
    pub(super) fn apply_menu_landed(&mut self, landed: MenuLanded) {
        if let Some(maps) = landed.maps {
            self.set_hd_mix(maps);
        }
        match landed.loaded {
            Ok(Some(loaded)) => {
                let seconds = loaded.sound.seconds();
                // So `pause_race_music` can restart this decode without disc or `ffmpeg`.
                self.menu_sound = Some(Arc::clone(&loaded.sound));
                if let Some(platform) = loaded.from
                    && !self.held.iter().any(|(held, _)| *held == platform)
                {
                    self.held.push((platform, Arc::clone(&loaded.sound)));
                }
                // A race that started while this was read owns the music bus;
                // `pause_race_music` starts the loop from `menu_sound` later.
                if self.race_context.is_some() || self.race_voice.is_some() || self.music.is_some()
                {
                    debug!("audio: music {} kept for after the race", loaded.what);
                    return;
                }
                self.music = self
                    .output
                    .with_mixer(|mixer| mixer.play(Play::looping(loaded.sound, Bus::Music)));
                self.music_from = self.music.and(loaded.from);
                debug!("audio: music {}, {seconds:.1} s, looping", loaded.what);
            }
            Ok(None) => warn!("audio: this source carries no music this can play"),
            Err(error) => race_music::warn_no_music("no music", &error),
        }
    }
}
