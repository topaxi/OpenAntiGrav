//! `Weather`'s wind: `Weather_Construct` seeds it and `FUN_088f1570` runs it
//! each frame.
//!
//! Two noises drive it. One picks the heading, `2 pi` of swing at `0.06` Hz;
//! the other the strength, `WindRange` of swing at `0.15` Hz on top of
//! `WindBase`. Each is chased by a one-pole filter of `0.2` a tick. The wind
//! is `(cos a, 0, sin a) * strength` with the circuit's `DriftY` added to its
//! height - the fall - and every frame `Weather_Update` hands the env effect
//! `0.3` of it as its first modifier. Checked against Fort Gale live: heading
//! `-0.156`, strength `5.0 + -4.02`, `(0.9678, -10.0, -0.1525)`.

use oag_core::Rng;
use oag_core::math::Vec3;

use super::noise::Perlin;

/// The `0.3` `Weather_Update` scales the wind by before the effect sees it.
pub const MODIFIER_SCALE: f32 = 0.3;

/// The one-pole filter's rate, per tick (`node+0x58`, `node+0x74`).
const SMOOTHING: f32 = 0.2;

/// The heading noise: `{phase, 2, 0.06, 2 pi}` (`node+0x40`).
const HEADING_FREQUENCY: f32 = 0.06;

/// The strength noise: `{phase, 2, 0.15, WindRange}` (`node+0x5c`).
const STRENGTH_FREQUENCY: f32 = 0.15;

/// Both noises sum two octaves.
const OCTAVES: u32 = 2;

/// A circuit's wind.
#[derive(Debug, Clone)]
pub struct Wind {
    table: Perlin,
    heading_phase: f32,
    strength_phase: f32,
    base: f32,
    range: f32,
    drift_y: f32,
    heading: f32,
    strength: f32,
}

impl Wind {
    /// `WindBase`, `WindRange` and `DriftY` as the circuit authors them. The
    /// phases are `rand() & 0x3fff` in the original, drawn here from `rng`.
    pub fn new(base: f32, range: f32, drift_y: f32, rng: &mut Rng) -> Self {
        let table = Perlin::new(rng);
        let phase = |rng: &mut Rng| (rng.next_u32() & 0x3fff) as f32;
        Self {
            table,
            heading_phase: phase(rng),
            strength_phase: phase(rng),
            base,
            range,
            drift_y,
            heading: 0.0,
            strength: 0.0,
        }
    }

    /// One frame of `dt` seconds, `clock` seconds into the race.
    pub fn advance(&mut self, dt: f32, clock: f32) {
        let heading = self.table.fractal(
            clock,
            self.heading_phase,
            OCTAVES,
            HEADING_FREQUENCY,
            std::f32::consts::TAU,
        );
        let strength = self.table.fractal(
            clock,
            self.strength_phase,
            OCTAVES,
            STRENGTH_FREQUENCY,
            self.range,
        );
        // `int(dt / (1/60))` steps of the filter, so a long frame catches up.
        for _ in 0..(dt * 60.0) as u32 {
            self.heading += (heading - self.heading) * SMOOTHING;
            self.strength += (strength - self.strength) * SMOOTHING;
        }
    }

    /// The wind this frame, world units per tick, before [`MODIFIER_SCALE`].
    #[must_use]
    pub fn vector(&self) -> Vec3 {
        let speed = self.base + self.strength;
        Vec3::new(
            self.heading.cos() * speed,
            self.drift_y,
            self.heading.sin() * speed,
        )
    }

    /// What the env effect is handed: [`Self::vector`] times [`MODIFIER_SCALE`].
    #[must_use]
    pub fn modifier(&self) -> Vec3 {
        self.vector() * MODIFIER_SCALE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_matches_the_wind_read_off_fort_gale() {
        // heading -0.156, strength 5.0 + -4.02 as the original's two filters stood.
        let mut wind = Wind::new(5.0, 25.0, -10.0, &mut Rng::new(1));
        wind.heading = -0.156_328_39;
        wind.strength = -4.020_229_3;
        let live = wind.vector();
        assert!((live.x - 0.9678).abs() < 1e-3, "{live:?}");
        assert!((live.y - -10.0).abs() < 1e-6);
        assert!((live.z - -0.1525).abs() < 1e-3, "{live:?}");
        assert!((wind.modifier().y - -3.0).abs() < 1e-6);
    }

    #[test]
    fn the_filter_chases_the_noise_a_fifth_a_tick() {
        let mut wind = Wind::new(5.0, 25.0, -10.0, &mut Rng::new(3));
        wind.advance(1.0 / 60.0, 1.0);
        let one = wind.strength;
        assert!(one.abs() <= 25.0 * 0.2 * 1.5 + 1e-3, "{one}");
        for step in 0..300 {
            wind.advance(1.0 / 60.0, 1.0 + step as f32 / 60.0);
        }
        assert!(wind.strength.abs() <= 25.0 * 1.5);
    }
}
