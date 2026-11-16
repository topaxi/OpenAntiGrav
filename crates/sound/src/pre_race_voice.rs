//! The pre-race flyby's voice-over, played through the music player.
//!
//! Pulse's `RaceMode_UpdateIntro` starts the circuit's `PreRaceDialogue` clip as its lock counter
//! reaches zero and stops the music player when the flyby ends
//! (`docs/ghidra/functions/psp-pulse-usa/race-intro.md`, "The music"). The composition root asks
//! for the clip's decode at race load ([`Audio::prepare_pre_race_voice`], off the frame thread)
//! and forwards the flyby's two calls ([`Audio::start_pre_race_voice`],
//! [`Audio::stop_pre_race_voice`]). The clip has a voice of its own: the race song waits parked
//! under [`Audio::hold_race_song`] and a stop here never touches it.

use super::*;

/// The decode in flight, the decoded clip and the voice playing it.
#[derive(Default)]
pub(super) struct PreRaceVoice {
    job: Option<oag_thread::Task<Result<Sound>>>,
    clip: Option<Arc<Sound>>,
    /// The flyby has asked for the clip and no stop has followed.
    wanted: bool,
    playing: Option<VoiceId>,
}

impl std::fmt::Debug for PreRaceVoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreRaceVoice")
            .field("decoding", &self.job.is_some())
            .field("decoded", &self.clip.is_some())
            .field("wanted", &self.wanted)
            .field("playing", &self.playing)
            .finish()
    }
}

impl Audio {
    /// Starts decoding the archive entry `entry` of the booted disc, replacing any clip held.
    ///
    /// Reads nothing on the calling thread. A `--dump-audio` capture waits for the decode so the
    /// clip starts on the tick the flyby asks for it, as [`Self::settle_music`] does for music.
    pub fn prepare_pre_race_voice(&mut self, discs: &MusicDiscs, entry: &str, cache_dir: &Path) {
        self.stop_pre_race_voice();
        self.pre_race_voice.clip = None;
        let discs = discs.clone();
        let entry = entry.to_string();
        let cache_dir = cache_dir.to_path_buf();
        self.pre_race_voice.job = Some(oag_thread::Task::spawn("pre-race-voice", move || {
            discs.wait_surveyed();
            let (source, _) = discs
                .pick(MusicSource::Auto)
                .context("no booted disc to read the pre-race voice-over from")?;
            library::load_named(discs.library, source, &entry, &cache_dir)
        }));
        if self.dump.is_some() {
            self.settle_pre_race_voice();
        }
    }

    /// Waits for the decode [`Self::prepare_pre_race_voice`] started, for a caller that needs the
    /// clip to start on a known tick.
    pub fn settle_pre_race_voice(&mut self) {
        self.land_pre_race_voice(true);
    }

    /// The flyby's counter reached zero: plays the clip, now or on the tick it lands.
    pub fn start_pre_race_voice(&mut self) {
        self.pre_race_voice.wanted = true;
        self.play_pre_race_voice();
    }

    /// The flyby ended: stops the clip, or drops it if it has not landed.
    pub fn stop_pre_race_voice(&mut self) {
        self.pre_race_voice.wanted = false;
        if let Some(id) = self.pre_race_voice.playing.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }
    }

    /// Whether the clip is sounding.
    #[must_use]
    pub fn pre_race_voice_playing(&self) -> bool {
        self.pre_race_voice
            .playing
            .is_some_and(|id| self.output.with_mixer(|mixer| mixer.is_playing(id)))
    }

    /// Picks up a finished decode and starts the clip if the flyby is waiting for it.
    pub(super) fn poll_pre_race_voice(&mut self) {
        self.land_pre_race_voice(false);
        self.play_pre_race_voice();
    }

    fn land_pre_race_voice(&mut self, wait: bool) {
        let Some(job) = &mut self.pre_race_voice.job else {
            return;
        };
        if !wait && !job.is_finished() {
            return;
        }
        let landed = job.join();
        self.pre_race_voice.job = None;
        match landed {
            Some(Ok(Ok(sound))) => self.pre_race_voice.clip = Some(Arc::new(sound)),
            Some(Ok(Err(why))) => warn!("audio: the pre-race voice-over plays silently ({why:#})"),
            _ => warn!("audio: the pre-race voice-over decode did not run"),
        }
    }

    fn play_pre_race_voice(&mut self) {
        if !self.pre_race_voice.wanted || self.pre_race_voice.playing.is_some() {
            return;
        }
        let Some(clip) = self.pre_race_voice.clip.clone() else {
            return;
        };
        self.pre_race_voice.playing = self
            .output
            .with_mixer(|mixer| mixer.play(Play::once(clip, Bus::Music)));
        if self.pre_race_voice.playing.is_none() {
            warn!("audio: no free voice for the pre-race voice-over, so it plays silently");
        }
    }
}
