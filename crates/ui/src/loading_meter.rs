//! The seven-triangle meter of Wipeout 2048's race load.
//!
//! `FrontEnd_DrawLoadingMeter` (`0x8105d4a8`) and its triangle
//! (`0x8105d1f0`), read in `docs/ghidra/functions/vita-2048-eu-v104/race-load.md`, "The box,
//! the bar and the meter". Seven triangles in a row, each a ring of three quads between an
//! outer and an inner triangle: hollow at the start, filling in from the left as the load
//! runs, each breathing on a 64-step cycle seven steps apart.

use crate::frontend::Draw;

/// Triangles in the row.
pub const COUNT: usize = 7;

/// The wrap of the breathing counter, in steps.
pub const CYCLE: u32 = 64;

/// Where the row is and how big its triangles are, in the layout's grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Meter {
    /// The centre of triangle 0, the right-most; the row runs left.
    pub at: [f32; 2],
    /// The distance between two centres.
    pub pitch: f32,
    /// The outer triangle's circumradius at full breath.
    pub outer: f32,
    /// The inner triangle's circumradius at full breath and no fill, before the `- 1`.
    pub inner: f32,
    /// How far an odd triangle sits lower than an even one.
    pub drop: f32,
}

/// Radians between a triangle's corners.
const CORNER: f32 = std::f32::consts::TAU / 3.0;

impl Meter {
    /// The centre of triangle `i`: `x = at.x - pitch i`, odd ones `drop` lower.
    #[must_use]
    pub fn centre(&self, i: usize) -> [f32; 2] {
        [
            self.at[0] - self.pitch * i as f32,
            self.at[1] + if i % 2 == 1 { self.drop } else { 0.0 },
        ]
    }

    /// How much of triangle `i` is still hollow, `0` solid to `1` hollow, at `fraction` of the
    /// load: `clamp((1 - fraction) * 7 - i, 0, 1)`.
    #[must_use]
    pub fn hollow(fraction: f32, i: usize) -> f32 {
        ((1.0 - fraction) * COUNT as f32 - i as f32).clamp(0.0, 1.0)
    }

    /// The breath of triangle `i` at step `counter`: the phase `(counter + 7 i) mod 64`
    /// ramps in over ten steps, holds, and ramps out over the last ten.
    #[must_use]
    pub fn pulse(counter: u32, i: usize) -> f32 {
        let phase = (counter + 7 * i as u32) % CYCLE;
        if phase < 10 {
            phase as f32 / 10.0
        } else if phase > 54 {
            (CYCLE - phase) as f32 / 10.0
        } else {
            1.0
        }
    }

    /// The corner `k` of a triangle of circumradius `radius` about `centre`. An even triangle
    /// points down, an odd one up (`a0` is `0` and a third of a half turn).
    fn corner(centre: [f32; 2], radius: f32, odd: bool, k: usize) -> [f32; 2] {
        let a0 = if odd {
            std::f32::consts::FRAC_PI_3
        } else {
            0.0
        };
        let angle = a0 + k as f32 * CORNER;
        let (sin, cos) = oag_core::math::sin_cos(angle);
        [centre[0] + radius * sin, centre[1] + radius * cos]
    }

    /// The row at `fraction` of the load and breathing step `counter`, in `color`.
    #[must_use]
    pub fn draw(&self, fraction: f32, counter: u32, color: [f32; 4]) -> Draw {
        let mut triangles = Vec::with_capacity(COUNT * 6);
        for i in 0..COUNT {
            let centre = self.centre(i);
            let odd = i % 2 == 1;
            let u = Self::pulse(counter, i);
            let outer = self.outer * u;
            let inner = self.inner * u * Self::hollow(fraction, i) - 1.0;
            for k in 0..3 {
                let o0 = Self::corner(centre, outer, odd, k);
                let o1 = Self::corner(centre, outer, odd, k + 1);
                let i0 = Self::corner(centre, inner, odd, k);
                let i1 = Self::corner(centre, inner, odd, k + 1);
                triangles.push([o0, o1, i1]);
                triangles.push([o0, i1, i0]);
            }
        }
        Draw::SolidTriangles { triangles, color }
    }
}

#[cfg(test)]
mod tests;
