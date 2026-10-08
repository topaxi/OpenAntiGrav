//! Wipeout HD's SPU vertex lights: the per-frame point-light list `EdgeGeom`
//! sums into every track chunk's `SpuVertexColours` stream.
//!
//! # What the original does
//!
//! Every frame the PPU collects up to 128 light candidates
//! (`SpuLight_AddCandidate`, `0x0040d990`), compacts the frustum survivors into
//! an 8-slot buffer (`SpuLight_CompactVisibleCandidates`), and hands the list
//! to the `EdgeGeom` SPU job with every track chunk it draws. Per vertex the
//! job computes
//!
//! ```text
//! sum over lights of  max(0, 1 - |light.pos - P| / D)^w * max(0, N . L) * colour
//! ```
//!
//! with `L` the unit vector from the vertex to the light, packs the sum as
//! RGBE, and the `SVC1` vertex program decodes it into the interpolant the
//! fragment program **adds to the diffuse irradiance before the albedo
//! multiply** - beside the ambient, the sun's `N.L` and the lightmap. Both
//! halves are read off the SPU binary and the shipped microcode:
//! `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "`EdgeGeom`'s light
//! path is read" (confidence 80) and "The `SVC1` combine is read" (88). The
//! RGBE round trip is exact to `256/255` (+0.4 %), which [`RGBE_ROUND_TRIP`]
//! carries and `mesh.wesl` applies.
//!
//! # What this renderer does with it
//!
//! [`SpuLights`] is the same list, in the same record layout - `(x, y, z, w,
//! r, g, b, D)` - bound as part of the `Scene` uniform so `mesh.wesl`'s vertex
//! stage can run the sum per vertex, as the SPU does. **Chosen, not measured**:
//! there is no per-chunk sphere cull on this side. The original's
//! `LightCulling` job is a performance gate - a light outside a chunk's
//! bounding sphere by more than `D` contributes exactly zero to every vertex
//! in it under the formula above - so running every light against every
//! vertex gives the same sum; with eight lights of range one to two units that
//! is cheap. The RGBE quantisation is not reproduced either: the sum is passed
//! as floats, so the only difference is the +0.4 % the decode's own `255`/
//! `128` literals introduce.
//!
//! # Who fills it
//!
//! `oag_raceplay::engine_light` (one light per craft behind its nozzle, the
//! one the live capture showed filling the buffer) and
//! `oag_raceplay::weapon_light` (the Missile explosion, the Rocket's draw and
//! the Bomb blast's two lights). The other producers renderer.md catalogued
//! (pickups, the Zone ship, wheel contacts) stay unwired until their triggers
//! are read.

/// The original's candidate-list cap: `SpuLight_AddCandidate` stops at
/// `0x80` records. The live buffer never held more than 8.
pub const MAX_SPU_LIGHTS: usize = 128;

/// The `SVC1` decode's own `rgb * 2^(a * 255 - 128)` against the packer's
/// `k + 129` exponent: the round trip lands at `256 / 255` of the input.
/// Measured (renderer.md, "The `SVC1` combine is read", confidence 88).
pub const RGBE_ROUND_TRIP: f32 = 256.0 / 255.0;

/// One light, in the record layout `SpuLight_AddCandidate` stores and the
/// SPU job reads: quad 1 `(x, y, z, w)`, quad 2 `(r, g, b, D)`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpuLight {
    /// World position, then `w`: the falloff exponent. `1.0` on every record
    /// the live capture held, and the SPU's fast path.
    pub position: [f32; 4],
    /// Linear light colour - the original's own magnitudes, `(40, 10, 4)` at
    /// rest for a Fury engine - then `D`: the range at which the falloff
    /// reaches zero, in world units.
    pub colour: [f32; 4],
}

impl SpuLight {
    /// A light that contributes nothing: zero colour, zero range.
    pub const NONE: Self = Self {
        position: [0.0; 4],
        colour: [0.0; 4],
    };
}

/// The per-frame list, as `shaders/types.wesl`'s `SpuLights` lays it out: a count in
/// the first lane of a padded word, then the fixed array.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpuLights {
    /// How many leading entries of `lights` are live. The other three lanes
    /// are padding to the array's 16-byte alignment.
    pub count: [u32; 4],
    /// The records, valid to `count`.
    pub lights: [SpuLight; MAX_SPU_LIGHTS],
}

impl SpuLights {
    /// No lights - what every draw outside a Wipeout HD race binds.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            count: [0; 4],
            lights: [SpuLight::NONE; MAX_SPU_LIGHTS],
        }
    }

    /// The first [`MAX_SPU_LIGHTS`] of `lights`, the rest dropped - the same
    /// cap the original's own producer stops at.
    #[must_use]
    pub fn from_slice(lights: &[SpuLight]) -> Self {
        let mut out = Self::none();
        let n = lights.len().min(MAX_SPU_LIGHTS);
        out.lights[..n].copy_from_slice(&lights[..n]);
        out.count[0] = n as u32;
        out
    }
}

const _: () = assert!(
    std::mem::size_of::<SpuLight>() == 32,
    "one record is two quadwords, as SpuLight_AddCandidate stores it"
);
const _: () = assert!(
    std::mem::size_of::<SpuLights>() == 16 + 32 * MAX_SPU_LIGHTS,
    "shaders/types.wesl's SpuLights is one padded count and the array"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_slice_counts_and_caps() {
        let one = SpuLight {
            position: [1.0, 2.0, 3.0, 1.0],
            colour: [40.0, 10.0, 4.0, 1.5],
        };
        let lights = SpuLights::from_slice(&[one]);
        assert_eq!(lights.count[0], 1);
        assert_eq!(lights.lights[0], one);
        assert_eq!(lights.lights[1], SpuLight::NONE);

        let many = vec![one; MAX_SPU_LIGHTS + 5];
        let capped = SpuLights::from_slice(&many);
        assert_eq!(capped.count[0] as usize, MAX_SPU_LIGHTS);
    }
}
