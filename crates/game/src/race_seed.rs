//! The seed of a race started from the front end.
//!
//! The world generator draws which pickup a weapon pad hands over and the AI's mistakes, so a
//! fixed seed makes every race deal the same weapons. The front end draws a fresh one per race;
//! `--seed`, a network race's server seed and every headless or `--race` run keep their own (the
//! seed is recorded in a replay or ghost header, which is what rebuilds the race exactly).

use std::sync::atomic::{AtomicU64, Ordering};

use web_time::{SystemTime, UNIX_EPOCH};

static DRAWS: AtomicU64 = AtomicU64::new(0);

/// A seed no earlier call in this process returned, mixed from the system clock.
///
/// Drawn here in the composition root: the simulation crates never see a clock.
#[must_use]
pub fn fresh() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            since.as_secs().wrapping_mul(1_000_000_007) ^ u64::from(since.subsec_nanos())
        });
    mix(nanos.wrapping_add(
        DRAWS
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15),
    ))
}

/// SplitMix64's finaliser, so neighbouring inputs give unrelated seeds.
const fn mix(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// The seed a front-end race launches with: the one already named (`--seed`), else a fresh draw.
#[must_use]
pub fn for_launch(named: Option<u64>) -> u64 {
    named.unwrap_or_else(fresh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_launches_get_different_seeds() {
        assert_ne!(for_launch(None), for_launch(None));
    }

    #[test]
    fn a_named_seed_wins() {
        assert_eq!(for_launch(Some(7)), 7);
    }
}
