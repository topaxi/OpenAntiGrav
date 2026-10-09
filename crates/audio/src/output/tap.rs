//! A recording of exactly what the output callback handed the device.
//!
//! **The only instrument that sees the signal a real device received.**
//! `--dump-audio` forces the null backend by construction - with a stream
//! attached the callback is already draining the mixer, and pulling the same
//! samples from the frame loop would race it - so a fault that only appears
//! with hardware attached cannot appear in that file. This records the same
//! buffer the stream is handed, from inside the callback.
//!
//! It exists because a step detector is the wrong instrument for a *thump*. A
//! click is a discontinuity between two samples and `Health::scan` finds those;
//! a low-frequency transient is not. A full-scale 50 Hz sine moves 0.0065
//! between samples at 48 kHz, which is under the noise floor of a busy race -
//! so an audible bass thump can be, and apparently is, invisible to every
//! counter in `Health`. A waveform is not.
//!
//! Bounded and preallocated: an audio thread that grows a `Vec` is an audio
//! thread that allocates, which is a dropout of its own.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

/// A fixed-length recording, filled by the callback and written by the frame
/// loop.
#[derive(Debug)]
pub struct Tap {
    samples: Mutex<Vec<f32>>,
    capacity: usize,
    /// How much has been recorded, readable without taking the lock so the
    /// frame loop can ask "is it full yet" sixty times a second for free.
    len: AtomicUsize,
    written: AtomicBool,
}

impl Tap {
    /// Room for `samples` interleaved stereo samples.
    #[must_use]
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn new(samples: usize) -> Self {
        Self {
            samples: Mutex::new(Vec::with_capacity(samples)),
            capacity: samples,
            len: AtomicUsize::new(0),
            written: AtomicBool::new(false),
        }
    }

    /// Appends one callback's worth, from the callback.
    ///
    /// `try_lock`, on the same reasoning the mixer's own callback uses: the
    /// only other holder is the frame loop taking it once, at the end, and a
    /// recording with a hole in it is better than a dropout in the thing being
    /// recorded.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn push(&self, stereo: &[f32]) {
        let len = self.len.load(Relaxed);
        if len >= self.capacity {
            return;
        }
        let Ok(mut samples) = self.samples.try_lock() else {
            return;
        };
        let room = self.capacity - samples.len();
        let take = stereo.len().min(room);
        samples.extend_from_slice(&stereo[..take]);
        self.len.store(samples.len(), Relaxed);
    }

    /// Whether there is a full recording waiting to be written.
    #[must_use]
    pub(crate) fn is_full(&self) -> bool {
        !self.written.load(Relaxed) && self.len.load(Relaxed) >= self.capacity
    }

    /// Takes the recording, exactly once.
    pub(crate) fn take(&self) -> Option<Vec<f32>> {
        if self.written.swap(true, Relaxed) {
            return None;
        }
        let mut samples = self.samples.lock().ok()?;
        Some(std::mem::take(&mut samples))
    }

    /// Samples recorded so far.
    #[must_use]
    pub fn recorded(&self) -> usize {
        self.len.load(Relaxed)
    }
}
