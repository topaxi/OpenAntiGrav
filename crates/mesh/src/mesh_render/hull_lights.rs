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
//! - **The scale is applied per craft in the shader, from the nearest ring
//!   point.** The original's four scale bytes are per craft, blended out of
//!   the spline points it sits over (`oag_vex::track::SplinePoint::light_scale`;
//!   byte 0 scales the ambient, byte 1 the directionals - the tunnel authors
//!   `[127, 127, 255, 255]`). The rig here is built at scale `256` (the bare
//!   colour bytes) and `mesh.wesl` applies `(byte * scale) >> 8` per craft
//!   from `oag_race::Course::light_scale`, which is how a craft darkens in a
//!   tunnel. Which of the four bytes is which is read from the builder's two
//!   scale arguments (ambient, then directional); the point-light pair is not
//!   reproduced. The nearest ring point stands in for the original's blend at
//!   the craft's own spline parameter. A draw with no scale reads `255`, about
//!   2 % under the bare colours, which is what a craft over Talon's open road
//!   reads live (`250` or `251`).
//! - **Only a craft's hull takes it.** 16 of 113 lit-branch models carried a
//!   light list live; the other 97 were never rebuilt, so the GE drew them
//!   unlit. Which models those are is not mapped, so everything but the hull
//!   keeps the stand-in rig.
//! - The slots the directionals leave are the circuit's `wopoint`/`wospot` lights,
//!   chosen per craft: see [`super::HullPointLights`]. They reach the grid hull on Tech
//!   de Ra, The Amphiseum and Fort Gale, and none does on Talon's Junction.

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
    /// No rig: every title but Pulse and Pure on the PSP, and a circuit that authors
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
    pub fn from_track(data: &[u8], nodes: &[vex::Node], scale: u16) -> Option<Self> {
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
pub fn ge_channel(colour: f32, scale: u16) -> f32 {
    let byte = ((colour * 255.0) as u32).min(255);
    ((byte * u32::from(scale)) >> 8) as f32 / 255.0
}

/// A craft's two hull-light scale bytes as the one float `mesh.wesl` reads
/// from the model uniform: `ambient | directional << 8`, integer-valued so it
/// survives the trip exactly. `0.0` means the default `255` for both.
#[must_use]
pub fn pack_hull_scale(ambient: u8, directional: u8) -> f32 {
    f32::from(u16::from(ambient) | u16::from(directional) << 8)
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
    fn a_hull_scale_packs_to_an_exact_integer_float() {
        assert_eq!(pack_hull_scale(0, 0), 0.0);
        assert_eq!(pack_hull_scale(127, 250), (127 + 250 * 256) as f32);
    }

    #[test]
    fn the_bare_rig_darkens_to_the_tunnels_scale() {
        // 16_Track's first light, 255 bare, at the tunnel's 127: 127 * 255 >> 8.
        assert_eq!((ge_channel(1.0, 127) * 255.0).round() as u32, 126);
        assert_eq!((ge_channel(1.0, 256) * 255.0).round() as u32, 255);
    }

    #[test]
    fn off_is_disabled() {
        assert!(!HullLights::OFF.enabled());
    }
}
