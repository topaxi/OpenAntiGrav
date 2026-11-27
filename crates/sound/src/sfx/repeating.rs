//! A cue whose command list runs while it is held, played for as long as it is.
//!
//! `~BLOWUP` re-keys a waveform on a loop while the explosion holds
//! the handler, and `~ROCKLOCK` beeps at the tempo a cue parameter picks; HD's
//! `~AUTOPILOT` and travel loops do the same over a longer list. All stop only
//! when the game releases the handle, so none is a finite
//! [`Timeline`](super::layers::Timeline). [`Playing`] drives the list's
//! [`Runner`](oag_formats::sblk::runner::Runner) from the simulation tick and
//! starts each key-on with the sub-tick delay that keeps beeps at their authored
//! spacing, not a 60 Hz grid.

use oag_audio::{Bus, Mixer, VoiceId};
use oag_core::Rng;

use super::CueTables;
use super::layers::{CueVoice, Program, RearPan, Where, bend_pitch, pan_for, start_panned};
use super::{Banks, Cue};

/// One voice a handle has keyed that may still be sounding.
#[derive(Debug, Clone, Copy)]
struct Live {
    id: VoiceId,
    /// The bend range of its descriptor, down and up.
    range: (i8, i8),
    angle: i32,
    rear: RearPan,
    /// Whether the voices of its play take their authored angles.
    panned: bool,
}

/// A held cue's open handle: its list part-way through, and the voices it
/// has keyed that may still be sounding.
#[derive(Debug, Clone)]
pub struct Playing {
    program: Program,
    /// Seconds of play the list has been advanced through.
    now: f64,
    voices: Vec<Live>,
    /// Where the handle was last heard, which its release plays from.
    at: Where,
    /// The handler's bend the live voices were last set to.
    bend: i16,
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
            at,
            bend: 0,
            keyed: 0,
        };
        let grains = playing.program.runner.start(&mut || rng.next_u32());
        playing.key(grains, 0.0, mixer, bus, at);
        playing
    }

    /// Moves the live voices to `at`, each keeping its own angle from the
    /// emitter, as the original's emitter update does for a held cue.
    pub fn place(&mut self, mixer: &mut Mixer, at: Where) {
        self.at = at;
        for voice in &self.voices {
            mixer.set_gain(voice.id, at.gain);
            mixer.set_pan(voice.id, pan_for(at, voice.angle, voice.panned, voice.rear));
            mixer.set_front(voice.id, at.front);
        }
    }

    /// The handler's bend as the list has left it.
    #[must_use]
    pub fn bend(&self) -> i16 {
        self.program.runner.bend()
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
        self.at = at;
        self.voices.retain(|v| mixer.is_playing(v.id));
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

    /// Ends the handle without its release: every voice it keyed stops,
    /// sounding or not yet. For leaving a race.
    pub fn stop(self, mixer: &mut Mixer) {
        for voice in self.voices {
            mixer.stop(voice.id);
        }
    }

    /// The game's stop of the handle: its voices stop, then the commands after
    /// the list's `0x2b` play, untracked
    /// ([`Runner::release`](oag_formats::sblk::runner::Runner::release)).
    pub fn release(mut self, mixer: &mut Mixer, bus: Bus, rng: &mut Rng) {
        for voice in self.voices.drain(..) {
            mixer.stop(voice.id);
        }
        let grains = self.program.runner.release(&mut || rng.next_u32());
        let at = self.at;
        self.key(grains, 0.0, mixer, bus, at);
    }

    fn key(
        &mut self,
        grains: Vec<oag_formats::sblk::timeline::Grain>,
        offset: f64,
        mixer: &mut Mixer,
        bus: Bus,
        at: Where,
    ) {
        // `Scream_SetVoiceBend` is the handler's: it reaches the voices already
        // keyed as well as the ones keyed after it.
        let bend = self.program.runner.bend();
        if bend != self.bend {
            self.bend = bend;
            for voice in &self.voices {
                let (down, up) = voice.range;
                mixer.set_pitch(voice.id, bend_pitch(f32::from(bend), down, up));
            }
        }
        let panned = self.program.panned || grains.iter().any(|g| g.angle != 0);
        for grain in &grains {
            let Some(layer) = self.program.layers.get(&grain.sound.command) else {
                continue;
            };
            let range = layer.bend.map_or((0, 0), |b| (b.down, b.up));
            let voice = CueVoice {
                sound: layer.sound.clone(),
                looping: layer.looping,
                delay: offset,
                angle: layer.angle,
                rear: layer.rear,
                pitch: bend_pitch(f32::from(bend), range.0, range.1),
            };
            let ids = start_panned(mixer, std::slice::from_ref(&voice), bus, at, panned);
            self.keyed += 1;
            self.voices.extend(ids.into_iter().map(|id| Live {
                id,
                range,
                angle: layer.angle,
                rear: layer.rear,
                panned,
            }));
        }
    }
}

/// The race's open unplaced handles: the explosion, the lock-on tone and the
/// Autopilot. Each drive method returns whether a program played the cue, so a
/// title with none keeps its caller's one-shot path.
#[derive(Default)]
pub struct Held {
    blowup: Option<Playing>,
    lock: Option<Playing>,
    autopilot: Option<Playing>,
}

pub(super) const TICK_SECONDS: f64 = 1.0 / super::super::TICK_HZ as f64;

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

    /// `~AUTOPILOT`: opened when the player's Autopilot starts running and
    /// released when it ends, which plays the list's tail. HD fires it unplaced
    /// (`Sound_PlayBankCue`) and stops it in `Autopilot_Update`. See
    /// [`Cue::Autopilot`].
    pub fn drive_autopilot(
        &mut self,
        active: bool,
        banks: &Banks,
        mixer: &mut Mixer,
        rng: &mut Rng,
    ) -> bool {
        let Some(program) = banks.program(Cue::Autopilot) else {
            return false;
        };
        let bus = Cue::Autopilot.bus();
        match (active, self.autopilot.is_some()) {
            (true, false) => {
                self.autopilot = Some(Playing::open(program, mixer, bus, Where::DRY, rng));
                log::trace!("sfx: held Autopilot opened, running its list");
            }
            (false, true) => {
                if let Some(playing) = self.autopilot.take() {
                    log::trace!("sfx: held Autopilot released");
                    playing.release(mixer, bus, rng);
                }
            }
            _ => {}
        }
        if let Some(playing) = &mut self.autopilot {
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
        let open = [self.blowup.take(), self.lock.take(), self.autopilot.take()];
        for playing in open.into_iter().flatten() {
            playing.stop(mixer);
        }
    }
}
