//! Deterministic pseudo-random number generation.
//!
//! The simulation never touches OS entropy. Every random draw comes from a
//! seeded generator whose state is part of the world snapshot, so a replay
//! reproduces the same sequence.
//!
//! # This is not the original generator
//!
//! Wipeout Pulse has its own PRNG, and AI decisions and pickup rolls will only
//! match the original once that generator is recovered from the binary. Until
//! then this crate provides a placeholder with the right *shape*: seeded,
//! snapshot-able, no hidden global state. Replacing the algorithm later is a
//! one-file change, which is the whole point of routing every draw through
//! here from the start.
//!
//! Tracked as an open item in `docs/reverse-engineering/methodology.md`.

/// A small, fast, fully deterministic generator (`xoshiro128**`).
///
/// Identical output on every platform: all operations are integer, and integer
/// overflow behaviour in Rust is defined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng {
    state: [u32; 4],
}

impl Rng {
    /// Seeds the generator.
    ///
    /// Seed 0 is remapped, because an all-zero state is a fixed point of
    /// `xoshiro` and would emit nothing but zeroes.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let seed = if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        };

        // SplitMix64 expansion, the conventional way to spread a small seed
        // across a larger state without correlating the words.
        let mut z = seed;
        let mut next = || {
            z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            x ^ (x >> 31)
        };

        let a = next();
        let b = next();
        Self {
            state: [a as u32, (a >> 32) as u32, b as u32, (b >> 32) as u32],
        }
    }

    /// Returns the next `u32`.
    ///
    /// The four state updates are **sequential**, and that matters: `s[1]` and
    /// `s[0]` read the *already updated* `s[2]` and `s[3]`. Writing them as one
    /// parallel assignment from the old words costs the generator two of its
    /// four terms, and because every operation here is linear over GF(2) the
    /// damage is measurable rather than aesthetic: the transition matrix drops
    /// from rank 128 to 119, so the map stops being a bijection, state is lost
    /// on every step, and the 2^128 - 1 period is gone. It still passes any test
    /// that only checks the generator against itself, which is why
    /// [`tests::the_step_is_a_bijection`] and the reference vector below exist.
    pub fn next_u32(&mut self) -> u32 {
        let s = &mut self.state;
        let result = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 9;

        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(11);

        result
    }

    /// Returns the next `u64`.
    pub fn next_u64(&mut self) -> u64 {
        u64::from(self.next_u32()) << 32 | u64::from(self.next_u32())
    }

    /// Returns a value in `0..n`, or `0` when `n` is zero.
    ///
    /// Uses Lemire's multiply-shift with rejection: unbiased, and the rejection
    /// branch is taken vanishingly rarely, but it is still a *deterministic*
    /// branch, so replays are unaffected.
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        let threshold = n.wrapping_neg() % n;
        loop {
            let x = self.next_u32();
            let m = u64::from(x) * u64::from(n);
            if (m as u32) >= threshold {
                return (m >> 32) as u32;
            }
        }
    }

    /// Returns a value in `[0, 1)`.
    ///
    /// Built by constructing the float's bits directly rather than dividing, so
    /// there is no rounding step whose result could vary.
    pub fn next_f32(&mut self) -> f32 {
        // 24 bits of mantissa, scaled by 2^-24. Exact.
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first eight outputs from state `[1, 2, 3, 4]`.
    ///
    /// Produced from the published `xoshiro128starstar.c` at
    /// <https://prng.di.unimi.it/xoshiro128starstar.c>, by an independent
    /// implementation of that source rather than by running this one. Two of
    /// them being zero or near-zero is expected: the output depends on `s[1]`
    /// alone, and a tiny seed state takes a few steps to fill out.
    ///
    /// **Do not regenerate these from the implementation.** They are the only
    /// thing here that would notice the generator quietly becoming a different
    /// generator, which is exactly what happened before.
    const REFERENCE_1_2_3_4: [u32; 8] = [
        0x0000_2d00,
        0x0000_0000,
        0x005a_7080,
        0x0438_9d80,
        0x7919_9d9b,
        0x6196_3b24,
        0x4cb9_b57a,
        0xde9d_7431,
    ];

    #[test]
    fn matches_the_published_reference_vector() {
        let mut r = Rng {
            state: [1, 2, 3, 4],
        };
        for (i, &expected) in REFERENCE_1_2_3_4.iter().enumerate() {
            assert_eq!(r.next_u32(), expected, "output {i}");
        }
    }

    /// Undoes one step, then checks the round trip.
    ///
    /// The reference generator's step is a bijection on the 2^128 states, which
    /// is what its full period rests on. An inverse that reconstructs the input
    /// exactly is a direct proof of that for the states tried, and it fails
    /// loudly for a step that has lost a term.
    fn unstep(next: [u32; 4]) -> [u32; 4] {
        let [s0_new, s1_new, s2_new, s3_new] = next;
        let s3_mid = s3_new.rotate_right(11);
        let s0 = s0_new ^ s3_mid;

        // `s2_mid ^ (s1 << 9) == s2_new` with `s1 == s1_new ^ s2_mid`, so
        // `u ^ (u << 9) == s2_new ^ (s1_new << 9)` for `u == s2_mid`. Shifting
        // left only ever moves bits up, so substituting the equation into
        // itself converges once the shift has walked off the top: 32 / 9 gives
        // four rounds.
        let v = s2_new ^ (s1_new << 9);
        let mut u = v;
        for _ in 0..4 {
            u = v ^ (u << 9);
        }

        let s1 = s1_new ^ u;
        [s0, s1, u ^ s0, s3_mid ^ s1]
    }

    #[test]
    fn the_step_is_a_bijection() {
        let mut r = Rng::new(2024);
        for step in 0..4096 {
            let before = r.state;
            r.next_u32();
            assert_eq!(
                unstep(r.state),
                before,
                "step {step} is not invertible, so the map is not a bijection"
            );
        }
        // Including the awkward states: all-ones, and one bit set anywhere.
        let mut states = vec![[u32::MAX; 4]];
        for word in 0..4 {
            for bit in [0, 1, 8, 9, 22, 23, 31] {
                let mut s = [0u32; 4];
                s[word] = 1 << bit;
                states.push(s);
            }
        }
        for state in states {
            let mut r = Rng { state };
            r.next_u32();
            assert_eq!(
                unstep(r.state),
                state,
                "state {state:08x?} is not recovered"
            );
        }
    }

    #[test]
    fn same_seed_gives_same_sequence() {
        let mut a = Rng::new(12345);
        let mut b = Rng::new(12345);
        for _ in 0..1000 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        let diverged = (0..64).any(|_| a.next_u32() != b.next_u32());
        assert!(diverged);
    }

    #[test]
    fn zero_seed_is_not_degenerate() {
        let mut r = Rng::new(0);
        let any_nonzero = (0..16).any(|_| r.next_u32() != 0);
        assert!(any_nonzero, "an all-zero xoshiro state emits only zeroes");
    }

    #[test]
    fn below_stays_in_range_and_covers_it() {
        let mut r = Rng::new(7);
        let mut seen = [false; 6];
        for _ in 0..10_000 {
            let v = r.below(6);
            assert!(v < 6);
            seen[v as usize] = true;
        }
        assert!(seen.iter().all(|&s| s), "every value in 0..6 should appear");
        assert_eq!(r.below(0), 0);
    }

    #[test]
    fn next_f32_stays_in_unit_interval() {
        let mut r = Rng::new(99);
        for _ in 0..10_000 {
            let v = r.next_f32();
            assert!((0.0..1.0).contains(&v), "{v} out of range");
        }
    }
}
