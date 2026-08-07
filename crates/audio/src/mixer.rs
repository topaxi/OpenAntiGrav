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

/// A mix bus, one per volume the original's options menu exposes.
///
/// The executable has two and only two: `"Music Volume"` at `0x08a78658` and
/// `"SFX Volume"` at `0x08a78668`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bus {
    /// Streamed music.
    Music,
    /// Everything else: effects, speech, engine.
    Sfx,
}

impl Bus {
    /// Number of buses, for array sizing.
    pub const COUNT: usize = 2;

    /// This bus's index into a `[_; Bus::COUNT]`.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Music => 0,
            Self::Sfx => 1,
        }
    }
}

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
    looping: bool,
    generation: u32,
}

impl Voice {
    fn is_free(&self) -> bool {
        self.sound.is_none()
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

    /// How many voices are sounding.
    #[must_use]
    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| !v.is_free()).count()
    }

    /// How many starts have been refused for want of a free slot.
    #[must_use]
    pub fn starved(&self) -> u64 {
        self.starved
    }

    /// Starts a voice, returning a handle to steer it with.
    ///
    /// Returns `None` when every slot is busy. Refusing rather than stealing is
    /// deliberate for now: stealing needs a priority the original keeps in the
    /// bank's undecoded cue table, and inventing one would be a guess dressed
    /// as behaviour.
    pub fn play(&mut self, play: Play) -> Option<VoiceId> {
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
            pitch: play.pitch.max(0.0),
            gain: play.gain.max(0.0),
            looping: play.looping,
            generation,
        };
        Some(VoiceId {
            slot: slot as u16,
            generation,
        })
    }

    /// Stops a voice and frees its slot. A stale handle does nothing.
    pub fn stop(&mut self, id: VoiceId) {
        if let Some(voice) = self.voice_mut(id) {
            *voice = Voice::default();
        }
    }

    /// Stops everything on one bus, which is how a race teardown silences its
    /// engines without touching the music.
    pub fn stop_bus(&mut self, bus: Bus) {
        for voice in &mut self.voices {
            if voice.bus == Some(bus) {
                *voice = Voice::default();
            }
        }
    }

    /// Stops every voice.
    pub fn stop_all(&mut self) {
        for voice in &mut self.voices {
            *voice = Voice::default();
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

    /// Whether the handle still addresses a sounding voice.
    #[must_use]
    pub fn is_playing(&self, id: VoiceId) -> bool {
        self.voice(id).is_some()
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
            let step = f64::from(sound.sample_rate()) / rate * f64::from(voice.pitch);
            let frames = sound.frames();

            let mut finished = false;
            for chunk in out.chunks_exact_mut(CHANNELS) {
                if frames == 0 {
                    finished = true;
                    break;
                }
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
                for (c, sample) in chunk.iter_mut().enumerate() {
                    *sample += (a[c] + (b[c] - a[c]) * frac) * gain;
                }
                voice.position += step;
            }

            if finished {
                *voice = Voice::default();
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
        let frames = (self.sample_rate / tick_hz.max(1)) as usize;
        let at = out.len();
        out.resize(at + frames * CHANNELS, 0.0);
        self.render(&mut out[at..]);
        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(frames: usize, channels: u16, rate: u32) -> Arc<Sound> {
        let samples = (0..frames * channels as usize)
            .map(|i| i16::try_from(i % 1000).unwrap_or(0))
            .collect();
        Arc::new(Sound::new(samples, channels, rate).expect("sound"))
    }

    #[test]
    fn a_sound_needs_whole_frames() {
        assert!(Sound::new(vec![0; 3], 2, 44_100).is_err());
        assert!(Sound::new(vec![0; 4], 2, 44_100).is_ok());
        assert!(Sound::new(vec![0; 4], 0, 44_100).is_err());
        assert!(Sound::new(vec![0; 4], 2, 0).is_err());
    }

    #[test]
    fn rendering_with_nothing_playing_writes_silence_not_a_short_buffer() {
        let mut mixer = Mixer::new(44_100);
        let mut out = vec![1.0f32; 64];
        mixer.render(&mut out);
        assert_eq!(out.len(), 64, "render must fill the whole buffer");
        assert!(out.iter().all(|s| *s == 0.0), "expected silence");
    }

    #[test]
    fn a_one_shot_frees_its_slot_when_it_ends() {
        let mut mixer = Mixer::new(44_100);
        let id = mixer
            .play(Play::once(tone(4, 2, 44_100), Bus::Sfx))
            .expect("a free slot");
        assert_eq!(mixer.active_voices(), 1);

        let mut out = vec![0.0f32; 64 * CHANNELS];
        mixer.render(&mut out);

        assert_eq!(mixer.active_voices(), 0, "the voice should have finished");
        assert!(!mixer.is_playing(id), "the handle should be stale");
    }

    #[test]
    fn a_looping_voice_keeps_going_past_its_end() {
        let mut mixer = Mixer::new(44_100);
        let id = mixer
            .play(Play::looping(tone(4, 2, 44_100), Bus::Sfx))
            .expect("a free slot");
        let mut out = vec![0.0f32; 1024 * CHANNELS];
        mixer.render(&mut out);
        assert!(mixer.is_playing(id), "a looping voice must not free itself");
    }

    #[test]
    fn a_stale_handle_cannot_steer_the_voice_that_took_its_slot() {
        let mut mixer = Mixer::new(44_100);
        let old = mixer
            .play(Play::once(tone(2, 2, 44_100), Bus::Sfx))
            .expect("a free slot");
        // Run it out so the slot is recycled.
        let mut out = vec![0.0f32; 64 * CHANNELS];
        mixer.render(&mut out);
        let new = mixer
            .play(Play::looping(tone(64, 2, 44_100), Bus::Sfx))
            .expect("a free slot");

        assert!(!mixer.is_playing(old));
        mixer.stop(old);
        assert!(
            mixer.is_playing(new),
            "stopping a stale handle must not stop whoever took the slot"
        );
    }

    #[test]
    fn the_pool_refuses_rather_than_steals_when_it_is_full() {
        let mut mixer = Mixer::new(44_100);
        for _ in 0..MAX_VOICES {
            assert!(
                mixer
                    .play(Play::looping(tone(64, 2, 44_100), Bus::Sfx))
                    .is_some()
            );
        }
        assert_eq!(mixer.active_voices(), MAX_VOICES);
        assert!(
            mixer
                .play(Play::once(tone(4, 2, 44_100), Bus::Sfx))
                .is_none()
        );
        assert_eq!(mixer.starved(), 1);
        assert_eq!(
            mixer.active_voices(),
            MAX_VOICES,
            "a refused start must not have displaced anyone"
        );
    }

    #[test]
    fn a_bus_gain_only_moves_its_own_bus() {
        let sound = tone(1024, 2, 44_100);
        let mut mixer = Mixer::new(44_100);
        mixer.play(Play::looping(sound.clone(), Bus::Music));
        mixer.set_bus_gain(Bus::Sfx, 0.0);
        let mut out = vec![0.0f32; 128 * CHANNELS];
        mixer.render(&mut out);
        let music_energy: f32 = out.iter().map(|s| s.abs()).sum();
        assert!(music_energy > 0.0, "silencing SFX must not silence music");

        mixer.stop_all();
        mixer.play(Play::looping(sound, Bus::Sfx));
        mixer.render(&mut out);
        let sfx_energy: f32 = out.iter().map(|s| s.abs()).sum();
        assert_eq!(sfx_energy, 0.0, "the SFX bus was set to zero");
    }

    #[test]
    fn stopping_a_bus_leaves_the_other_one_sounding() {
        let sound = tone(1024, 2, 44_100);
        let mut mixer = Mixer::new(44_100);
        let music = mixer
            .play(Play::looping(sound.clone(), Bus::Music))
            .expect("slot");
        let sfx = mixer.play(Play::looping(sound, Bus::Sfx)).expect("slot");
        mixer.stop_bus(Bus::Sfx);
        assert!(mixer.is_playing(music));
        assert!(!mixer.is_playing(sfx));
    }

    #[test]
    fn a_tick_renders_the_same_frame_count_every_time() {
        let mut mixer = Mixer::new(48_000);
        let mut out = Vec::new();
        for _ in 0..60 {
            assert_eq!(mixer.render_tick(60, &mut out), 800);
        }
        assert_eq!(
            out.len(),
            48_000 * CHANNELS,
            "sixty ticks at 60 Hz is one second of audio"
        );
    }

    #[test]
    fn the_same_control_sequence_renders_the_same_samples_twice() {
        let render = || {
            let mut mixer = Mixer::new(44_100);
            mixer.play(Play::looping(tone(777, 1, 48_000), Bus::Music));
            let mut out = Vec::new();
            for tick in 0..30 {
                if tick == 10 {
                    mixer.play(Play::once(tone(300, 2, 22_050), Bus::Sfx));
                }
                mixer.render_tick(60, &mut out);
            }
            out
        };
        assert_eq!(render(), render(), "the mixer must not read a clock");
    }

    #[test]
    fn a_mono_source_arrives_on_both_channels() {
        let mut mixer = Mixer::new(44_100);
        mixer.play(Play::looping(tone(512, 1, 44_100), Bus::Sfx));
        let mut out = vec![0.0f32; 64 * CHANNELS];
        mixer.render(&mut out);
        for pair in out.chunks_exact(CHANNELS) {
            assert_eq!(pair[0], pair[1], "mono must duplicate, not pan");
        }
    }

    #[test]
    fn resampling_a_faster_source_consumes_it_faster() {
        // 88.2 kHz into a 44.1 kHz mixer is two source frames per output
        // frame, so a 200-frame sound lasts 100 output frames.
        let mut mixer = Mixer::new(44_100);
        let id = mixer
            .play(Play::once(tone(200, 2, 88_200), Bus::Sfx))
            .expect("slot");
        let mut out = vec![0.0f32; 99 * CHANNELS];
        mixer.render(&mut out);
        assert!(mixer.is_playing(id), "99 frames in, it should still sound");
        mixer.render(&mut out);
        assert!(!mixer.is_playing(id), "198 frames in, it should be done");
    }
}
