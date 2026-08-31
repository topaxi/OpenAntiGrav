//! The mixer: voices, buses, and the sample loop that turns them into output.
//!
//! Plain data with no device handle, so it runs identically under a real
//! output stream, under the offline dump and under `cargo nextest` on a machine
//! with no sound card. That is the same split
//! [`oag_render::sparks::Sparks`](../../../render/src/sparks.rs) makes against
//! its GPU pipeline, and for the same reason.
//!
//! # Two clocks
//!
//! Control and audio run at different rates and this module keeps them apart.
//! [`Mixer::play`], [`Mixer::set_pitch`] and [`Mixer::set_gain`] are *control*
//! and belong on the simulation's 60 Hz tick; [`Mixer::render`] is *audio* and
//! fills however many frames the output asks for. Nothing here reads a clock -
//! elapsed time is whatever the caller renders, which is what makes the offline
//! dump reproduce the real-time path exactly rather than approximately.
//!
//! # Not yet `sceSasCore`
//!
//! The original mixes on the PSP's hardware synth, so an accurate path
//! eventually means ADSR envelopes and hardware reverb - see
//! `docs/ghidra/functions/psp-pulse-usa/imports.md`. This implements pitch and
//! gain, which is the part whose law is actually recovered
//! (`Exhaust_UpdateEngineSound`, confidence 80). Envelopes and reverb are a
//! deliberate, recorded deferral, not an oversight.

use std::sync::Arc;

/// Output channel count. Everything is mixed to interleaved stereo.
pub const CHANNELS: usize = 2;

/// How many voices can sound at once.
///
/// The PSP's `sceSasCore` has exactly 32, and matching it means a voice-stealing
/// bug shows up here rather than only on hardware.
pub const MAX_VOICES: usize = 32;

/// A mix bus: one class of sound, with its own gain.
///
/// **Not one per row of the original's options menu.** That menu has two and
/// only two - `"Music Volume"` at `0x08a78658` and `"SFX Volume"` at
/// `0x08a78668` - and this had two variants because of it. The original's own
/// mixer is not built that way: `Audio_SetGroupVolume` (`0x089956d8`) carries
/// **sixteen** group volumes plus a master, and `Audio_UpdateGroupVolumes`
/// drives nine of them every frame, one of them through a duck. Classes below
/// the menu rows are the original's structure, not a port-side invention. See
/// [`audio-levels.md`](../../../../docs/ghidra/functions/psp-pulse-usa/audio-levels.md)
/// and [ADR-0027](../../../../docs/architecture/adr/0027-three-mix-buses.md).
///
/// Three of those classes are exposed here. Which recovered *group index* each
/// corresponds to is not known, so this is a port-side split along a line the
/// original draws, not a transcription of its table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bus {
    /// Streamed music.
    Music,
    /// Effects: the engine, collisions, pads, weapons.
    Sfx,
    /// Voice lines - the announcer, and anything else the original keeps in
    /// `speech.bnk` rather than beside the effects.
    Speech,
}

impl Bus {
    /// Number of buses, for array sizing.
    pub const COUNT: usize = 3;

    /// This bus's index into a `[_; Bus::COUNT]`.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Music => 0,
            Self::Sfx => 1,
            Self::Speech => 2,
        }
    }
}

/// Output frames a stopped voice fades over rather than being cut dead.
///
/// About 5.8 ms at 44.1 kHz - long enough that the ramp itself carries no
/// audible energy of its own, and short enough that a cue losing the level of
/// its last few milliseconds is not a cue that sounds shorter.
///
/// **Length is bounded from both ends, which is why it is not shorter.** The
/// first attempt at this held a finished voice's last sample and decayed
/// *that*; 64 frames of it was still audible, because a held value ramped to
/// zero is a unipolar pulse and a unipolar pulse is a thump - and lengthening
/// it only moves the pulse deeper into the bass. Ramping the waveform itself
/// has no such floor, so the fade can be as long as it needs to be.
///
/// **Measured, not chosen for tidiness.** Every held cue on the Pulse discs
/// peaks close to full scale - `~ENGINE` at 0.958, `~SHIELD` at 0.949,
/// `~ROCKLOCK` at 0.949 - and [`Mixer::stop`] used to clear the slot outright,
/// which cuts the waveform wherever the playhead happened to be. A step from
/// most of full scale to zero is a click, and a click through a speaker is
/// heard as a thump rather than as the silence it actually is. A one-shot
/// running to its own end does not need this - the same measurement puts every
/// cue's last sample under 0.02 - but a *stopped* one has no reason to be
/// anywhere near zero.
///
/// **This is not the hardware's envelope.** `__sceSasSetADSR`'s release is real
/// and unrecovered ([ADR-0018](../../../docs/architecture/adr/0018-audio-mixer-architecture.md)
/// defers it); this is the shortest fade that removes a discontinuity, and it
/// is deliberately too short to be mistaken for one.
const RELEASE_FRAMES: u32 = 256;

/// Decoded audio, ready to play.
///
/// Interleaved 16-bit signed, the form both PS-ADPCM decode
/// ([`oag_formats::sblk::decode_adpcm`]) and the PS2 PCM archives already
/// produce, so nothing is converted on the way in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sound {
    samples: Vec<i16>,
    channels: u16,
    sample_rate: u32,
}

impl Sound {
    /// Wraps decoded interleaved samples.
    ///
    /// # Errors
    ///
    /// If `channels` is zero, `sample_rate` is zero, or the sample count is not
    /// a whole number of frames.
    pub fn new(samples: Vec<i16>, channels: u16, sample_rate: u32) -> anyhow::Result<Self> {
        anyhow::ensure!(channels > 0, "a sound needs at least one channel");
        anyhow::ensure!(sample_rate > 0, "a sound needs a non-zero sample rate");
        anyhow::ensure!(
            samples.len().is_multiple_of(channels as usize),
            "{} samples is not a whole number of {channels}-channel frames",
            samples.len()
        );
        Ok(Self {
            samples,
            channels,
            sample_rate,
        })
    }

    /// Frames, meaning sample groups, not individual samples.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels as usize
    }

    /// Channels per frame.
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// The rate the samples were recorded at.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Duration in seconds.
    #[must_use]
    pub fn seconds(&self) -> f32 {
        self.frames() as f32 / self.sample_rate as f32
    }

    /// One frame as a stereo pair in `-1.0..=1.0`.
    ///
    /// A mono source is duplicated to both channels; a source with more than
    /// two channels keeps the first two. Out-of-range frames are silence, which
    /// is what makes the interpolator's read one past the end safe.
    fn frame(&self, index: usize) -> [f32; CHANNELS] {
        if index >= self.frames() {
            return [0.0; CHANNELS];
        }
        let at = index * self.channels as usize;
        let left = f32::from(self.samples[at]) / -f32::from(i16::MIN);
        let right = if self.channels == 1 {
            left
        } else {
            f32::from(self.samples[at + 1]) / -f32::from(i16::MIN)
        };
        [left, right]
    }
}

/// Identifies a playing voice for later control.
///
/// Carries a generation, so a handle to a voice that has since finished and
/// been reused addresses nothing rather than silently retargeting whatever took
/// its slot. That matters for the looping cues the original marks with a `~`
/// prefix - `"~ENGINE"` is held for a ship's whole race and must not start
/// steering somebody else's sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceId {
    slot: u16,
    generation: u32,
}

#[derive(Debug, Clone, Default)]
struct Voice {
    sound: Option<Arc<Sound>>,
    bus: Option<Bus>,
    /// Playback position in source frames. Fractional: the step is the rate
    /// ratio times pitch, so a 48 kHz track on a 44.1 kHz device advances by
    /// about 1.088 frames per output frame.
    position: f64,
    pitch: f32,
    gain: f32,
    /// The recovered stereo position, or [`None`] for a voice that has none.
    /// See [`Play::pan`].
    pan: Option<f32>,
    looping: bool,
    generation: u32,
    /// Output frames left of the fade a stopped voice goes out on, or `None`
    /// while it is sounding normally. See [`RELEASE_FRAMES`].
    release: Option<u32>,
}

impl Voice {
    fn is_free(&self) -> bool {
        self.sound.is_none()
    }

    /// Takes the handle away and starts the fade out.
    ///
    /// Generation zero is the one no [`VoiceId`] can carry - [`Mixer::play`]
    /// increments before it hands one out, so the first is one - which is what
    /// makes a releasing voice unaddressable without a second flag. Called
    /// twice, the fade is not restarted: the second stop of a voice already on
    /// its way out would otherwise make it louder again.
    fn release(&mut self) {
        if self.is_free() {
            return;
        }
        self.generation = 0;
        self.release.get_or_insert(RELEASE_FRAMES);
    }
}

/// How a voice is started.
#[derive(Debug, Clone)]
pub struct Play {
    /// What to play.
    pub sound: Arc<Sound>,
    /// Which volume controls it.
    pub bus: Bus,
    /// Linear gain, `1.0` being unattenuated.
    pub gain: f32,
    /// Playback rate multiplier, `1.0` being the source's own rate.
    pub pitch: f32,
    /// Where between the speakers this sits: `-1.0` left, `+1.0` right.
    ///
    /// **[`None`] means "this voice has no position", not "centred".** The two
    /// differ by 3 dB and the difference is deliberate: the original's pan
    /// table gives a *centred* source `0.7071` in each channel
    /// ([`crate::spatial::pan_gains`]), so a music stream or a movie run
    /// through the panner at zero would come out quieter than it is meant to
    /// be. A voice whose position was never recovered is left alone.
    pub pan: Option<f32>,
    /// Whether to restart at the end instead of stopping.
    pub looping: bool,
}

impl Play {
    /// A one-shot on the effects bus at unit gain and unit pitch.
    #[must_use]
    pub fn once(sound: Arc<Sound>, bus: Bus) -> Self {
        Self {
            sound,
            bus,
            gain: 1.0,
            pitch: 1.0,
            pan: None,
            looping: false,
        }
    }

    /// A looping voice, the shape the original's `~`-prefixed cues take.
    #[must_use]
    pub fn looping(sound: Arc<Sound>, bus: Bus) -> Self {
        Self {
            looping: true,
            ..Self::once(sound, bus)
        }
    }
}

/// The voice pool and the bus gains, and the loop that renders them.
#[derive(Debug, Clone)]
pub struct Mixer {
    voices: Vec<Voice>,
    bus_gain: [f32; Bus::COUNT],
    master_gain: f32,
    sample_rate: u32,
    generation: u32,
    /// Voices refused because every slot was busy, for the perf overlay and
    /// for tests. Silent starvation is the failure mode worth being able to
    /// see.
    starved: u64,
    /// Output samples the sum drove past full scale, for the same reason
    /// [`Self::starved`] is counted. See [`Self::render`].
    clipped: u64,
    /// Sample-rate remainder carried between [`Self::render_tick`] calls.
    ///
    /// **Because `sample_rate / tick_hz` is not always a whole number.** 48,000
    /// and 44,100 both divide by 60 exactly, which is why truncating went
    /// unnoticed until finding U7 of the 2026-08-18 review; a rate that does
    /// not divide dropped up to `tick_hz - 1` frames a second, so the
    /// `--dump-audio` WAV drifted steadily behind the simulation it is supposed
    /// to be comparable with. Carrying the remainder makes the *average* exact
    /// while every individual tick stays within one frame of nominal.
    tick_frame_remainder: u32,
}

impl Mixer {
    /// A mixer rendering at `sample_rate`, all buses open.
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        Self {
            voices: vec![Voice::default(); MAX_VOICES],
            bus_gain: [1.0; Bus::COUNT],
            master_gain: 1.0,
            sample_rate: sample_rate.max(1),
            generation: 0,
            starved: 0,
            clipped: 0,
            tick_frame_remainder: 0,
        }
    }

    /// The rate [`Mixer::render`] produces.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Sets one bus's gain, which is what a volume slider moves.
    pub fn set_bus_gain(&mut self, bus: Bus, gain: f32) {
        self.bus_gain[bus.index()] = gain.max(0.0);
    }

    /// Reads one bus's gain.
    #[must_use]
    pub fn bus_gain(&self, bus: Bus) -> f32 {
        self.bus_gain[bus.index()]
    }

    /// Scales everything, after the buses. Used to mute wholesale.
    pub fn set_master_gain(&mut self, gain: f32) {
        self.master_gain = gain.max(0.0);
    }

    /// How many voices are live - started, not stopped, not finished.
    ///
    /// **A voice on its release fade is not counted, though it is still
    /// sounding and still holding its slot** for up to [`RELEASE_FRAMES`]
    /// output frames. Every caller reads this as "did the thing I started get
    /// released", which is a question about the handle rather than about the
    /// last millisecond and a half of its tail; counting the fade would make
    /// "stopped" mean "stopped, then rendered", which is a distinction no
    /// caller has.
    #[must_use]
    pub fn active_voices(&self) -> usize {
        self.voices
            .iter()
            .filter(|v| !v.is_free() && v.release.is_none())
            .count()
    }

    /// How many starts have been refused for want of a free slot.
    #[must_use]
    pub fn starved(&self) -> u64 {
        self.starved
    }

    /// How many output samples the voice sum has driven past full scale.
    ///
    /// Zero is a mix with headroom left. Anything else is the mix saturating -
    /// see [`Self::render`] for why that is what the console does rather than
    /// something to be fixed by turning a bus down.
    #[must_use]
    pub fn clipped(&self) -> u64 {
        self.clipped
    }

    /// Starts a voice, returning a handle to steer it with.
    ///
    /// Returns `None` when every slot is busy, or when the play could never
    /// finish. Refusing rather than stealing is deliberate for now: stealing
    /// needs a priority the original keeps in the bank's undecoded cue table,
    /// and inventing one would be a guess dressed as behaviour.
    pub fn play(&mut self, play: Play) -> Option<VoiceId> {
        // **A pitch of zero is refused, not clamped to zero.** `pitch.max(0.0)`
        // accepted it, and a voice whose playhead never advances emits a
        // constant DC offset - inaudible, but a bias on the bus - and holds its
        // slot until something stops it by handle, which for a fire-and-forget
        // cue is never. It counts as starvation because that is what it causes:
        // one fewer slot, visible in the overlay rather than silent. Finding U6
        // of the 2026-08-18 review. A negative or NaN pitch goes the same way.
        if play.pitch <= 0.0 || play.pitch.is_nan() {
            self.starved += 1;
            return None;
        }
        let slot = self.voices.iter().position(Voice::is_free).or_else(|| {
            self.starved += 1;
            None
        })?;
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        self.voices[slot] = Voice {
            sound: Some(play.sound),
            bus: Some(play.bus),
            position: 0.0,
            pitch: play.pitch,
            gain: play.gain.max(0.0),
            pan: play.pan,
            looping: play.looping,
            generation,
            release: None,
        };
        Some(VoiceId {
            slot: slot as u16,
            generation,
        })
    }

    /// Stops a voice. A stale handle does nothing.
    ///
    /// The handle is dead the moment this returns - [`Self::is_playing`] says
    /// no and nothing can retune it - but the slot is held for
    /// [`RELEASE_FRAMES`] more output frames while the voice fades, because
    /// clearing it here is what put a step into the mix. See that constant.
    pub fn stop(&mut self, id: VoiceId) {
        if let Some(voice) = self.voice_mut(id) {
            voice.release();
        }
    }

    /// Stops everything on one bus, which is how a race teardown silences its
    /// engines without touching the music.
    pub fn stop_bus(&mut self, bus: Bus) {
        for voice in &mut self.voices {
            if voice.bus == Some(bus) {
                voice.release();
            }
        }
    }

    /// Stops every voice.
    pub fn stop_all(&mut self) {
        for voice in &mut self.voices {
            voice.release();
        }
    }

    /// Retunes a live voice. This is the per-tick engine-note control.
    pub fn set_pitch(&mut self, id: VoiceId, pitch: f32) {
        if let Some(voice) = self.voice_mut(id) {
            voice.pitch = pitch.max(0.0);
        }
    }

    /// Re-levels a live voice.
    pub fn set_gain(&mut self, id: VoiceId, gain: f32) {
        if let Some(voice) = self.voice_mut(id) {
            voice.gain = gain.max(0.0);
        }
    }

    /// Moves a live voice between the speakers, or takes its position away.
    ///
    /// The per-tick half of positional audio: the original recomputes an
    /// emitter's angle every frame and pushes it at the sounding instance
    /// (`SoundInstance_UpdateSpatial`), so a craft that overtakes the camera
    /// crosses the stereo field rather than jumping at its next cue.
    pub fn set_pan(&mut self, id: VoiceId, pan: Option<f32>) {
        if let Some(voice) = self.voice_mut(id) {
            voice.pan = pan;
        }
    }

    /// Whether the handle still addresses a sounding voice.
    #[must_use]
    pub fn is_playing(&self, id: VoiceId) -> bool {
        self.voice(id).is_some()
    }

    /// How far into its own source a voice has played, in seconds.
    ///
    /// The source's clock, not the device's: [`Voice::position`] counts source
    /// frames, so a 48 kHz track on a 44.1 kHz device reports the second of
    /// itself that is sounding rather than the second of output that carried
    /// it. That is what a caller pacing something else against this wants -
    /// `crate::mixer` is the only place that knows the resampling step, and a
    /// playhead in output seconds would make every consumer undo it.
    ///
    /// `None` for a handle that no longer addresses a sounding voice, which is
    /// the same answer [`Self::is_playing`] gives and for the same reason: a
    /// finished voice's slot may already belong to somebody else.
    #[must_use]
    pub fn position(&self, id: VoiceId) -> Option<f64> {
        let voice = self.voice(id)?;
        let sound = voice.sound.as_ref()?;
        Some(voice.position / f64::from(sound.sample_rate()))
    }

    /// Moves a live voice's playhead to `seconds` into its own source.
    ///
    /// **The source's clock, not the device's**, exactly as [`Self::position`]
    /// reports it - the two are each other's inverse, and that is the whole
    /// reason this takes seconds rather than frames. Handing one voice's frame
    /// count to another voice at a different rate is the mistake it exists to
    /// make impossible: 48,000 frames into a 48 kHz track is one second, and
    /// into a 44,100 Hz one it is 1.088.
    ///
    /// Past the end of a looping sound this wraps rather than ending the voice,
    /// which is the same rule [`Self::render`] applies when the playhead runs
    /// off the end of one. On a one-shot it lands past the end and the voice
    /// finishes on the next render. A negative or non-finite `seconds` is
    /// clamped to the start rather than refused: there is nothing a caller
    /// could usefully do with a failure here, and a voice at a NaN position
    /// would silently render nothing for ever.
    ///
    /// Returns whether the handle still addressed a sounding voice.
    pub fn seek(&mut self, id: VoiceId, seconds: f64) -> bool {
        let Some(voice) = self.voice_mut(id) else {
            return false;
        };
        let Some(sound) = voice.sound.as_ref() else {
            return false;
        };
        let frames = sound.frames() as f64;
        let rate = f64::from(sound.sample_rate());
        let mut position = if seconds.is_finite() {
            (seconds * rate).max(0.0)
        } else {
            0.0
        };
        if voice.looping && frames > 0.0 {
            position %= frames;
        }
        voice.position = position;
        true
    }

    fn voice(&self, id: VoiceId) -> Option<&Voice> {
        self.voices
            .get(usize::from(id.slot))
            .filter(|v| !v.is_free() && v.generation == id.generation)
    }

    fn voice_mut(&mut self, id: VoiceId) -> Option<&mut Voice> {
        self.voices
            .get_mut(usize::from(id.slot))
            .filter(|v| !v.is_free() && v.generation == id.generation)
    }

    /// Renders `out.len() / CHANNELS` frames of interleaved stereo, summing
    /// every live voice.
    ///
    /// Overwrites `out` rather than adding to it, and always fills it whole -
    /// silence is a buffer of zeroes, never a short write, because a short
    /// write to an output callback is a click.
    ///
    /// # The sum saturates, and that is the console's own answer
    ///
    /// Eight craft on the grid hold eight `~ENGINE` voices inside the flare's
    /// 50-unit radius at once, so the sum leaves full scale long before
    /// anything unusual has happened. **Nothing in the original's chain
    /// reserves headroom for that**: `Audio_Init_q` (`0x089906e4`) sets all
    /// fifteen group volumes *and* the master to `0x400`, and
    /// `Audio_OutputThread_q` (`0x0898ca54`) hands `master << 5` = `0x8000` -
    /// `PSP_AUDIO_VOLUME_MAX` - to `sceAudioOutputPannedBlocking`. Unity end to
    /// end, into a 16-bit buffer that clamps. See
    /// [`audio-levels.md`](../../../../docs/ghidra/functions/psp-pulse-usa/audio-levels.md).
    ///
    /// So the clamp below is the port of a hardware behaviour, not a limiter
    /// invented to make a loud mix comfortable - there is no evidenced
    /// attenuation to apply, and picking one would be exactly the
    /// plausible-looking stand-in `CLAUDE.md` forbids. What the clamp does buy
    /// is that an `f32` device and an `i16` device now receive the same
    /// samples, where before the mixer handed the host whatever the sum
    /// reached and each backend saturated it in its own place.
    ///
    /// [`Self::clipped`] counts what it cost. **Nothing reads it yet** - it is
    /// there for tests and for the perf overlay to pick up, the way
    /// [`Self::starved`] is, so that a mix saturating for a whole race is a
    /// number somebody can ask for rather than only a sound.
    pub fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let master = self.master_gain;
        let bus_gain = self.bus_gain;
        let rate = f64::from(self.sample_rate);

        for voice in &mut self.voices {
            let Some(sound) = voice.sound.clone() else {
                continue;
            };
            let Some(bus) = voice.bus else {
                continue;
            };
            let gain = voice.gain * bus_gain[bus.index()] * master;
            // One pair for the whole buffer. The original moves a pan over a
            // ramp inside SCREAM rather than per sample, and a tick is short
            // enough that stepping it here would model an interpolator we have
            // not read.
            let channel_gain = voice.pan.map_or([1.0; CHANNELS], crate::spatial::pan_gains);
            let step = f64::from(sound.sample_rate()) / rate * f64::from(voice.pitch);
            let frames = sound.frames();

            let mut finished = false;
            // Counted down inside the loop so the fade is per output frame
            // rather than per buffer: a 512-frame buffer would otherwise take
            // the whole release in one step, which is the click this removes.
            let mut release = voice.release;
            // How many source frames before the end the closing fade starts,
            // which is `RELEASE_FRAMES` of *output* converted into the source's
            // own clock by the resampling step. See `RELEASE_FRAMES`.
            // ...and never more than a quarter of the source, so that a cue
            // shorter than the fade is shortened rather than swallowed.
            let closing = (f64::from(RELEASE_FRAMES) * step.abs()).min(frames as f64 / 4.0);
            for chunk in out.as_chunks_mut::<CHANNELS>().0 {
                if frames == 0 {
                    finished = true;
                    break;
                }
                let fade = match release {
                    None => 1.0,
                    Some(0) => {
                        finished = true;
                        break;
                    }
                    Some(left) => {
                        release = Some(left - 1);
                        left as f32 / RELEASE_FRAMES as f32
                    }
                };
                if voice.position >= frames as f64 {
                    if voice.looping {
                        // Modulo rather than reset: at a step above one frame
                        // the overshoot is real time, and dropping it makes a
                        // looped bed drift slow.
                        voice.position %= frames as f64;
                    } else {
                        finished = true;
                        break;
                    }
                }

                let index = voice.position as usize;
                let frac = (voice.position - index as f64) as f32;
                let a = sound.frame(index);
                let b = if voice.looping && index + 1 >= frames {
                    sound.frame(0)
                } else {
                    sound.frame(index + 1)
                };
                // **The closing fade, on the waveform rather than after it.** A
                // one-shot that stops mid-signal is cut at whatever value it
                // reached, and on Wipeout HD that is routinely most of full
                // scale - so the last stretch of the source is ramped down
                // instead. Ramping the *signal* rather than holding its last
                // sample and decaying that matters: a held value ramped to zero
                // is a unipolar pulse, which is itself a low-frequency thump,
                // and the longer the ramp the deeper it sits. This costs the
                // cue the level of its final few milliseconds and adds nothing.
                let closing_fade = if voice.looping {
                    1.0
                } else {
                    let left = frames as f64 - voice.position;
                    if closing > 0.0 && left < closing {
                        (left / closing).clamp(0.0, 1.0) as f32
                    } else {
                        1.0
                    }
                };
                for (c, sample) in chunk.iter_mut().enumerate() {
                    *sample += (a[c] + (b[c] - a[c]) * frac)
                        * gain
                        * channel_gain[c]
                        * fade
                        * closing_fade;
                }
                voice.position += step;
            }

            if finished {
                *voice = Voice::default();
            } else {
                voice.release = release;
            }
        }

        for sample in out.iter_mut() {
            if *sample < -1.0 || *sample > 1.0 {
                self.clipped += 1;
                *sample = sample.clamp(-1.0, 1.0);
            }
        }
    }

    /// Renders exactly one simulation tick's worth of audio, appending to
    /// `out`.
    ///
    /// The frame count comes from the tick rate alone, never a wall clock, so
    /// the offline dump and a real-time stream produce the same samples for the
    /// same control sequence. Returns how many frames were written.
    pub fn render_tick(&mut self, tick_hz: u32, out: &mut Vec<f32>) -> usize {
        let tick_hz = tick_hz.max(1);
        // Whole frames plus whatever the last ticks left over - see
        // [`Mixer::tick_frame_remainder`].
        let total = self.sample_rate + self.tick_frame_remainder;
        let frames = (total / tick_hz) as usize;
        self.tick_frame_remainder = total % tick_hz;
        let at = out.len();
        out.resize(at + frames * CHANNELS, 0.0);
        self.render(&mut out[at..]);
        frames
    }
}

#[cfg(test)]
mod tests;
