//! The hull's point and spot lights: what fills the GE slots the circuit's
//! `DirectionalLight`s leave free.
//!
//! `SceneLight_BuildLightingList` (`0x0887a4f0`) gives the hull at most four
//! hardware lights: the circuit's directionals first, then - in the slots left -
//! the strongest of the `wopoint`/`wospot` nodes ([`oag_vex::world_lights`]) by
//! strength at the craft. Read live on Tech de Ra (three directionals, so one
//! slot): the fourth light was the `wopoint` 10.25 units ahead of and above the
//! grid, colour `(61, 142, 157)`, and it is what turns that hull from the
//! directionals' orange to blue-white (`docs/ghidra/functions/psp-pulse-usa/
//! scene-light.md`, "The point and spot lights").
//!
//! ```text
//! d        = |craft - light|
//! point    : 1                         d < inner
//!            t^2 (3 - 2t), t = (outer - d) / (outer - inner)   d <= outer
//!            0                         beyond
//! spot     : the point's strength x a cone term over the angle between the
//!            spot's axis and the direction light -> craft:
//!            1 inside cos(inner / 2), 0 outside cos(outer / 2), smoothstep between
//! colour   : trunc(c * 255 * strength) clamped to 255, then (byte * scale) >> 8,
//!            scale = the craft's third spline byte for a point, fourth for a spot
//! vector   : light - craft, handed to the GE as a *directional* light. The vector
//!            is unnormalised in the list; the reference renderer normalises it, and
//!            so does this (measured against the original's wings: within two levels
//!            of 255, where the unnormalised vector is 15-30 levels too bright).
//! ```
//!
//! The list is rebuilt from where the craft is, so this is a pure function of
//! the craft's position, its scale bytes and the scenery clock: a light under an
//! `Anim Transform` is re-placed every call, as the original re-tests it every
//! call (`scene-light.md`, "A light that moves").

use oag_vex::vex;
use oag_vex::world_lights::{self, PlacedLight, WorldLight};

/// The most point or spot lights a hull ever takes: the GE has four slots and
/// every shipped circuit authors two or three directionals.
pub const HULL_POINT_SLOTS: usize = 2;

/// One slot as the model uniform carries it: `xyz` the unit vector towards the
/// light, `w` the colour's three bytes packed `r | g << 8 | b << 16` as an
/// integer-valued float, `0.0` for an empty slot.
pub type HullPoint = [f32; 4];

/// A circuit's point and spot lights and how many hull slots the directionals
/// leave them.
#[derive(Debug, Clone, Default)]
pub struct HullPointLights {
    lights: Vec<PlacedLight>,
    slots: usize,
}

impl HullPointLights {
    /// None at all: every title and circuit without the rig.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// The lights a circuit's `track.vex` authors, with `directionals` of the
    /// four slots already taken.
    #[must_use]
    pub fn from_track(data: &[u8], nodes: &[vex::Node], directionals: usize) -> Self {
        Self::from_lights(world_lights::placed_lights(data, nodes), directionals)
    }

    /// A set of lights already placed, with `directionals` of the four slots taken.
    #[must_use]
    pub fn from_lights(lights: Vec<PlacedLight>, directionals: usize) -> Self {
        Self {
            lights,
            slots: 4usize.saturating_sub(directionals).min(HULL_POINT_SLOTS),
        }
    }

    /// How many lights the circuit authors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lights.len()
    }

    /// Whether the circuit authors none, or leaves no slot for one.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lights.is_empty() || self.slots == 0
    }

    /// The slots for a craft at `position` with the scenery clock at `seconds`
    /// (a light under an `Anim Transform` is where that clock has put it), and the
    /// craft's third and fourth spline scale bytes (`point`, `spot`).
    #[must_use]
    pub fn at(
        &self,
        position: [f32; 3],
        seconds: f32,
        [point, spot]: [u8; 2],
    ) -> [HullPoint; HULL_POINT_SLOTS] {
        let mut out = [[0.0; 4]; HULL_POINT_SLOTS];
        if self.is_empty() {
            return out;
        }
        let mut chosen: Vec<(WorldLight, f32)> = Vec::new();
        let mut weakest = f32::MAX;
        for placed in &self.lights {
            let light = placed.at(seconds);
            let s = strength(&light, position) * 255.0;
            if s.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
                || (chosen.len() == self.slots && weakest > s)
            {
                continue;
            }
            if chosen.len() < self.slots {
                chosen.push((light, s));
                weakest = weakest.min(s);
            } else if weakest < s {
                let at = chosen
                    .iter()
                    .enumerate()
                    .fold(
                        (f32::MAX, 0),
                        |best, (i, c)| if c.1 < best.0 { (c.1, i) } else { best },
                    )
                    .1;
                chosen[at] = (light, s);
                weakest = chosen.iter().fold(f32::MAX, |m, c| m.min(c.1));
            }
        }
        for (slot, (light, s)) in chosen.into_iter().enumerate() {
            let to = [
                light.position[0] - position[0],
                light.position[1] - position[1],
                light.position[2] - position[2],
            ];
            let length = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt();
            if length <= f32::EPSILON {
                continue;
            }
            let scale = u32::from(if light.cone.is_some() { spot } else { point });
            let byte = |c: f32| (((c * s) as u32).min(255) * scale) >> 8;
            let packed = byte(light.colour[0])
                | (byte(light.colour[1]) << 8)
                | (byte(light.colour[2]) << 16);
            out[slot] = [
                to[0] / length,
                to[1] / length,
                to[2] / length,
                packed as f32,
            ];
        }
        out
    }
}

/// `0..=1`: how much of `light` reaches `position`.
fn strength(light: &WorldLight, position: [f32; 3]) -> f32 {
    let from = [
        position[0] - light.position[0],
        position[1] - light.position[1],
        position[2] - light.position[2],
    ];
    let d2 = from[0] * from[0] + from[1] * from[1] + from[2] * from[2];
    let Some(cone) = light.cone else {
        return falloff(d2, light);
    };
    if d2 == 0.0 {
        return 0.0;
    }
    let s = falloff(d2, light);
    let d = d2.sqrt();
    let cos = (from[0] * cone.axis[0] + from[1] * cone.axis[1] + from[2] * cone.axis[2]) / d;
    let (inner, outer) = (
        (cone.inner_angle * 0.5).cos(),
        (cone.outer_angle * 0.5).cos(),
    );
    if cos < outer {
        0.0
    } else if cos > inner {
        s
    } else {
        let t = (outer - cos) / (outer - inner);
        t * t * (3.0 - 2.0 * t) * s
    }
}

/// The radial term shared by both classes.
fn falloff(d2: f32, light: &WorldLight) -> f32 {
    let (inner, outer) = (light.inner_radius, light.outer_radius);
    if d2 < inner * inner {
        1.0
    } else if d2 <= outer * outer {
        let t = (outer - d2.sqrt()) / (outer - inner);
        t * t * (3.0 - 2.0 * t)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests;
