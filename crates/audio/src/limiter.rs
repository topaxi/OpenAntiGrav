//! The master limiter: a stereo-linked look-ahead peak limiter that sits after
//! the mixer's sum.
//!
//! The sum of a busy race (eight engines, weapons, music, every volume at 100)
//! leaves full scale, and a hard clamp there is audible as crackle. This
//! delays the signal by a couple of milliseconds, finds the gain that keeps
//! the coming peak under [`CEILING`], and ramps to it so that the peak is
//! reached at full attenuation without a step. Gain returns slowly, so a
//! sustained loud passage does not pump.
//!
//! Allocation-free after construction: the delay line and both gain rings are
//! sized once from the sample rate.

use crate::mixer::CHANNELS;

/// The loudest sample the limiter lets through. A little under full scale, so
/// that a later 16-bit conversion or a device's own rounding cannot clip.
pub const CEILING: f32 = 0.95;

/// How far ahead of the output the limiter looks, in seconds. The gain has
/// reached its target by the time the peak leaves the delay line.
pub const LOOKAHEAD_SECONDS: f32 = 0.002;

/// How long the gain stays down after the signal stops demanding it, in
/// seconds. Longer than a period of the lowest engine note, so the gain does
/// not ride the waveform.
pub const HOLD_SECONDS: f32 = 0.025;

/// Time constant of the gain's return to unity, in seconds.
pub const RELEASE_SECONDS: f32 = 0.12;

/// Stereo-linked look-ahead peak limiter.
#[derive(Debug, Clone)]
pub struct Limiter {
    len: usize,
    delay: Vec<[f32; CHANNELS]>,
    required: Vec<f32>,
    floor: Vec<f32>,
    pos: usize,
    hot: usize,
    floor_hot: usize,
    average: f64,
    hold_len: u32,
    hold: u32,
    released: f32,
    release_coeff: f32,
    limited: u64,
    deepest: f32,
    peak: f32,
}

impl Limiter {
    /// A limiter for `sample_rate`, with unity gain and an empty delay line.
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        let len = ((sample_rate as f32 * LOOKAHEAD_SECONDS) as usize).max(2);
        Self {
            len,
            delay: vec![[0.0; CHANNELS]; len],
            required: vec![1.0; len],
            floor: vec![1.0; len],
            pos: 0,
            hot: 0,
            floor_hot: 0,
            average: len as f64,
            hold_len: (sample_rate as f32 * HOLD_SECONDS) as u32,
            hold: 0,
            released: 1.0,
            release_coeff: 1.0 - (-1.0 / (RELEASE_SECONDS * sample_rate as f32)).exp(),
            limited: 0,
            deepest: 1.0,
            peak: 0.0,
        }
    }

    /// Frames the output trails the input by.
    #[must_use]
    pub fn latency(&self) -> usize {
        self.len - 1
    }

    /// Frames whose gain was below unity.
    #[must_use]
    pub fn limited(&self) -> u64 {
        self.limited
    }

    /// The lowest gain applied so far (1.0 is never touched).
    #[must_use]
    pub fn deepest(&self) -> f32 {
        self.deepest
    }

    /// The largest absolute input sample seen.
    #[must_use]
    pub fn peak(&self) -> f32 {
        self.peak
    }

    /// Limits interleaved stereo `samples` in place.
    pub fn process(&mut self, samples: &mut [f32]) {
        for frame in samples.chunks_exact_mut(CHANNELS) {
            let mut incoming = [0.0; CHANNELS];
            incoming.copy_from_slice(frame);
            let loudest = incoming.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
            self.peak = self.peak.max(loudest);
            let need = if loudest > CEILING {
                CEILING / loudest
            } else {
                1.0
            };

            let slot = self.pos;
            self.hot -= usize::from(self.required[slot] < 1.0);
            self.hot += usize::from(need < 1.0);
            self.required[slot] = need;

            let quiet = self.hot == 0 && self.floor_hot == 0;
            let target = if quiet {
                self.average = self.len as f64;
                self.floor[slot] = 1.0;
                1.0
            } else {
                let lowest = self.required.iter().fold(1.0_f32, |m, g| m.min(*g));
                self.floor_hot -= usize::from(self.floor[slot] < 1.0);
                self.floor_hot += usize::from(lowest < 1.0);
                self.average += f64::from(lowest) - f64::from(self.floor[slot]);
                self.floor[slot] = lowest;
                if self.floor_hot == 0 && self.hot == 0 {
                    self.average = self.len as f64;
                }
                (self.average / self.len as f64) as f32
            };

            if target <= self.released {
                self.released = target;
                self.hold = self.hold_len;
            } else if self.hold > 0 {
                self.hold -= 1;
            } else {
                self.released =
                    target.min(self.released + (target - self.released) * self.release_coeff);
            }
            if self.released > 0.9999 && target >= 1.0 {
                self.released = 1.0;
            }
            let gain = self.released;
            if gain < 1.0 {
                self.limited += 1;
                self.deepest = self.deepest.min(gain);
            }

            // One frame less than the gain rings: the average then covers the
            // frame leaving the delay line in every one of its terms.
            let next = if slot + 1 == self.len { 0 } else { slot + 1 };
            let delayed = self.delay[next];
            self.delay[slot] = incoming;
            for (out, sample) in frame.iter_mut().zip(delayed) {
                *out = sample * gain;
            }
            self.pos = next;
        }
    }
}

#[cfg(test)]
mod tests;
