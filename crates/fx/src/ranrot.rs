//! The original's particle generator, `PsysRng_Next` (`0x088f8b60`): a
//! 17-word RANROT-B, ported so a render-side effect that the original builds
//! from a known seed lands on the same values.
//!
//! The algorithm, the seed fill and the range helpers are read on
//! `docs/ghidra/functions/psp-pulse-usa/prng.md`. The cloud field
//! (`crate::cloud`) is its first user: `FUN_08933c1c` reseeds this generator
//! from a `cloudGroup`'s `Seed` and draws every sprite from it, and a port fed
//! the seed read out of a running original's RAM reproduces every record that
//! group built, bit for bit (`clouds.md`). Why a second generator exists beside
//! `oag_core::Rng` at all is
//! `docs/architecture/adr/0056-a-render-side-port-of-the-particle-generator.md`.
//!
//! This is render-side only. The simulation's randomness stays
//! `oag_core::Rng`; nothing here reaches a state hash.

/// `PsysRng`'s state: the 17-word buffer and its two taps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranrot {
    buffer: [u32; 17],
    p1: usize,
    p2: usize,
}

impl Ranrot {
    /// `PsysRng_Reseed` (`0x088f8d30`): fills the buffer from `seed` with
    /// `s = rotl(s, 5) + 0x61` and resets the taps to `0` and `10`.
    ///
    /// Unlike `PsysRng_Seed` this discards no warm-up draws, which is the path
    /// the cloud build takes.
    #[must_use]
    pub fn reseeded(seed: u32) -> Self {
        let mut buffer = [0; 17];
        let mut s = seed;
        for word in &mut buffer {
            *word = s;
            s = s.rotate_left(5).wrapping_add(0x61);
        }
        Self {
            buffer,
            p1: 0,
            p2: 10,
        }
    }

    /// `PsysRng_Next`: one draw in `[0, 1]`, closed at the top.
    ///
    /// The word is converted the original's way, signed and then corrected by
    /// `2^32` when negative, and scaled by `2^-32`.
    pub fn next_f32(&mut self) -> f32 {
        let x = self.buffer[self.p2]
            .rotate_left(5)
            .wrapping_add(self.buffer[self.p1].rotate_left(3));
        self.buffer[self.p1] = x;
        self.p1 = if self.p1 == 0 { 16 } else { self.p1 - 1 };
        self.p2 = if self.p2 == 0 { 16 } else { self.p2 - 1 };
        #[allow(clippy::cast_possible_wrap)]
        let signed = x as i32;
        #[allow(clippy::cast_precision_loss)]
        let mut f = signed as f32;
        if signed < 0 {
            f += 4_294_967_296.0;
        }
        f * (1.0 / 4_294_967_296.0)
    }

    /// `Psys_RandFloatRange` (`0x088f8d94`): `lo + (hi - lo) * next`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let r = self.next_f32();
        lo + (hi - lo) * r
    }

    /// `Psys_RandIntRange` (`0x088f8c3c`): an integer in `lo..=hi`, the
    /// product truncated and clamped to `hi - lo` when the draw is exactly `1`.
    pub fn int_range(&mut self, lo: i32, hi: i32) -> i32 {
        let interval = hi - lo + 1;
        #[allow(clippy::cast_precision_loss)]
        let scaled = interval as f32 * self.next_f32();
        #[allow(clippy::cast_possible_truncation)]
        let mut i = scaled as i32;
        if interval <= i {
            i = hi - lo;
        }
        lo + i
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reseed fill is `rotl(s, 5) + 0x61` from the seed itself, the taps
    /// start at `0` and `10` - read off `PsysRng_Reseed`.
    #[test]
    fn the_reseed_fills_by_rotate_and_add() {
        let rng = Ranrot::reseeded(1);
        assert_eq!(rng.buffer[0], 1);
        assert_eq!(rng.buffer[1], (1u32 << 5) + 0x61);
        assert_eq!(rng.buffer[2], ((1u32 << 5) + 0x61).rotate_left(5) + 0x61);
        assert_eq!((rng.p1, rng.p2), (0, 10));
    }

    /// The first draw is `rotl(buf[10], 5) + rotl(buf[0], 3)`, stored back at
    /// tap 1, and both taps step down and wrap to 16.
    #[test]
    fn a_draw_adds_two_rotated_taps_and_steps_down() {
        let mut rng = Ranrot::reseeded(2079);
        let expect = rng.buffer[10]
            .rotate_left(5)
            .wrapping_add(rng.buffer[0].rotate_left(3));
        let _ = rng.next_f32();
        assert_eq!(rng.buffer[0], expect);
        assert_eq!((rng.p1, rng.p2), (16, 9));
    }

    /// `int_range` never leaves `lo..=hi`, including the clamp's edge.
    #[test]
    fn int_range_stays_inside_its_bounds() {
        let mut rng = Ranrot::reseeded(9169);
        for _ in 0..2000 {
            assert!((0..=3).contains(&rng.int_range(0, 3)));
        }
    }
}
