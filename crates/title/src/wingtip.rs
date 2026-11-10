//! The wingtip contrails: two white ribbons a craft lays from its `wingtip` locators after a
//! speed-pad crossing, Wipeout HD's `ShipWingtipVortices` node and the `VortexLine` element it
//! feeds (`docs/ghidra/functions/ps3-hdfury-eu/wingtip-vortices.md`).
//!
//! A title that carries the class names its [`Wingtip`]; every other title leaves
//! [`crate::Title::wingtip`] `None` and draws nothing unless the player's setting lends another
//! source's (`oag_raceplay`'s `WingtipContrails`).

use crate::Origin;
use crate::weapons::FatLineSpec;

/// The craft speed to ribbon strength curve: nothing below `floor`, `(v - floor) * slope` up to
/// `ceiling`, `top` from there (units per second).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedRamp {
    pub floor: f32,
    pub ceiling: f32,
    pub slope: f32,
    pub top: f32,
}

impl SpeedRamp {
    /// The strength at forward speed `speed`.
    #[must_use]
    pub fn at(&self, speed: f32) -> f32 {
        if speed < self.floor {
            0.0
        } else if speed >= self.ceiling {
            self.top
        } else {
            (speed - self.floor) * self.slope
        }
    }
}

/// One title's wingtip contrail config.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wingtip {
    /// The ribbon's texture, an archive entry name folded as the title's other ribbons' are.
    pub texture: &'static str,
    /// The element's config block: widths, node life, `u` scale, chains, capacity, phase step.
    /// The radius only picks a direction here and is normalised away.
    pub line: FatLineSpec,
    /// Units per second a node drifts away from the wing tip.
    pub drift: f32,
    /// The ribbon's colour, `vBaseColour`'s RGB.
    pub colour: [f32; 3],
    /// Seconds the per-locator timer is refilled to while the trigger holds.
    pub refill: f32,
    pub speed: SpeedRamp,
    /// Elements in the ribbon pool, two a craft while it runs.
    pub pool: usize,
    /// Where this came from.
    pub origin: Origin,
}

impl Wingtip {
    /// The HD lineage's config block, `0x00921090` in HD's executable and byte for byte in
    /// 2048's (`docs/ghidra/functions/ps3-hdfury-eu/wingtip-vortices.md`): the numbers are the
    /// same wherever the class ships, only the texture entry's name and extension differ.
    #[must_use]
    pub const fn hd_lineage(texture: &'static str, origin: Origin) -> Self {
        Self {
            texture,
            line: FatLineSpec {
                width: [0.05, 1.5],
                lifetime: 0.7,
                u_per_unit: 0.025,
                chains: 3,
                capacity: 100,
                phase_step: 0.06,
                radius: 0.8,
                ramp: false,
            },
            drift: 2.5,
            colour: [1.0, 1.0, 1.0],
            refill: 1.3,
            speed: SpeedRamp {
                floor: 50.0,
                ceiling: 100.0,
                slope: 0.006,
                top: 0.3,
            },
            pool: 18,
            origin,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HD: Wingtip = Wingtip::hd_lineage("x", Origin::Measured);

    /// The speed ramp is `0` below 50, `(v - 50) * 0.006` to 100, `0.3` from there on.
    #[test]
    fn the_speed_ramp_has_its_floor_slope_and_ceiling() {
        assert_eq!(HD.speed.at(49.9), 0.0);
        assert_eq!(HD.speed.at(50.0), 0.0);
        assert!((HD.speed.at(75.0) - 0.15).abs() < 1e-6);
        assert_eq!(HD.speed.at(100.0), 0.3);
        assert_eq!(HD.speed.at(400.0), 0.3);
    }
}
