//! A cue whose command list repeats, played for as long as it is held.
//!
//! `~BLOWUP` re-keys a waveform every 43 master ticks while the explosion holds
//! the handler, and `~ROCKLOCK` beeps at the tempo a cue parameter picks; both
//! stop only when the game releases the handle, so neither is a finite
//! [`Timeline`](super::layers::Timeline). [`Playing`] drives the list's
//! [`Runner`](oag_formats::sblk::runner::Runner) from the simulation tick and
//! starts each key-on with the sub-tick delay that keeps beeps at their authored
//! spacing, not a 60 Hz grid.

use oag_audio::{Bus, Mixer, VoiceId};
use oag_core::Rng;

use super::layers::{CueVoice, Program, Where, start};
use super::{Banks, Cue};

/// A repeating cue's open handle: its list part-way through, and the voices it
/// has keyed that may still be sounding.
#[derive(Debug)]
pub struct Playing {
    program: Program,
    /// Seconds of play the list has been advanced through.
    now: f64,
    voices: Vec<VoiceId>,
    keyed: usize,
}

impl Playing {
    /// Opens `program` and runs its zero-delay prefix, as `Scream_StartSound`
    /// does before it hands the handle back.
    pub fn open(program: &Program, mixer: &mut Mixer, bus: Bus, at: Where, rng: &mut Rng) -> Self {
        let mut playing = Self {
            program: program.clone(),
            now: 0.0,
            voices: Vec::new(),
            keyed: 0,
        };
        let grains = playing.program.runner.start(&mut || rng.next_u32());
        playing.key(grains, 0.0, mixer, bus, at);
        playing
    }

    /// How many voices the list has keyed on since it opened, sounding or not.
    #[must_use]
    pub fn keyed(&self) -> usize {
        self.keyed
    }

    /// Writes the handle's cue parameter, as `Scream_SetCueParameter` does.
    pub fn set_parameter(&mut self, index: usize, value: i8) {
        self.program.runner.set_parameter(index, value);
    }

    /// Advances the list by `seconds` of simulation time, keying on whatever
    /// falls due and holding it back by how far into the tick it falls.
    pub fn advance(&mut self, seconds: f64, mixer: &mut Mixer, bus: Bus, at: Where, rng: &mut Rng) {
        self.voices.retain(|&id| mixer.is_playing(id));
        let end = self.now + seconds;
        loop {
            let next =
                f64::from(self.program.runner.tick_count() + 1) / self.program.ticks_per_second;
            if next >= end || self.program.runner.ended() {
                break;
            }
            let grains = self.program.runner.tick(&mut || rng.next_u32());
            self.key(grains, (next - self.now).max(0.0), mixer, bus, at);
        }
        self.now = end;
    }

    /// Releases the handle: every voice it keyed stops, sounding or not yet.
    pub fn stop(self, mixer: &mut Mixer) {
        for id in self.voices {
            mixer.stop(id);
        }
    }

    fn key(
        &mut self,
        grains: Vec<oag_formats::sblk::timeline::Grain>,
        offset: f64,
        mixer: &mut Mixer,
        bus: Bus,
        at: Where,
    ) {
        let voices: Vec<CueVoice> = grains
            .iter()
            .filter_map(|grain| self.program.layers.get(&grain.sound.command))
            .map(|layer| CueVoice {
                sound: layer.sound.clone(),
                looping: layer.looping,
                delay: offset,
                angle: layer.angle,
                pitch: 1.0,
            })
            .collect();
        let ids = start(mixer, &voices, bus, at);
        self.keyed += voices.len();
        self.voices.extend(ids);
    }
}

/// The race's open repeating handles: the explosion and the lock-on tone. Each
/// drive method returns whether a program played the cue, so a title with no
/// measured tick keeps its caller's one-shot path.
#[derive(Default)]
pub struct Held {
    blowup: Option<Playing>,
    lock: Option<Playing>,
}

/// One simulation tick, in seconds.
const TICK_SECONDS: f64 = 1.0 / super::super::TICK_HZ as f64;

impl Held {
    /// `~BLOWUP`: opened when the player's craft starts exploding, re-keying its
    /// list until it ends. See [`Cue::Blowup`].
    pub fn drive_blowup(
        &mut self,
        exploding: bool,
        banks: &Banks,
        mixer: &mut Mixer,
        rng: &mut Rng,
    ) -> bool {
        let Some(program) = banks.program(Cue::Blowup) else {
            return false;
        };
        let bus = Cue::Blowup.bus();
        match (exploding, self.blowup.is_some()) {
            (true, false) => {
                self.blowup = Some(Playing::open(program, mixer, bus, Where::DRY, rng));
            }
            (false, true) => {
                if let Some(playing) = self.blowup.take() {
                    playing.stop(mixer);
                }
            }
            _ => {}
        }
        if let Some(playing) = &mut self.blowup {
            playing.advance(TICK_SECONDS, mixer, bus, Where::DRY, rng);
        }
        true
    }

    /// `~ROCKLOCK`: one voice from the first frame the reticle has anything,
    /// parameter `0` seeking and `1` locked, stopped when the target goes
    /// (`HudSight_UpdateTone`'s shape). See [`Cue::LockOn`].
    pub fn drive_sight(
        &mut self,
        state: oag_race::sight::State,
        banks: &Banks,
        mixer: &mut Mixer,
        rng: &mut Rng,
    ) -> bool {
        use oag_race::sight::State;
        let Some(program) = banks.program(Cue::LockOn) else {
            return false;
        };
        let bus = Cue::LockOn.bus();
        match state {
            State::Absent => {
                if let Some(playing) = self.lock.take() {
                    playing.stop(mixer);
                }
            }
            State::Seeking | State::Locked => {
                let playing = self
                    .lock
                    .get_or_insert_with(|| Playing::open(program, mixer, bus, Where::DRY, rng));
                playing.set_parameter(0, i8::from(state == State::Locked));
                playing.advance(TICK_SECONDS, mixer, bus, Where::DRY, rng);
            }
        }
        true
    }

    /// Releases every open handle, on leaving the race.
    pub fn stop_all(&mut self, mixer: &mut Mixer) {
        for playing in [self.blowup.take(), self.lock.take()].into_iter().flatten() {
            playing.stop(mixer);
        }
    }
}
