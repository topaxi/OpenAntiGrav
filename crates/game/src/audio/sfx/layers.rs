//! A cue played as the timeline it authors: several voices, each keyed on
//! after its own delay, rather than one waveform chosen from the set.
//!
//! Pulse's `PLASMAHITSHIP` is three waveforms keyed on together and a fourth
//! 200 master ticks later; `~SHIELD` is two loops held at once; `PLASMA` is one
//! shot fired at +30 degrees and again at -30 fifteen ticks later, then a tail.
//! Choosing one of them at random is the bug the Zone announcer had. See
//! `oag_formats::sblk::timeline` for what the command list does and
//! `docs/formats/psp-audio.md` for the evidence.
//!
//! **Voices, not a mixed buffer.** The original keys each grain on its own SAS
//! voice, and a cue's grains differ in sample rate, in whether they loop and in
//! how far they are pitch-bent, so a single pre-mixed buffer cannot hold them.
//! Each grain is one [`CueVoice`]; [`start`] hands them to the mixer with the
//! start delay [`oag_audio::Mixer::delay_start`] gives.
//!
//! What is authored: which waveforms, when (in master ticks at the build's
//! [`SequenceTick::ticks_per_second`]), each grain's volume
//! terms, its descriptor pan angle, the alternate groups (`0x19`) and the
//! random bend (`0x1b`) through each descriptor's bend range. What is chosen,
//! not measured: a delay is rounded to a whole output frame; a **placed** cue's
//! emitter pan and a grain's authored angle are combined as
//! `sin(asin(emitter pan) + angle)`, which assumes an emitter in the front half;
//! and the pitch a bend draw becomes is the linear-in-semitones law the
//! descriptor's range implies (`Scream_ComputeVoiceNote`), not a table read.

use std::collections::BTreeMap;
use std::sync::Arc;

use oag_audio::spatial::{pan_of_angle, pan_volume_gain};
use oag_audio::{Bus, Mixer, Play, Sound, VoiceId};
use oag_core::Rng;
use oag_formats::sblk::Bank;
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
/// `Ok(None)` keeps the flat pick: the title's tick is unknown, the walk does
/// not model the cue completely, a grain's angle is in the rear half the pan law
/// does not model, or the cue is what a flat pick already plays (one centred
/// grain at tick zero, no audible bend).
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

    let mut pcm: BTreeMap<u32, Vec<i16>> = BTreeMap::new();
    let mut sounds: BTreeMap<(u32, u32), Arc<Sound>> = BTreeMap::new();
    let mut out = Vec::with_capacity(all.len());
    for timeline in &all {
        let mut layers = Vec::with_capacity(timeline.grains.len());
        for grain in &timeline.grains {
            let samples = match pcm.get(&grain.sound.offset) {
                Some(samples) => samples,
                None => {
                    let decoded = decode_waveform(bank, &grain.sound, name)?;
                    pcm.entry(grain.sound.offset).or_insert(decoded)
                }
            };
            if samples.is_empty() {
                return Ok(None);
            }
            let gain = grain.scale * pan_volume_gain(grain.cue_volume, grain.sound.volume);
            let key = (grain.sound.offset, gain.to_bits());
            let sound = match sounds.get(&key) {
                Some(sound) => Arc::clone(sound),
                None => {
                    let sound = Arc::new(
                        Sound::new(samples.clone(), 1, grain.sound.sample_rate())?
                            .with_pan_volume_gain(gain),
                    );
                    sounds.insert(key, Arc::clone(&sound));
                    sound
                }
            };
            layers.push(Layer {
                sound,
                looping: grain.sound.is_looping(),
                delay: f64::from(grain.tick) / ticks_per_second,
                angle: grain.angle,
                bend: grain.bend.map(|draw| Bend {
                    draw,
                    down: grain.sound.bend_down,
                    up: grain.sound.bend_up,
                }),
            });
        }
        out.push(Timeline {
            layers,
            bends: timeline.bends.clone(),
        });
    }
    Ok(Some(out))
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
/// A dry cue whose voices are all centred keeps no pan at all, as every
/// single-waveform cue always played. Once any voice of the cue is off-centre
/// every voice takes its authored angle, so a centred grain does not sit 3 dB
/// louder beside a panned one. A placed cue shifts the emitter's own pan by the
/// angle.
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
