//! The race playlist's order: [`MusicOrder`] and the [`Shuffle`] a random one plays by.
//!
//! Pulse's `Music Order` option offers Sequential and Random, Sequential by default
//! (`docs/ui/menus-original.md`, "Game Options"). Sequential is the pool's own numbering walked
//! in order (`pool`). Random plays a **deck**: every song once in a shuffled order, then a fresh
//! shuffle, never opening a cycle on the song that just ended.
//!
//! Presentation randomness only. It owns its own [`Rng`] seeded by the host, and no simulation
//! state, state hash or race `Rng` reaches or leaves it.

use oag_core::Rng;
use serde::{Deserialize, Serialize};

/// How many played songs are remembered for "previous".
const TRAIL_MAX: usize = 128;

/// In which order the race playlist plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicOrder {
    /// The playlist's own numbering, wrapping. The default, as on the disc.
    #[default]
    Sequential,
    /// A shuffled deck: every song once before any song twice.
    Random,
}

impl MusicOrder {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Sequential => "sequential",
            Self::Random => "random",
        }
    }

    /// Every value, for the menus and for error messages.
    pub const ALL: [Self; 2] = [Self::Sequential, Self::Random];
}

impl std::str::FromStr for MusicOrder {
    type Err = String;

    fn from_str(text: &str) -> std::result::Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|order| order.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a music order; try sequential or random"))
    }
}

impl std::fmt::Display for MusicOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// The shuffled deck and the songs played before, over playlist indices.
///
/// The playlist's length is passed to every call, not stored as the truth: it grows as folder
/// and disc scans land and changes when a source is switched, and the deck follows it.
#[derive(Debug, Clone)]
pub struct Shuffle {
    rng: Rng,
    /// This cycle's songs not played yet; the next one is the last element.
    deck: Vec<usize>,
    /// The playlist length the deck was built for, `0` while no deck exists.
    built_for: usize,
    /// Songs played, oldest first, ending with the newest (which may not be the current one
    /// while `back` is non-zero).
    trail: Vec<usize>,
    /// How many songs back from the newest the current one is, after "previous".
    back: usize,
}

impl Shuffle {
    /// A shuffle drawing from `seed`.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Rng::new(seed),
            deck: Vec::new(),
            built_for: 0,
            trail: Vec::new(),
            back: 0,
        }
    }

    /// Reseeds the generator and forgets the deck; the trail and the current song stay.
    pub fn reseed(&mut self, seed: u64) {
        self.rng = Rng::new(seed);
        self.deck.clear();
        self.built_for = 0;
    }

    /// A seed for a one-off pick (a run's first race song, chosen where the length is known).
    pub fn draw_seed(&mut self) -> u64 {
        self.rng.next_u64()
    }

    /// The song the playlist is on.
    #[must_use]
    pub fn current(&self) -> Option<usize> {
        self.trail
            .len()
            .checked_sub(self.back + 1)
            .map(|i| self.trail[i])
    }

    /// The playlist is on `index` for a reason this did not choose (a run's first song, a
    /// resume, a switched source): it is the song just played, and the rest of the cycle is
    /// every other song. A song this chose is already the current one and nothing moves.
    pub fn landed(&mut self, index: usize) {
        if self.current() == Some(index) {
            return;
        }
        self.trail.clear();
        self.trail.push(index);
        self.back = 0;
        self.deck.clear();
        self.built_for = 0;
    }

    /// The song [`Self::advance`] will play, without consuming it. The same answer until the
    /// playlist moves, so a fetch started from it is the one the advance wants.
    pub fn peek_next(&mut self, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        if self.back > 0 {
            return self.trail.get(self.trail.len() - self.back).copied();
        }
        self.fit(len);
        self.deck.last().copied()
    }

    /// Moves on and returns the song to play.
    pub fn advance(&mut self, len: usize) -> Option<usize> {
        let next = self.peek_next(len)?;
        if self.back > 0 {
            self.back -= 1;
        } else {
            self.deck.pop();
            self.trail.push(next);
            if self.trail.len() > TRAIL_MAX {
                self.trail.remove(0);
            }
        }
        Some(next)
    }

    /// Steps back to the song played before the current one, or `None` at the start of what
    /// is remembered. Going forward again replays the same songs before the deck is touched.
    pub fn previous(&mut self, len: usize) -> Option<usize> {
        let earlier = self
            .trail
            .len()
            .checked_sub(self.back + 2)
            .filter(|&i| self.trail[i] < len)?;
        self.back += 1;
        Some(self.trail[earlier])
    }

    /// Brings the deck to `len` songs: builds it when there is none, adds the songs a longer
    /// playlist gained at random places, drops the ones a shorter one lost, and starts the next
    /// cycle when this one is spent.
    fn fit(&mut self, len: usize) {
        if self.built_for == 0 {
            let current = self.current();
            self.deck = (0..len).filter(|&i| Some(i) != current).collect();
            self.shuffle_deck();
            self.built_for = len;
        } else if self.built_for != len {
            self.deck.retain(|&i| i < len);
            for gained in self.built_for..len {
                let at = self.rng.below(self.deck.len() as u32 + 1) as usize;
                self.deck.insert(at, gained);
            }
            self.built_for = len;
        }
        if self.deck.is_empty() {
            self.deck = (0..len).collect();
            self.shuffle_deck();
            // The cycle must not open on the song that just played.
            if len > 1 && self.trail.last() == self.deck.last() {
                let other = self.rng.below(len as u32 - 1) as usize;
                let last = self.deck.len() - 1;
                self.deck.swap(other, last);
            }
        }
    }

    fn shuffle_deck(&mut self) {
        for i in (1..self.deck.len()).rev() {
            let j = self.rng.below(i as u32 + 1) as usize;
            self.deck.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests;
