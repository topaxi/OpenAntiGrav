//! [`Verifier`]: plays a [`Replay`]'s inputs into a race and checks every
//! stored hash on the way.
//!
//! **This is what "the simulation changed" means for a replay.** Not a version
//! string compared against another, but the run itself re-driven: if every
//! stored hash comes back, this build reproduces the run bit for bit; the
//! first one that does not is where it stopped, and [`Desync`] says which tick
//! that was. A full-race replay viewer stops drawing the resimulation there;
//! a ghost does not care, because it is drawn from poses. See ADR-0055.

use oag_gameplay::PlayerInputs;

use crate::file::Replay;

/// Where a replay stopped reproducing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("replay desynchronised by tick {tick}: expected {expected:#018x}, got {found:#018x}")]
pub struct Desync {
    /// The world tick whose hash disagreed. `0` is the state before the first
    /// tick - a race that was not set up the way the recording's was.
    pub tick: u64,
    /// The hash the recording stored.
    pub expected: u64,
    /// The hash this build arrived at.
    pub found: u64,
}

/// Steps through a replay's ticks, checking hashes as they fall due.
#[derive(Debug)]
pub struct Verifier<'a> {
    replay: &'a Replay,
    tick: u64,
}

impl<'a> Verifier<'a> {
    /// Checks the race's state before its first tick against the recording's.
    ///
    /// # Errors
    ///
    /// [`Desync`] at tick 0 when the race was not built the way the recorded
    /// one was: another track, class, team or seed.
    pub fn new(replay: &'a Replay, initial_hash: u64) -> Result<Self, Desync> {
        if initial_hash != replay.initial_hash {
            return Err(Desync {
                tick: 0,
                expected: replay.initial_hash,
                found: initial_hash,
            });
        }
        Ok(Self { replay, tick: 0 })
    }

    /// The inputs for the next tick, or `None` once the recording is spent.
    #[must_use]
    pub fn next_inputs(&self) -> Option<PlayerInputs> {
        (self.tick < self.replay.ticks()).then(|| self.replay.inputs_at(self.tick))
    }

    /// Whether the tick about to be reported has a stored hash to check.
    #[must_use]
    pub fn hash_due(&self) -> bool {
        let interval = u64::from(self.replay.header.hash_interval.max(1));
        (self.tick + 1).is_multiple_of(interval) && self.stored_index().is_some()
    }

    fn stored_index(&self) -> Option<usize> {
        let interval = u64::from(self.replay.header.hash_interval.max(1));
        let index = usize::try_from((self.tick + 1) / interval)
            .ok()?
            .checked_sub(1)?;
        (index < self.replay.hashes.len()).then_some(index)
    }

    /// Reports that a tick ran; `hash` is asked for only when one is due.
    ///
    /// # Errors
    ///
    /// [`Desync`] when the stored hash for this tick disagrees.
    pub fn after_tick(&mut self, hash: impl FnOnce() -> u64) -> Result<(), Desync> {
        let due = self.hash_due().then(|| self.stored_index()).flatten();
        self.tick += 1;
        if let Some(index) = due {
            let found = hash();
            let expected = self.replay.hashes[index];
            if found != expected {
                return Err(Desync {
                    tick: self.tick,
                    expected,
                    found,
                });
            }
        }
        Ok(())
    }

    /// Ticks played so far.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Plays the whole recording through `step`, which runs one tick with the
    /// inputs it is handed and returns the race's state hash after it.
    ///
    /// The convenience form, for a caller that can afford a hash every tick;
    /// [`Self::next_inputs`] and [`Self::after_tick`] are the incremental one.
    ///
    /// # Errors
    ///
    /// The first [`Desync`], at tick 0 or later.
    pub fn run(
        replay: &Replay,
        initial_hash: u64,
        mut step: impl FnMut(&PlayerInputs) -> u64,
    ) -> Result<u64, Desync> {
        let mut verifier = Verifier::new(replay, initial_hash)?;
        while let Some(inputs) = verifier.next_inputs() {
            let hash = step(&inputs);
            verifier.after_tick(|| hash)?;
        }
        Ok(verifier.tick())
    }
}
