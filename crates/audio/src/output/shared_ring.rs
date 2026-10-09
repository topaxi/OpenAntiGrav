//! The ring the browser's audio thread reads: interleaved stereo `f32` in the
//! module's own shared memory, with a fixed header a JavaScript
//! `AudioWorkletProcessor` (`web/audio-worklet.js`) reads by address.
//!
//! Native output keeps `rtrb`, whose layout is its own business. This one's
//! layout is the contract with JavaScript, so it is spelled out: a
//! [`Header`] of four `u32` words, and a power-of-two array of samples stored
//! as their `f32` bits. The indices are free-running sample counts that wrap
//! at `u32::MAX`; a slot is `index & (capacity - 1)`, which is why the
//! capacity is a power of two.
//!
//! One producer (the render-ahead loop, [`super::render::Ahead`]) and one
//! consumer (the worklet). The producer writes samples, then publishes
//! `write`; the consumer reads `write`, copies, then publishes `read`. Every
//! word is an atomic, so there is no `unsafe` here and the JavaScript side's
//! `Atomics.load`/`Atomics.store` pair with these the way two threads of this
//! module would. The [`SharedRing::pop`] here is the worklet's half in Rust:
//! the tests exercise it, and `audio-worklet.js` is a transcription of it.

use std::sync::atomic::{AtomicU32, Ordering};

/// Word offsets into [`Header`], the numbers `audio-worklet.js` uses.
pub(crate) mod word {
    /// Samples the consumer has taken, free-running.
    pub(crate) const READ: usize = 0;
    /// Samples the producer has published, free-running.
    pub(crate) const WRITE: usize = 1;
    /// Render quanta the consumer could not fill, counted once it has seen
    /// its first sample (a stream that has not started yet is not starving).
    pub(crate) const UNDERRUNS: usize = 2;
    /// Render quanta the consumer has played, short ones included.
    pub(crate) const QUANTA: usize = 3;
}

/// The words the worklet reads and writes, at a fixed address.
#[repr(C)]
#[derive(Debug, Default)]
pub(crate) struct Header {
    words: [AtomicU32; 4],
}

/// Interleaved stereo samples shared with the browser's audio thread.
#[derive(Debug)]
pub(crate) struct SharedRing {
    header: Box<Header>,
    samples: Box<[AtomicU32]>,
}

impl SharedRing {
    /// A ring holding at least `min_samples`, rounded up to a power of two.
    pub(crate) fn new(min_samples: usize) -> Self {
        let capacity = min_samples.max(2).next_power_of_two();
        Self {
            header: Box::default(),
            samples: (0..capacity).map(|_| AtomicU32::new(0)).collect(),
        }
    }

    pub(crate) fn capacity(&self) -> usize {
        self.samples.len()
    }

    fn word(&self, at: usize) -> u32 {
        self.header.words[at].load(Ordering::Acquire)
    }

    /// Samples published and not yet taken.
    pub(crate) fn occupied(&self) -> usize {
        self.word(word::WRITE).wrapping_sub(self.word(word::READ)) as usize
    }

    /// Samples the producer may still write.
    pub(crate) fn slots(&self) -> usize {
        self.capacity() - self.occupied()
    }

    /// Writes as much of `samples` as fits and publishes it; returns how much.
    pub(crate) fn push(&self, samples: &[f32]) -> usize {
        let mask = self.capacity() - 1;
        let write = self.word(word::WRITE);
        let n = samples.len().min(self.slots());
        for (i, &sample) in samples[..n].iter().enumerate() {
            let slot = (write as usize).wrapping_add(i) & mask;
            self.samples[slot].store(sample.to_bits(), Ordering::Relaxed);
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "n is at most the capacity, which fits a u32 index"
        )]
        self.header.words[word::WRITE].store(write.wrapping_add(n as u32), Ordering::Release);
        n
    }

    /// The worklet's half, in Rust: fills `out` from the ring, silence past
    /// what is there, and counts the quantum. Returns the samples taken.
    #[cfg(test)]
    pub(crate) fn pop(&self, out: &mut [f32]) -> usize {
        let mask = self.capacity() - 1;
        let read = self.word(word::READ);
        let write = self.word(word::WRITE);
        let n = out.len().min(write.wrapping_sub(read) as usize);
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = if i < n {
                f32::from_bits(
                    self.samples[(read as usize).wrapping_add(i) & mask].load(Ordering::Relaxed),
                )
            } else {
                0.0
            };
        }
        self.header.words[word::QUANTA].fetch_add(1, Ordering::Relaxed);
        if n < out.len() && write != 0 {
            self.header.words[word::UNDERRUNS].fetch_add(1, Ordering::Relaxed);
        }
        #[expect(clippy::cast_possible_truncation, reason = "n is at most the capacity")]
        self.header.words[word::READ].store(read.wrapping_add(n as u32), Ordering::Release);
        n
    }

    /// Quanta the worklet could not fill since it started.
    pub(crate) fn underruns(&self) -> u32 {
        self.word(word::UNDERRUNS)
    }

    /// Quanta the worklet has played.
    pub(crate) fn quanta(&self) -> u32 {
        self.word(word::QUANTA)
    }

    /// The header's address, for the worklet.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn header_addr(&self) -> usize {
        std::ptr::from_ref::<Header>(&self.header) as usize
    }

    /// The first sample's address, for the worklet.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn samples_addr(&self) -> usize {
        self.samples.as_ptr() as usize
    }

    /// Starts the counters over, for a test that wraps them.
    #[cfg(test)]
    fn set_indices(&self, at: u32) {
        self.header.words[word::READ].store(at, Ordering::Relaxed);
        self.header.words[word::WRITE].store(at, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_capacity_is_a_power_of_two_at_least_the_ask() {
        assert_eq!(SharedRing::new(6000).capacity(), 8192);
        assert_eq!(SharedRing::new(4096).capacity(), 4096);
    }

    #[test]
    fn what_is_pushed_comes_out_in_order_and_the_rest_is_silence() {
        let ring = SharedRing::new(8);
        assert_eq!(ring.push(&[1.0, 2.0, 3.0]), 3);
        let mut out = [9.0; 4];
        assert_eq!(ring.pop(&mut out), 3);
        assert_eq!(out, [1.0, 2.0, 3.0, 0.0]);
        assert_eq!(ring.underruns(), 1);
        assert_eq!(ring.quanta(), 1);
    }

    #[test]
    fn a_full_ring_takes_no_more_and_frees_what_is_read() {
        let ring = SharedRing::new(4);
        assert_eq!(ring.push(&[1.0; 6]), 4);
        assert_eq!(ring.slots(), 0);
        let mut out = [0.0; 2];
        ring.pop(&mut out);
        assert_eq!(ring.slots(), 2);
        assert_eq!(ring.underruns(), 0);
    }

    #[test]
    fn a_stream_that_has_not_started_is_not_counted_as_starving() {
        let ring = SharedRing::new(4);
        let mut out = [0.0; 2];
        ring.pop(&mut out);
        assert_eq!(ring.underruns(), 0);
        assert_eq!(ring.quanta(), 1);
    }

    #[test]
    fn the_indices_wrap_at_u32_max_without_losing_a_sample() {
        let ring = SharedRing::new(8);
        ring.set_indices(u32::MAX - 2);
        assert_eq!(ring.push(&[1.0, 2.0, 3.0, 4.0, 5.0]), 5);
        assert_eq!(ring.occupied(), 5);
        let mut out = [0.0; 5];
        assert_eq!(ring.pop(&mut out), 5);
        assert_eq!(out, [1.0, 2.0, 3.0, 4.0, 5.0]);
    }
}
