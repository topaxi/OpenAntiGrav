//! A cue that is a sequence, laid out once at load as the one line it plays.
//!
//! Pulse's Zone milestones (`zone_5` .. `zone_100`) are not takes of one word:
//! each is a timeline (the child `ZONE`, the number, the child `CLEAR`) with
//! authored delays, every word keyed on twice at pan angles 30 and 330 degrees
//! (`oag_formats::sblk::timeline`, `docs/formats/psp-audio.md`). One waveform
//! chosen at random is what a player heard as "clear", "zone" or "10" alone.
//!
//! Composing it into one stereo [`Sound`] keeps one voice on `Bus::Speech` per
//! announcement, places every grain at a sample rather than a race tick, and
//! leaves the mixer untouched.
//!
//! Authored: which waveform, when (master ticks at the build's
//! [`SequenceTick::ticks_per_second`]), pan angle, volume terms. Chosen: a grain
//! lands on the nearest sample, and the master tick's phase against the cue's
//! start is taken as zero (the original starts a cue between ticks, up to one
//! tick, 3.9 ms, either way).

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::sync::Arc;

use oag_audio::Sound;
use oag_audio::spatial::{pan_gains, pan_of_angle, pan_volume_gain};
use oag_formats::sblk::Bank;
use oag_formats::sblk::timeline::WalkModel;
use oag_title::SequenceTick;

use super::banks::decode_waveform;

/// The finished line and what it was made of, for the load report.
#[derive(Debug, Clone)]
pub(super) struct Sequence {
    pub sound: Arc<Sound>,
    pub grains: usize,
}

/// Lays `name`'s timeline out as one stereo sound, when it is a real sequence.
///
/// `Ok(None)` keeps the caller's flat pick: the cue is not a complete timeline
/// (an unmodelled opcode, an unresolved child), the tick is
/// [unknown](SequenceTick::Unknown) (2048, Pure), the cue is a single grain
/// (Pure's announcer lines, which keep their level), a grain loops, its angle is
/// in the rear half the pan law does not model, or its rate differs.
///
/// # Errors
///
/// A waveform the timeline reaches does not decode.
pub(super) fn compose_sequence(
    bank: &Bank,
    name: &str,
    tick: SequenceTick,
) -> anyhow::Result<Option<Sequence>> {
    let Some(cue) = bank.cue_named(name) else {
        return Ok(None);
    };
    let Some(ticks_per_second) = tick.ticks_per_second() else {
        return Ok(None);
    };
    let tick_seconds = 1.0 / ticks_per_second;
    let model = WalkModel {
        goto_markers: tick.follows_gotos(),
    };
    let timeline = bank.cue_timeline_modelled(&cue, &[], model);
    if !timeline.is_complete() || timeline.grains.is_empty() {
        return Ok(None);
    }
    // One grain is the whole line (Pure's announcer lines): a flat pick plays it
    // at its level. Two grains of one word are not: HD's `zone_N` is the number at
    // +30 degrees and again at -30 five ticks later, which a flat pick collapses.
    if timeline.grains.len() < 2 {
        return Ok(None);
    }
    let rate = timeline.grains[0].sound.sample_rate();
    let mut pans = Vec::with_capacity(timeline.grains.len());
    for grain in &timeline.grains {
        let Some(pan) = pan_of_angle(grain.angle) else {
            return Ok(None);
        };
        if grain.sound.is_looping() || grain.sound.sample_rate() != rate {
            return Ok(None);
        }
        pans.push(pan);
    }

    let mut decoded: BTreeMap<u32, Vec<i16>> = BTreeMap::new();
    for grain in &timeline.grains {
        if let Entry::Vacant(slot) = decoded.entry(grain.sound.offset) {
            slot.insert(decode_waveform(bank, &grain.sound, name)?);
        }
    }

    // Each grain's volume law, `Scream_PanVolumePair`'s `2 * cue^2 * sound^2`
    // behind the handler's scale. The composite carries the largest as its
    // `pan_volume_gain` and every grain the ratio, so no loud grain rounds into
    // `i16` range before the mixer's gain lands.
    let gains: Vec<f32> = timeline
        .grains
        .iter()
        .map(|g| g.scale * pan_volume_gain(g.cue_volume, g.sound.volume))
        .collect();
    let reference = gains.iter().copied().fold(0.0_f32, f32::max);
    if reference <= 0.0 {
        return Ok(None);
    }

    let start = |tick: u32| (f64::from(tick) * tick_seconds * f64::from(rate)).round() as usize;
    let items: Vec<Item> = timeline
        .grains
        .iter()
        .zip(&pans)
        .zip(&gains)
        .map(|((grain, &pan), &gain)| Item {
            start: start(grain.tick),
            pcm: &decoded[&grain.sound.offset],
            pan,
            gain,
        })
        .collect();
    let (samples, gain) = lay_out(&items, reference);
    let sound = Sound::new(samples, 2, rate)?.with_pan_volume_gain(gain);
    Ok(Some(Sequence {
        sound: Arc::new(sound),
        grains: timeline.grains.len(),
    }))
}

/// One grain, ready to lay down.
struct Item<'a> {
    /// First output frame.
    start: usize,
    pcm: &'a [i16],
    /// [`Play::pan`](oag_audio::Play::pan) position.
    pan: f32,
    /// The grain's own volume law.
    gain: f32,
}

/// Sums the grains into one interleaved stereo buffer, returning it with the
/// `pan_volume_gain` the sound should carry.
///
/// Each grain is scaled by `gain / reference` and the equal-power pan pair. When
/// the sum passes 16-bit full scale the buffer is scaled down and the gain up by
/// the same factor: headroom, not a clamp, so `gain * sample` is unchanged.
fn lay_out(items: &[Item], reference: f32) -> (Vec<i16>, f32) {
    let frames = items
        .iter()
        .map(|item| item.start + item.pcm.len())
        .max()
        .unwrap_or(0);
    let mut mix = vec![0.0_f32; frames * 2];
    for item in items {
        let [left, right] = pan_gains(item.pan);
        let relative = item.gain / reference;
        for (i, &sample) in item.pcm.iter().enumerate() {
            let x = f32::from(sample) * relative;
            mix[(item.start + i) * 2] += x * left;
            mix[(item.start + i) * 2 + 1] += x * right;
        }
    }
    let peak = mix.iter().fold(0.0_f32, |p, x| p.max(x.abs()));
    let norm = if peak > 32767.0 { 32767.0 / peak } else { 1.0 };
    let samples = mix.iter().map(|&x| (x * norm).round() as i16).collect();
    (samples, reference / norm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grain_lands_on_its_own_frame_at_its_own_pan() {
        let items = [Item {
            start: 3,
            pcm: &[1000, -1000],
            pan: 0.5,
            gain: 2.0,
        }];
        let (samples, gain) = lay_out(&items, 2.0);
        assert_eq!(gain, 2.0);
        assert_eq!(
            samples.len(),
            5 * 2,
            "three frames of lead-in, two of sound"
        );
        assert!(samples[..6].iter().all(|&s| s == 0));
        // Pan 0.5 is left 0.5, right 0.866: an equal-power pair.
        assert_eq!(samples[6], 500);
        assert_eq!(samples[7], 866);
        assert_eq!(samples[8], -500);
        assert_eq!(samples[9], -866);
    }

    #[test]
    fn a_quieter_grain_is_scaled_by_its_ratio_to_the_reference() {
        let items = [Item {
            start: 0,
            pcm: &[1000],
            pan: 0.0,
            gain: 1.0,
        }];
        let (samples, gain) = lay_out(&items, 2.0);
        assert_eq!(gain, 2.0);
        // Half of 1000 at the centred 0.7071 on each side.
        assert_eq!(samples, vec![354, 354]);
    }

    #[test]
    fn overlapping_grains_add() {
        let items = [
            Item {
                start: 0,
                pcm: &[100, 100],
                pan: 0.0,
                gain: 1.0,
            },
            Item {
                start: 1,
                pcm: &[100],
                pan: 0.0,
                gain: 1.0,
            },
        ];
        let (samples, _) = lay_out(&items, 1.0);
        assert_eq!(samples, vec![71, 71, 141, 141]);
    }

    #[test]
    fn headroom_is_taken_from_the_buffer_and_given_back_to_the_gain() {
        let items = [
            Item {
                start: 0,
                pcm: &[i16::MAX],
                pan: 0.0,
                gain: 1.0,
            },
            Item {
                start: 0,
                pcm: &[i16::MAX],
                pan: 0.0,
                gain: 1.0,
            },
        ];
        let (samples, gain) = lay_out(&items, 1.0);
        assert_eq!(samples[0], samples[1]);
        assert!(
            gain > 1.0,
            "the mixer's gain must make up what the buffer lost"
        );
        // gain * sample recovers the unclipped sum: 2 * 32767 * 0.7071.
        let recovered = f32::from(samples[0]) * gain;
        assert!(
            (recovered - 2.0 * 32767.0 * std::f32::consts::FRAC_1_SQRT_2).abs() < 2.0,
            "{recovered}"
        );
    }
}
