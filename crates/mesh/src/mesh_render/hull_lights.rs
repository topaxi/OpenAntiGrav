//! Pulse's hull lights: the GE hardware light list a PSP craft is drawn
//! under, built from the circuit's own `AmbientLight` and `DirectionalLight`
//! nodes.
//!
//! Read live on PPSSPP and reproduced from the disc to the unit - see
//! `docs/ghidra/functions/psp-pulse-usa/scene-light.md`.
//! `SceneLight_BuildLightingList` (`0x0887a4f0`) turns every `AmbientLight`
//! into one global ambient and every `DirectionalLight` (four at most) into a
//! diffuse-only GE light, each colour byte
//! `(min(255, trunc(c * 255)) * scale) >> 8`, and the hull batch's material
//! is white. The GE then lights each vertex:
//!
//! ```text
//! colour = clamp(ambient + sum(diffuse_i * max(0, N . L_i)), 0, 1)
//! ```
//!
//! and `MODULATE`s the texel by it. `mesh.wesl`'s `vs_main` does exactly that
//! for a lit vertex whenever [`HullLights::enabled`] is set, in place of the
//! stand-in rig and the invented grey a vertex without colour otherwise
//! carries.
//!
//! # What is ours
//!
//! - **The scale is `255`.** The original's four scale bytes are per craft,
//!   blended out of the spline points it sits over
//!   (`oag_vex::track::SplinePoint::light_scale`), and a race does not carry
//!   that blend to the renderer yet. `255` is what 455 of Talon's Junction's
//!   862 points author; a craft over them reads `250` or `251` live, so this
//!   is about 2 % bright there, and up to twice as bright in a tunnel that
//!   authors `127`. Chosen, not measured.
//! - **Only a craft's hull takes it.** 16 of 113 lit-branch models carried a
//!   light list live; the other 97 were never rebuilt, so the GE drew them
//!   unlit. Which models those are is not mapped, so everything but the hull
//!   keeps the stand-in rig.
//! - The point-light slots are not reproduced: none reach a hull on the grid.

use oag_vex::{lighting, vex};

/// How many hardware lights the GE has, and how many `DirectionalLight`s the
/// original collects (`World_CollectMarkerLists` caps the list at four).
pub const HULL_LIGHTS: usize = 4;

/// The GE light list, as `mesh.wesl` reads it: one global ambient and four
/// directional lights, colours already in `0..=1`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HullLights {
    /// The global ambient (`0x5c`), and `.w` = `1.0` when this rig is on.
    pub ambient: [f32; 4],
    /// Unit vector towards each light, `.w` = `1.0` when that light is on.
    pub direction: [[f32; 4]; HULL_LIGHTS],
    /// Each light's diffuse colour (`LDC`); `.w` unused.
    pub diffuse: [[f32; 4]; HULL_LIGHTS],
}

impl HullLights {
    /// No rig: every title but Pulse on the PSP, and a circuit that authors
    /// no light at all.
    pub const OFF: Self = Self {
        ambient: [0.0; 4],
        direction: [[0.0; 4]; HULL_LIGHTS],
        diffuse: [[0.0; 4]; HULL_LIGHTS],
    };

    /// Whether `mesh.wesl` lights a hull with this rig.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.ambient[3] > 0.5
    }

    /// The rig a circuit's `track.vex` authors, at the given scale.
    ///
    /// `None` when it authors neither light class, which the original meets
    /// with a flat grey ambient (`0x30 * scale >> 8`) and one fixed light that
    /// this does not reproduce.
    #[must_use]
    pub fn from_track(data: &[u8], nodes: &[vex::Node], scale: u8) -> Option<Self> {
        let ambients = lighting::ambient_lights(data, nodes);
        let directionals = lighting::directional_lights(data, nodes);
        if ambients.is_empty() && directionals.is_empty() {
            return None;
        }
        // The builder sums the colours as floats and converts once.
        let sum = ambients.iter().fold([0.0f32; 3], |acc, light| {
            [
                acc[0] + light.colour[0],
                acc[1] + light.colour[1],
                acc[2] + light.colour[2],
            ]
        });
        let mut out = Self::OFF;
        out.ambient = [
            ge_channel(sum[0], scale),
            ge_channel(sum[1], scale),
            ge_channel(sum[2], scale),
            1.0,
        ];
        for (slot, light) in directionals.iter().take(HULL_LIGHTS).enumerate() {
            // Row 2 of the node's world matrix is the vector the GE is handed,
            // matched live to three decimals on all three of 16_Track's.
            let row = [light.to_world[8], light.to_world[9], light.to_world[10]];
            let length = (row[0] * row[0] + row[1] * row[1] + row[2] * row[2]).sqrt();
            if length <= f32::EPSILON {
                continue;
            }
            out.direction[slot] = [row[0] / length, row[1] / length, row[2] / length, 1.0];
            out.diffuse[slot] = [
                ge_channel(light.colour[0], scale),
                ge_channel(light.colour[1], scale),
                ge_channel(light.colour[2], scale),
                0.0,
            ];
        }
        Some(out)
    }
}

/// One colour channel the way `SceneLight_BuildLightingList` packs it:
/// `min(255, trunc(c * 255))`, times the scale, shifted right by eight, and
/// read back by the GE as a byte.
#[must_use]
pub fn ge_channel(colour: f32, scale: u8) -> f32 {
    let byte = ((colour * 255.0) as u32).min(255);
    ((byte * u32::from(scale)) >> 8) as f32 / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_channel_is_packed_the_way_the_live_list_reads() {
        // 16_Track's first DirectionalLight, (1.0, 0.9365, 0.619) at scale 250,
        // read live as LDC0 = (249, 232, 153).
        let bytes = [1.0, 0.9365, 0.619].map(|c| (ge_channel(c, 250) * 255.0).round() as u32);
        assert_eq!(bytes, [249, 232, 153]);
    }

    #[test]
    fn off_is_disabled() {
        assert!(!HullLights::OFF.enabled());
    }
}
