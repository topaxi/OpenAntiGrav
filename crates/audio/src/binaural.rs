//! The `headphones` spatial mode: a head-related transfer function per voice.
//!
//! One [`Binaural`] holds the HRIR sphere and the FFT scratch and is shared by
//! every voice of a mixer; each headphone voice owns a [`VoiceState`] with its
//! input block, its output queue and the convolution history. The convolver works
//! in whole blocks, so a voice feeds it one sample per output frame and reads
//! one back from a queue primed with [`LATENCY`] frames of silence. That keeps
//! the result a function of the voice's own frame count, never of how the
//! caller chunks `Mixer::render`.
//!
//! The impulse responses are SADIE II subject D1 (`assets/audio/hrtf/README.md`).
//! Block length, step count and the horizontal-plane direction are **chosen,
//! not measured**; the original has nothing like this.

use std::collections::VecDeque;

use hrtf::{HrirSphere, HrtfContext, HrtfProcessor, Vec3};

const HRIR: &[u8] = include_bytes!("../../../assets/audio/hrtf/sadie2-d1-48k-200.hrir");

/// Samples per convolution block, across which the convolver crossfades from the
/// previous direction to the new one. A power of two plus one is the length its FFT
/// plans fastest: 1.29 ms of CPU per second of one voice against 3.35 ms at 512.
const BLOCK: usize = 513;
/// Frames a headphone voice is behind a plain one.
pub const LATENCY: usize = BLOCK;

/// The shared convolver.
#[derive(Debug, Clone)]
pub struct Binaural {
    processor: HrtfProcessor,
}

impl Binaural {
    /// Builds the convolver for a mixer rendering at `sample_rate`. Decodes and
    /// transforms the whole sphere, so call it off the audio thread.
    ///
    /// # Errors
    ///
    /// If the shipped sphere does not parse, which a test rules out.
    pub fn new(sample_rate: u32) -> anyhow::Result<Self> {
        let sphere = HrirSphere::new(HRIR, sample_rate)
            .map_err(|e| anyhow::anyhow!("the shipped HRIR sphere did not parse: {e:?}"))?;
        Ok(Self {
            processor: HrtfProcessor::new(sphere, BLOCK),
        })
    }
}

/// One voice's place in the convolver.
#[derive(Debug, Clone)]
pub(crate) struct VoiceState {
    input: Vec<f32>,
    output: VecDeque<(f32, f32)>,
    history: Vec<f32>,
    previous: Vec3,
    /// Frames of silence still to push once the source has ended, so that its
    /// last samples come out of the queue.
    drain: usize,
}

impl VoiceState {
    pub(crate) fn new() -> Self {
        Self {
            input: Vec::with_capacity(LATENCY),
            output: std::iter::repeat_n((0.0, 0.0), LATENCY).collect(),
            history: Vec::new(),
            previous: Vec3::new(0.0, 0.0, 1.0),
            drain: LATENCY,
        }
    }

    /// Takes one mono input frame, returns the ear pair [`LATENCY`] frames old.
    /// `side` and `front` are the voice's pan and forward component.
    pub(crate) fn push(
        &mut self,
        binaural: &mut Binaural,
        mono: f32,
        side: f32,
        front: f32,
    ) -> (f32, f32) {
        self.input.push(mono);
        if self.input.len() == LATENCY {
            let (x, z) = if side == 0.0 && front == 0.0 {
                (0.0, 1.0)
            } else {
                let norm = side.hypot(front);
                (side / norm, front / norm)
            };
            let now = Vec3::new(x, 0.0, z);
            let mut block = vec![(0.0, 0.0); LATENCY];
            binaural.processor.process_samples(HrtfContext {
                source: &self.input,
                output: &mut block,
                new_sample_vector: now,
                prev_sample_vector: self.previous,
                new_gain: 1.0,
                prev_gain: 1.0,
                history: &mut self.history,
            });
            self.previous = now;
            self.input.clear();
            self.output.extend(block);
        }
        self.output.pop_front().unwrap_or((0.0, 0.0))
    }

    /// One frame of the tail after the source ended: `None` once it has
    /// drained.
    pub(crate) fn flush(
        &mut self,
        binaural: &mut Binaural,
        side: f32,
        front: f32,
    ) -> Option<(f32, f32)> {
        if self.drain == 0 {
            return None;
        }
        self.drain -= 1;
        Some(self.push(binaural, 0.0, side, front))
    }
}

#[cfg(test)]
mod tests;
