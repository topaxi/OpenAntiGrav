//! [`Recorder`]: builds a [`Replay`] one tick at a time, alongside a race.

use oag_gameplay::{InputSnapshot, PlayerInputs};

use crate::file::Replay;
use crate::header::Header;
use crate::pose::GhostLap;

/// Collects a run's inputs and hashes as it is driven.
///
/// **Called after each tick, with the inputs that tick was handed** - the
/// value [`Race::tick`] received, not a device read, since anything between
/// the two (a consumed press, a sanitised axis) is part of what the
/// simulation saw.
///
/// [`Race::tick`]: https://docs.rs/oag-game
#[derive(Debug, Clone)]
pub struct Recorder {
    replay: Replay,
}

impl Recorder {
    /// Starts a recording of a race whose state hashes to `initial_hash`
    /// before its first tick.
    #[must_use]
    pub fn new(header: Header, initial_hash: u64) -> Self {
        let streams = header.slots.len();
        Self {
            replay: Replay {
                header,
                inputs: vec![Vec::new(); streams],
                initial_hash,
                hashes: Vec::new(),
                ghost: None,
            },
        }
    }

    /// Records one tick: `inputs` is what it was handed, and `hash` the
    /// race's state hash after it - asked for only on the ticks one is stored.
    pub fn record(&mut self, inputs: &PlayerInputs, hash: impl FnOnce() -> u64) {
        // Asked before the push, which is what moves `ticks()` on.
        let due = self.hash_due();
        for (&slot, stream) in self
            .replay
            .header
            .slots
            .iter()
            .zip(self.replay.inputs.iter_mut())
        {
            stream.push(*inputs.get(usize::from(slot)));
        }
        if due {
            self.replay.hashes.push(hash());
        }
    }

    /// Whether the tick about to be recorded is one whose hash is stored.
    ///
    /// A caller that computes its hash eagerly can skip the work on every other
    /// tick by asking this first.
    #[must_use]
    pub fn hash_due(&self) -> bool {
        let interval = u64::from(self.replay.header.hash_interval.max(1));
        (self.ticks() + 1).is_multiple_of(interval)
    }

    /// Ticks recorded so far.
    #[must_use]
    pub fn ticks(&self) -> u64 {
        self.replay.ticks()
    }

    /// The header being recorded under.
    #[must_use]
    pub fn header(&self) -> &Header {
        &self.replay.header
    }

    /// The recording so far, as a [`Replay`], with `ghost` attached.
    ///
    /// A copy rather than a hand-over, because a run that just set a best lap
    /// is still going: Speed Lap never finishes, and the next lap may be
    /// quicker again.
    #[must_use]
    pub fn replay(&self, ghost: Option<GhostLap>) -> Replay {
        let mut replay = self.replay.clone();
        replay.ghost = ghost;
        replay
    }

    /// The recording, ending here.
    #[must_use]
    pub fn finish(self) -> Replay {
        self.replay
    }

    /// The snapshot slot `slot` was handed on tick `tick`, if recorded.
    #[must_use]
    pub fn input(&self, slot: u8, tick: u64) -> Option<&InputSnapshot> {
        let stream = self.replay.header.slots.iter().position(|&s| s == slot)?;
        self.replay.inputs[stream].get(usize::try_from(tick).ok()?)
    }
}
