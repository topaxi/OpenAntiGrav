//! Which side of a fork a driver takes.
//!
//! **Recovered, not ours.** `Ai_ChooseBranch` (`0x08854920`) flips a fair coin,
//! bit 8 of `rand()`, each time a craft enters a path whose exit junction
//! forks, and holds the choice until the craft is past the merge. Nothing else
//! enters it: not position, class, difficulty, pilot or corridor. Wipeout 2048,
//! HD and Omega flip the same coin. See
//! `docs/ghidra/functions/psp-pulse-usa/ai-branch-choice.md`.
//!
//! **What is ours is where the coin comes from.** The original draws from the
//! one global generator everything shares; a draw here comes off
//! `oag_core::Rng` seeded from this driver's own seed and how many forks it has
//! decided, so it moves nothing else in the race and replays identically.
//!
//! A fork can sit inside an alternate (2048's `square`, `mall`, `subway`), so a
//! way round is a *sequence* of coins: [`choose`] rolls them in order against
//! each route's own `choices` (`oag_race::course::Route::choices`), the same sequence the original
//! rolls one path at a time, so a route two forks deep is taken one time in
//! four. Rolling them all at the first fork rather than as each is reached
//! changes when the draws happen, not what they decide.

use oag_core::rng::Rng;

/// What a driver carries about forks. Plain integers, so it sits in the world
/// snapshot beside the rest of [`crate::Driver`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Branching {
    /// The line the driver is on: `0` the ring, `k` the course's route `k - 1`.
    pub route: u16,
    /// The ring path whose fork this driver last decided at, plus one; `0`
    /// when it is not on a pre-fork path. The latch that makes the coin one
    /// draw per visit rather than one per tick - `Ai_ChooseBranch`'s state 1.
    pub decided_at: u16,
    /// Whether it has reached the route's own samples - state 2, after which
    /// arriving back on the ring part of the line means it is past the merge.
    pub entered: bool,
    /// Forks decided so far, the counter the coins are drawn against.
    pub visits: u32,
}

impl Branching {
    /// On the ring, nothing decided: where a craft is put back after a respawn
    /// or a teleport. **Ours**: the original keeps its excluded path through a
    /// respawn and lets the next locate sort it out, which this ring has no
    /// equivalent of. The visit count survives, so the next coin is a fresh one.
    #[must_use]
    pub fn on_ring(self) -> Self {
        Self {
            visits: self.visits,
            ..Self::default()
        }
    }
}

/// The stream a branch coin is drawn from, so it never shares a draw with the
/// noise streams in `driver.rs`.
const BRANCH_STREAM: u64 = 0x6272_616e_6368;

/// One fair coin for `seed`'s `visit`-th draw: `true` takes the alternate.
///
/// Bit 8, like the original's `andi a0,v0,0x100` at `0x08854a24`, and with the
/// same sense: bit set excludes the alternate, so it is clear that takes it.
/// Any single bit of a well-mixed draw is fair; this one is kept so the
/// reading and the port say the same thing.
#[must_use]
pub fn coin(seed: u32, visit: u32) -> bool {
    let mixed = (u64::from(seed) << 32 | u64::from(visit)) ^ BRANCH_STREAM;
    Rng::new(mixed).next_u32() & 0x100 == 0
}

/// Rolls coins against `routes` - each a route's index and its `choices` - and
/// returns the route they pick, or `None` for the ring.
///
/// The first coin is the ring fork: `false` stays on the ring. Each later coin
/// narrows the candidates to the routes whose choices begin with what has been
/// rolled, until one route's whole sequence is matched. A roll that matches
/// nothing (a dead-end alternate the course dropped) stays on the ring.
/// `visits` is advanced once per coin drawn.
pub fn choose<'a>(
    seed: u32,
    visits: &mut u32,
    routes: impl Iterator<Item = (usize, &'a [bool])> + Clone,
) -> Option<usize> {
    let mut rolled: Vec<bool> = Vec::new();
    // A route is never deeper than the file has paths; this bounds a malformed
    // one rather than the shipped data.
    for _ in 0..64 {
        let heads = coin(seed, *visits);
        *visits = visits.wrapping_add(1);
        rolled.push(heads);
        if rolled == [false] {
            return None;
        }
        let mut candidates = routes.clone().filter(|(_, c)| c.starts_with(&rolled));
        let first = candidates.next()?;
        if candidates.next().is_none() && first.1.len() == rolled.len() {
            return Some(first.0);
        }
        if let Some(exact) = routes.clone().find(|(_, c)| *c == rolled.as_slice()) {
            return Some(exact.0);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_coin_is_fair_over_many_visits() {
        let heads = (0..10_000).filter(|&v| coin(0x1234_5678, v)).count();
        assert!(
            (4_700..=5_300).contains(&heads),
            "{heads} alternates in 10,000 coins"
        );
    }

    #[test]
    fn the_coin_is_the_same_on_every_run() {
        let first: Vec<bool> = (0..64).map(|v| coin(7, v)).collect();
        let second: Vec<bool> = (0..64).map(|v| coin(7, v)).collect();
        assert_eq!(first, second);
    }

    #[test]
    fn eight_drivers_do_not_all_take_the_same_side() {
        // The maintainer's observation, as an assertion: before this, every
        // craft took the same branch at every fork.
        let sides: Vec<bool> = (1..=8u32)
            .map(|slot| coin(slot.wrapping_mul(0x9e37_79b9), 0))
            .collect();
        assert!(sides.contains(&true) && sides.contains(&false), "{sides:?}");
    }

    #[test]
    fn a_route_two_forks_deep_is_taken_one_time_in_four() {
        // The nested shape of 2048's `square`: the ring fork, then a second
        // fork on the alternate.
        let routes: [(usize, &[bool]); 2] = [(0, &[true, false]), (1, &[true, true])];
        let mut counts = [0usize; 3];
        let mut visits = 0;
        for seed in 1..=4_000u32 {
            match choose(seed, &mut visits, routes.iter().copied()) {
                None => counts[2] += 1,
                Some(r) => counts[r] += 1,
            }
        }
        assert!((1_850..=2_150).contains(&counts[2]), "ring {counts:?}");
        assert!((850..=1_150).contains(&counts[0]), "first leaf {counts:?}");
        assert!((850..=1_150).contains(&counts[1]), "second leaf {counts:?}");
    }

    #[test]
    fn a_single_route_is_half_and_half() {
        let routes: [(usize, &[bool]); 1] = [(3, &[true])];
        let mut visits = 0;
        let taken = (1..=4_000u32)
            .filter(|&seed| choose(seed, &mut visits, routes.iter().copied()) == Some(3))
            .count();
        assert!((1_850..=2_150).contains(&taken), "{taken} of 4,000");
    }
}
