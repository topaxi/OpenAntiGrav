//! A cue played as the timeline it authors: several voices, each keyed on after
//! its own delay, rather than one waveform chosen from the set.
//!
//! Pulse's `PLASMAHITSHIP` is three waveforms keyed on together and a fourth 200
//! master ticks later; `~SHIELD` is two loops held at once; `PLASMA` is one shot
//! at +30 degrees and again at -30 fifteen ticks later, then a tail. Choosing one
//! at random is the bug the Zone announcer had. See `oag_formats::sblk::timeline`
//! and `docs/formats/psp-audio.md`.
//!
//! Voices, not a mixed buffer: the original keys each grain on its own SAS voice,
//! and grains differ in rate, looping and pitch bend. Each grain is one
//! [`CueVoice`]; [`start`] hands them to the mixer with the delay
//! [`oag_audio::Mixer::delay_start`] gives.
//!
//! Authored: which waveforms, when (master ticks at the build's
//! [`SequenceTick::ticks_per_second`]), each grain's volume terms and descriptor
//! pan angle, the alternate groups (`0x19`) and the random bend (`0x1b`) through
//! each descriptor's bend range. Chosen, not measured: a delay is rounded to a
//! whole output frame; a placed cue's emitter pan and a grain's angle combine as
//! `sin(asin(emitter pan) + angle)`, assuming an emitter in the front half; and
//! a bend draw becomes the linear-in-semitones pitch the descriptor's range
//! implies (`Scream_ComputeVoiceNote`), not a table read.

use std::collections::BTreeMap;
use std::sync::Arc;

use oag_audio::spatial::{pan_of_angle, pan_volume_gain};
use oag_audio::{Bus, Mixer, Play, Sound, VoiceId};
use oag_core::Rng;
use oag_formats::sblk::Bank;
use oag_formats::sblk::runner::Runner;
use oag_formats::sblk::timeline::WalkModel;
use oag_title::SequenceTick;

use super::banks::decode_waveform;

/// One waveform of a timeline cue.
#[derive(Debug, Clone)]
pub struct Layer {
    /// The waveform, mono at its own rate, carrying this grain's volume law.
    pub sound: Arc<Sound>,
    /// Whether the descriptor loops it.
    pub looping: bool,
    /// Seconds after the cue starts.
    pub delay: f64,
    /// Pan angle in degrees, the descriptor's plus any child records'.
    pub angle: i32,
    /// The random bend that detunes it, when one was in force.
    pub bend: Option<Bend>,
}

/// A layer's share in one `0x1b` draw.
#[derive(Debug, Clone, Copy)]
pub struct Bend {
    /// Index into [`Timeline::bends`].
    pub draw: usize,
    /// Semitones at full downward bend, the descriptor's `+0x08`.
    pub down: i8,
    /// Semitones at full upward bend, the descriptor's `+0x09`.
    pub up: i8,
}

/// One alternate combination of a cue.
#[derive(Debug, Clone)]
pub struct Timeline {
    pub layers: Vec<Layer>,
    /// Each `0x1b` execution's percentage operand.
    pub bends: Vec<i8>,
}

/// A voice ready to hand to the mixer.
#[derive(Debug, Clone)]
pub struct CueVoice {
    pub sound: Arc<Sound>,
    pub looping: bool,
    /// Seconds after the cue starts.
    pub delay: f64,
    pub angle: i32,
    /// Playback rate multiplier from the bend draw, `1.0` for none.
    pub pitch: f32,
}

impl CueVoice {
    /// A single waveform at the cue's start, unbent and centred.
    #[must_use]
    pub fn plain(sound: Arc<Sound>, looping: bool) -> Self {
        Self {
            sound,
            looping,
            delay: 0.0,
            angle: 0,
            pitch: 1.0,
        }
    }
}

impl Timeline {
    /// The voices for one play, drawing each `0x1b` from `rng`.
    ///
    /// The draw is `Scream_OpRandomBend`'s: a value in `-0x8000..0x7fff` scaled
    /// by the operand's percentage, shared by every grain under that opcode.
    pub fn voices(&self, rng: &mut Rng) -> Vec<CueVoice> {
        let draws: Vec<f32> = self
            .bends
            .iter()
            .map(|&percent| {
                let raw = (rng.below(0x7fff) as i64 * 0xffff / 0x7fff) - 0x8000;
                let bend = (raw * i64::from(percent) / 100) as i16;
                f32::from(bend)
            })
            .collect();
        self.layers
            .iter()
            .map(|layer| {
                let pitch = layer.bend.map_or(1.0, |b| {
                    let bend = draws[b.draw];
                    let semitones = if bend < 0.0 {
                        f32::from(b.down) * bend / 32768.0
                    } else {
                        f32::from(b.up) * bend / 32767.0
                    };
                    (semitones / 12.0).exp2()
                });
                CueVoice {
                    sound: Arc::clone(&layer.sound),
                    looping: layer.looping,
                    delay: layer.delay,
                    angle: layer.angle,
                    pitch,
                }
            })
            .collect()
    }
}

/// Every alternate combination of `name` as a timeline, when it is worth one.
///
/// `Ok(None)` keeps the flat pick: unknown tick, an incompletely modelled walk,
/// a grain angle in the rear half the pan law does not model, or a cue a flat
/// pick already plays (one centred grain at tick zero, no audible bend).
///
/// # Errors
///
/// A waveform the timeline reaches does not decode.
pub(super) fn timelines(
    bank: &Bank,
    name: &str,
    tick: SequenceTick,
) -> anyhow::Result<Option<Vec<Timeline>>> {
    let Some(ticks_per_second) = tick.ticks_per_second() else {
        return Ok(None);
    };
    let Some(cue) = bank.cue_named(name) else {
        return Ok(None);
    };
    let model = WalkModel {
        goto_markers: tick.follows_gotos(),
    };
    let Some(all) = bank.cue_timelines_modelled(&cue, model) else {
        return Ok(None);
    };
    if all.is_empty() || all.iter().any(|t| !t.is_complete() || t.grains.is_empty()) {
        return Ok(None);
    }
    let audible_bend = |g: &oag_formats::sblk::timeline::Grain| {
        g.bend.is_some() && (g.sound.bend_down != 0 || g.sound.bend_up != 0)
    };
    let flat = all.iter().all(|t| {
        t.grains.len() == 1
            && t.grains[0].tick == 0
            && t.grains[0].angle == 0
            && t.grains[0].scale == 1.0
            && !audible_bend(&t.grains[0])
    });
    if flat {
        return Ok(None);
    }
    if all
        .iter()
        .flat_map(|t| &t.grains)
        .any(|g| pan_of_angle(g.angle).is_none())
    {
        return Ok(None);
    }

    let mut decoder = Decoder::default();
    let mut out = Vec::with_capacity(all.len());
    for timeline in &all {
        let mut layers = Vec::with_capacity(timeline.grains.len());
        for grain in &timeline.grains {
            let Some(layer) = decoder.layer(bank, name, grain, ticks_per_second)? else {
                return Ok(None);
            };
            layers.push(layer);
        }
        out.push(Timeline {
            layers,
            bends: timeline.bends.clone(),
        });
    }
    Ok(Some(out))
}

/// Decoded waveforms shared across the grains of one cue: the samples once per
/// waveform, the [`Sound`] once per waveform and gain.
#[derive(Default)]
struct Decoder {
    pcm: BTreeMap<u32, Vec<i16>>,
    sounds: BTreeMap<(u32, u32), Arc<Sound>>,
}

impl Decoder {
    /// One grain as a [`Layer`], or `None` when its waveform decodes to nothing.
    fn layer(
        &mut self,
        bank: &Bank,
        name: &str,
        grain: &oag_formats::sblk::timeline::Grain,
        ticks_per_second: f64,
    ) -> anyhow::Result<Option<Layer>> {
        let samples = match self.pcm.get(&grain.sound.offset) {
            Some(samples) => samples,
            None => {
                let decoded = decode_waveform(bank, &grain.sound, name)?;
                self.pcm.entry(grain.sound.offset).or_insert(decoded)
            }
        };
        if samples.is_empty() {
            return Ok(None);
        }
        let gain = grain.scale * pan_volume_gain(grain.cue_volume, grain.sound.volume);
        let key = (grain.sound.offset, gain.to_bits());
        let sound = match self.sounds.get(&key) {
            Some(sound) => Arc::clone(sound),
            None => {
                let sound = Arc::new(
                    Sound::new(samples.clone(), 1, grain.sound.sample_rate())?
                        .with_pan_volume_gain(gain),
                );
                self.sounds.insert(key, Arc::clone(&sound));
                sound
            }
        };
        Ok(Some(Layer {
            sound,
            looping: grain.sound.is_looping(),
            delay: f64::from(grain.tick) / ticks_per_second,
            angle: grain.angle,
            bend: grain.bend.map(|draw| Bend {
                draw,
                down: grain.sound.bend_down,
                up: grain.sound.bend_up,
            }),
        }))
    }
}

/// A cue whose list repeats, ready to be played for as long as it is held.
///
/// The list itself is an [`oag_formats::sblk::runner::Runner`]; every waveform
/// it can key on is decoded here, once, so a tick never decodes.
#[derive(Debug, Clone)]
pub struct Program {
    pub(super) runner: Runner,
    /// The layer each key-on command starts, by its command index.
    pub(super) layers: BTreeMap<usize, Layer>,
    pub(super) ticks_per_second: f64,
}

/// `name` as a [`Program`], when the title runs on the PSP tick, the list is
/// one a runner runs and every waveform it keys on decodes and pans.
///
/// # Errors
///
/// A waveform the list reaches does not decode.
pub(super) fn program(
    bank: &Bank,
    name: &str,
    tick: SequenceTick,
) -> anyhow::Result<Option<Program>> {
    // The runner's semantics are Pulse PSP's; PS2 lends its tick, the same way
    // the timelines do. A title on another tick keeps the one-shot path.
    if !matches!(tick, SequenceTick::Psp) {
        return Ok(None);
    }
    let Some(ticks_per_second) = tick.ticks_per_second() else {
        return Ok(None);
    };
    let Some(runner) = bank.cue_named(name).and_then(|cue| bank.cue_runner(&cue)) else {
        return Ok(None);
    };
    let mut decoder = Decoder::default();
    let mut layers = BTreeMap::new();
    for grain in runner.key_ons() {
        if pan_of_angle(grain.angle).is_none() {
            return Ok(None);
        }
        let Some(layer) = decoder.layer(bank, name, grain, ticks_per_second)? else {
            return Ok(None);
        };
        layers.insert(grain.sound.command, layer);
    }
    Ok(Some(Program {
        runner,
        layers,
        ticks_per_second,
    }))
}

/// Where the voices of one play go: gain and pan from the cue's placement.
#[derive(Debug, Clone, Copy)]
pub struct Where {
    /// The placement's gain, `1.0` for a dry cue.
    pub gain: f32,
    /// The emitter's pan, or [`None`] for a dry cue.
    pub pan: Option<f32>,
}

impl Where {
    /// Unplaced and at full volume.
    pub const DRY: Self = Self {
        gain: 1.0,
        pan: None,
    };
}

/// The pan a voice at `angle` takes under `at`.
///
/// A dry cue with all voices centred keeps no pan, as every single-waveform cue
/// did. Once any voice is off-centre every voice takes its authored angle, so a
/// centred grain is not 3 dB louder beside a panned one. A placed cue shifts the
/// emitter's pan by the angle.
fn pan_for(at: Where, angle: i32, any_off_centre: bool) -> Option<f32> {
    match at.pan {
        Some(pan) if angle == 0 => Some(pan),
        Some(pan) => {
            let emitter = pan.clamp(-1.0, 1.0).asin().to_degrees().round() as i32;
            pan_of_angle(emitter + angle).or(Some(pan))
        }
        None if any_off_centre => pan_of_angle(angle),
        None => None,
    }
}

/// Starts every voice of a play on `mixer`, returning the handles.
///
/// A voice the pool refuses is simply absent from the result (the mixer counts
/// it as starved).
pub fn start(mixer: &mut Mixer, voices: &[CueVoice], bus: Bus, at: Where) -> Vec<VoiceId> {
    let any_off_centre = voices.iter().any(|v| v.angle != 0);
    let mut ids = Vec::with_capacity(voices.len());
    for voice in voices {
        let base = if voice.looping {
            Play::looping(Arc::clone(&voice.sound), bus)
        } else {
            Play::once(Arc::clone(&voice.sound), bus)
        };
        let Some(id) = mixer.play(Play {
            gain: at.gain,
            pan: pan_for(at, voice.angle, any_off_centre),
            pitch: voice.pitch,
            ..base
        }) else {
            continue;
        };
        if voice.delay > 0.0 {
            mixer.delay_start(id, voice.delay);
        }
        ids.push(id);
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sound() -> Arc<Sound> {
        Arc::new(Sound::new(vec![1000; 8], 1, 8).unwrap())
    }

    fn layer(delay: f64, angle: i32, bend: Option<Bend>) -> Layer {
        Layer {
            sound: sound(),
            looping: false,
            delay,
            angle,
            bend,
        }
    }

    #[test]
    fn a_bend_draw_is_shared_and_stays_inside_the_descriptor_range() {
        let timeline = Timeline {
            layers: vec![
                layer(
                    0.0,
                    0,
                    Some(Bend {
                        draw: 0,
                        down: 2,
                        up: 2,
                    }),
                ),
                layer(
                    0.5,
                    0,
                    Some(Bend {
                        draw: 0,
                        down: 0,
                        up: 0,
                    }),
                ),
                layer(1.0, 0, None),
            ],
            bends: vec![100],
        };
        let mut rng = Rng::new(5);
        let mut lowest = f32::MAX;
        let mut highest = f32::MIN;
        for _ in 0..500 {
            let voices = timeline.voices(&mut rng);
            assert_eq!(voices[1].pitch, 1.0, "a zero range cannot detune");
            assert_eq!(voices[2].pitch, 1.0, "no bend, no detune");
            lowest = lowest.min(voices[0].pitch);
            highest = highest.max(voices[0].pitch);
        }
        // Two semitones either way is a factor of 2^(2/12) = 1.1225.
        assert!((0.890..0.93).contains(&lowest), "{lowest}");
        assert!(highest <= 1.1225 && highest > 1.08, "{highest}");
    }

    #[test]
    fn a_dry_cue_pans_only_once_a_voice_is_off_centre() {
        assert_eq!(pan_for(Where::DRY, 0, false), None);
        assert_eq!(pan_for(Where::DRY, 0, true), Some(0.0));
        let right = pan_for(Where::DRY, 30, true).unwrap();
        let left = pan_for(Where::DRY, 330, true).unwrap();
        assert!((right - 0.5).abs() < 1e-6 && (left + 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_placed_cue_keeps_its_emitters_pan_for_a_centred_voice_and_shifts_it_otherwise() {
        let at = Where {
            gain: 0.5,
            pan: Some(0.5),
        };
        assert_eq!(pan_for(at, 0, false), Some(0.5));
        // 30 degrees from an emitter at 30 degrees is 60.
        let shifted = pan_for(at, 30, true).unwrap();
        assert!(
            (shifted - 60f32.to_radians().sin()).abs() < 1e-3,
            "{shifted}"
        );
    }

    #[test]
    fn started_voices_hold_silent_for_their_delay() {
        let voices = vec![
            CueVoice::plain(sound(), false),
            CueVoice {
                delay: 0.5,
                ..CueVoice::plain(sound(), false)
            },
        ];
        let mut mixer = Mixer::new(8);
        let ids = start(&mut mixer, &voices, Bus::Sfx, Where::DRY);
        assert_eq!(ids.len(), 2);
        let mut out = vec![0.0; 6 * 2];
        mixer.render(&mut out);
        let left: Vec<f32> = out.chunks(2).map(|f| f[0]).collect();
        // Sound is 8 frames at 8 Hz: the first voice plays all six frames; the
        // second joins at frame 4.
        assert!(left[..4].windows(2).all(|w| w[0] == w[1]), "{left:?}");
        assert!(left[4] > left[3] * 1.9, "{left:?}");
    }
}
