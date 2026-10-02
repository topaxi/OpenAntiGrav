//! Ken Perlin's one-dimensional gradient noise, as `FUN_088f2aa4` and
//! `FUN_088f2918` build it.
//!
//! **The table is the original's law and its contents are not.** The
//! executable fills a 256-entry permutation and a gradient per entry,
//! `(rand % 512 - 256) / 256`, from the C library's `rand()` the first time
//! it is asked, so no two runs of the original share a table. Here the same
//! table is drawn from the seeded [`Rng`]: the shape of the noise and its
//! statistics are the original's, the particular wind a race gets is chosen,
//! not measured.

use oag_core::Rng;

/// Entries in the permutation and gradient tables.
const SIZE: usize = 256;

/// One table.
#[derive(Debug, Clone)]
pub struct Perlin {
    permutation: [u8; SIZE],
    gradient: [f32; SIZE],
}

impl Perlin {
    /// Fills a table: identity permutation shuffled by `rand & 0xff` swaps from
    /// the top down, as `FUN_088f2604` does, and a gradient per entry.
    pub fn new(rng: &mut Rng) -> Self {
        let mut permutation = [0u8; SIZE];
        let mut gradient = [0.0f32; SIZE];
        for (index, (slot, gradient)) in permutation.iter_mut().zip(&mut gradient).enumerate() {
            *slot = index as u8;
            *gradient = ((rng.next_u32() % 512) as f32 - 256.0) / 256.0;
        }
        for index in (1..SIZE).rev() {
            let other = (rng.next_u32() & 0xff) as usize;
            permutation.swap(index, other);
        }
        Self {
            permutation,
            gradient,
        }
    }

    /// `FUN_088f2918`: the noise at `x`, about `-0.5..0.5`.
    #[must_use]
    pub fn noise(&self, x: f32) -> f32 {
        let shifted = x + 4096.0;
        let cell = shifted as u32;
        let fraction = shifted - cell as f32;
        let low = usize::from(self.permutation[(cell & 0xff) as usize]);
        let high = usize::from(self.permutation[((cell & 0xff) as usize + 1) & 0xff]);
        let (a, b) = (self.gradient[low], self.gradient[high]);
        fraction * a
            + fraction * fraction * (3.0 - fraction * 2.0) * ((fraction - 1.0) * b - fraction * a)
    }

    /// `FUN_088f2aa4`: the octaves summed, each half the last, scaled by
    /// `amplitude`. `phase` is the struct's first word, `frequency` its third.
    #[must_use]
    pub fn fractal(
        &self,
        clock: f32,
        phase: f32,
        octaves: u32,
        frequency: f32,
        amplitude: f32,
    ) -> f32 {
        let mut sum = 0.0;
        let mut weight = 1.0;
        for octave in 1..=octaves {
            sum += weight * self.noise(octave as f32 * (clock * frequency + phase));
            weight *= 0.5;
        }
        sum * amplitude
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_noise_is_zero_on_a_lattice_point_and_continuous_between() {
        let table = Perlin::new(&mut Rng::new(7));
        for whole in [0.0f32, 3.0, 100.0] {
            assert!(table.noise(whole).abs() < 1e-3, "{whole}");
        }
        let (a, b) = (table.noise(10.499), table.noise(10.501));
        assert!((a - b).abs() < 0.01, "{a} {b}");
    }

    #[test]
    fn it_stays_within_the_amplitude_it_is_given() {
        let table = Perlin::new(&mut Rng::new(11));
        let mut peak = 0.0f32;
        for step in 0..4000 {
            let value = table.fractal(step as f32 * 0.37, 123.0, 2, 0.15, 25.0);
            peak = peak.max(value.abs());
        }
        assert!(peak > 3.0 && peak < 25.0 * 1.5, "{peak}");
    }
}
