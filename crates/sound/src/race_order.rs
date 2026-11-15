//! Which song the race playlist plays next, by the player's `MUSIC ORDER`.
//!
//! Sequential is `RacePick::after`/`before` on the pool's numbering (resolved on the fetch
//! worker, which knows the listing). Random asks the [`Shuffle`] for an index here on the frame
//! thread and hands the worker an exact one: the playlist's length is known by then
//! ([`Audio::race_tracks`] once a song has landed), and the first song of a run, whose length is
//! not, is drawn on the worker from a seed ([`RacePick::random_first`]).

use crate::shuffle::{MusicOrder, Shuffle};
use crate::*;

/// The seed a shuffle starts from until the host gives it one; tests rely on it being fixed.
const DEFAULT_SEED: u64 = 0x5348_5546_464C_4531;

/// The order setting and the deck it plays by.
#[derive(Debug)]
pub(crate) struct RaceOrder {
    pub(crate) order: MusicOrder,
    pub(crate) shuffle: Shuffle,
}

impl RaceOrder {
    pub(crate) fn new(order: MusicOrder) -> Self {
        Self {
            order,
            shuffle: Shuffle::new(DEFAULT_SEED),
        }
    }

    fn random(&self) -> bool {
        self.order == MusicOrder::Random
    }
}

impl Audio {
    /// Plays the race playlist in `order` from the next song on. A song fetched ahead for the
    /// old order is dropped; the one playing goes on.
    pub fn set_music_order(&mut self, order: MusicOrder) {
        if self.race_order.order != order {
            self.race_order.order = order;
            self.race_prefetch = None;
        }
    }

    /// Seeds the shuffle from something the host has and the simulation does not (the clock).
    pub fn seed_music_shuffle(&mut self, seed: u64) {
        self.race_order.shuffle.reseed(seed);
    }

    /// The playlist is on `index`: remembered for the shuffle's "previous" and its cycle.
    pub(super) fn set_race_index(&mut self, index: usize) {
        self.race_index = Some(index);
        self.race_order.shuffle.landed(index);
    }

    /// The playlist's length as far as the frame thread knows it.
    fn known_playlist_len(&self, discs: &MusicDiscs) -> usize {
        pool::playlist_len(&discs.pool, self.race_tracks)
    }

    /// What follows the song at `index`, without moving: for the fetch ahead.
    pub(super) fn peek_next_race_pick(&mut self, discs: &MusicDiscs, index: usize) -> RacePick {
        let len = self.known_playlist_len(discs);
        if self.race_order.random()
            && let Some(next) = self.race_order.shuffle.peek_next(len)
        {
            return RacePick::exact(next);
        }
        RacePick::after(index)
    }

    /// What follows the song at `index`, moving the playlist on to it.
    pub(super) fn take_next_race_pick(&mut self, discs: &MusicDiscs, index: usize) -> RacePick {
        let len = self.known_playlist_len(discs);
        if self.race_order.random()
            && let Some(next) = self.race_order.shuffle.advance(len)
        {
            return RacePick::exact(next);
        }
        RacePick::after(index)
    }

    /// The song played before the one at `index`, moving the playlist back to it; `None` under
    /// Random when nothing earlier is remembered.
    pub(super) fn take_previous_race_pick(
        &mut self,
        discs: &MusicDiscs,
        index: usize,
    ) -> Option<RacePick> {
        if !self.race_order.random() {
            return Some(RacePick::before(index));
        }
        let len = self.known_playlist_len(discs);
        self.race_order.shuffle.previous(len).map(RacePick::exact)
    }

    /// The first song of a run: the saved place if there is one and it holds, else the disc's
    /// own first race song, or under Random a song drawn on the worker.
    pub(super) fn first_race_pick(&mut self) -> RacePick {
        let saved = self.pending_restore.take();
        if self.race_order.random() {
            RacePick::random_first(self.race_order.shuffle.draw_seed()).checking(saved)
        } else {
            RacePick::first_race().checking(saved)
        }
    }
}

#[cfg(test)]
mod tests;
