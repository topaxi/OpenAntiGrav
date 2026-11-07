//! Wipeout HD's `FatLine` ribbon: chains of world-space nodes laid around an
//! anchor, each node ageing on its own clock, drawn as a camera-facing strip.
//! The Missile's two heads each lay one short straight stub and one helix.
//!
//! Recovered from `RibbonEffects` pools `+0x50` and `+0x68` (`FatLine_PushChains`
//! `0x002ab9d0`, `FatLine_Update` `0x002ab608`, `FatLine_AlphaOverLife`
//! `0x002ab410`) and the vertex program `FatLine_vp` (`0x0092ef00`). Evidence:
//! `docs/ghidra/functions/ps3-hdfury-eu/missile-ribbon.md`.
//!
//! Like [`crate::rocket_smoke`] the nodes stay where they were laid and die on
//! their own clock, so a ribbon outlives its missile. Unlike it, the strip's side
//! is built from the eye each frame (the vertex program's two cross products), so
//! [`Ribbon::extend_vertices`] takes the camera.
//!
//! **Chosen, not measured**, each labelled where used: the helix's handedness
//! ([`Ribbon::push`]), the vertex tint ([`TINT`]) and which strip side carries
//! `v = 1`.

use std::collections::VecDeque;
use std::f32::consts::PI;

use oag_core::math::Vec3;
use oag_mesh::mesh::GpuVertex;

use crate::rocket_smoke::OpacityRamp;

/// One element's config block, as the engine copies it out of its static table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spec {
    /// Half-width of a node when laid, and at death.
    pub width: [f32; 2],
    /// Seconds a node lives.
    pub lifetime: f32,
    /// Texture `u` per world unit of arc length (`FatLine_vp`'s `c[211].x`).
    pub u_per_unit: f32,
    /// Chains laid per push.
    pub chains: usize,
    /// Nodes a chain's draw buffer holds.
    pub capacity: usize,
    /// Phase added after each chain of a push, radians.
    pub phase_step: f32,
    /// Distance of a node from the anchor.
    pub radius: f32,
    /// Whether alpha reads the opacity ramp rather than `1 - age`.
    pub ramp: bool,
}

/// The vertex colour's RGB, `c[209]` of `FatLine_vp`. Its source is a global
/// vector `0x002ec748` negates and was not read; the texture (`ThickLine_Red`) is
/// red already.
///
/// **Chosen, not measured**: white.
pub const TINT: [f32; 3] = [1.0, 1.0, 1.0];

/// One laid node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Node {
    pub position: Vec3,
    /// Seconds left.
    pub life: f32,
    pub half_width: f32,
    /// `1 - t` at the last update: the alpha's argument.
    pub fade: f32,
    /// Running arc length from the chain's oldest node, rewritten each update;
    /// `None` until the first.
    pub u: Option<f32>,
}

/// One element: `spec.chains` chains of nodes, oldest first, and the phase the
/// next push starts at.
#[derive(Debug, Clone)]
pub struct Ribbon {
    spec: Spec,
    chains: Vec<VecDeque<Node>>,
    phase: f32,
}

impl Ribbon {
    #[must_use]
    pub fn new(spec: Spec) -> Self {
        Self {
            spec,
            chains: vec![VecDeque::new(); spec.chains],
            phase: 0.0,
        }
    }

    /// `FatLine_PushChains` with the Missile's `f1 = 1.0`: one node per chain,
    /// each `radius` from `position` along `reference` turned about `axis` by
    /// `pi / chains * i + phase`, the phase then stepping.
    ///
    /// `axis` is the anchor matrix's row 2 and `reference` its row 1. The turn is
    /// the Rodrigues rotation the vector code builds; its sine term's sign is
    /// **chosen, not measured**: `axis x reference` here, the right-handed turn.
    pub fn push(&mut self, position: Vec3, axis: Vec3, reference: Vec3) {
        let step = PI / self.spec.chains as f32;
        let across = axis.cross(reference);
        for (i, chain) in self.chains.iter_mut().enumerate() {
            let theta = step * i as f32 + self.phase;
            let (sin, cos) = theta.sin_cos();
            chain.push_back(Node {
                position: position + (reference * cos + across * sin) * self.spec.radius,
                life: self.spec.lifetime,
                half_width: self.spec.width[0],
                fade: 0.0,
                u: None,
            });
            self.phase += self.spec.phase_step;
        }
    }

    /// `FatLine_Update`, each chain oldest to newest. A node whose life was spent
    /// before this update is freed; the rest take their width and fade from the
    /// life they had, lose `dt`, and re-sum the arc length. Returns whether any
    /// node was walked, the element's "alive".
    pub fn advance(&mut self, dt: f32) -> bool {
        let rate = 1.0 / self.spec.lifetime;
        let [birth, death] = self.spec.width;
        let mut walked = false;
        for chain in &mut self.chains {
            chain.retain(|node| node.life > 0.0);
            let mut arc = 0.0f32;
            let mut previous: Option<Vec3> = None;
            for node in chain.iter_mut() {
                walked = true;
                let t = node.life * rate;
                node.half_width = t * (birth - death) + death;
                node.fade = 1.0 - t;
                node.life -= dt;
                if let Some(previous) = previous {
                    arc += (node.position - previous).length();
                }
                node.u = Some(arc);
                previous = Some(node.position);
            }
        }
        walked
    }

    /// Whether every node has died, so the element can be freed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chains.iter().all(VecDeque::is_empty)
    }

    /// Nodes alive across every chain.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.chains.iter().map(VecDeque::len).sum()
    }

    /// A node's alpha as the update leaves it: `FatLine_AlphaOverLife`, except that
    /// both ends of the chain are cleared after the walk.
    fn alpha(&self, chain: &VecDeque<Node>, index: usize, ramp: &OpacityRamp) -> f32 {
        if index == 0 || index + 1 >= chain.len() {
            return 0.0;
        }
        let fade = chain[index].fade;
        if self.spec.ramp {
            f32::from(ramp.lookup(fade)) / 255.0
        } else {
            fade
        }
    }

    /// Appends the triangle list of every chain's strip as `FatLine_vp` draws it.
    pub fn extend_vertices(&self, out: &mut Vec<GpuVertex>, eye: Vec3, ramp: &OpacityRamp) {
        for chain in &self.chains {
            let samples: Vec<(&Node, f32)> = chain
                .iter()
                .enumerate()
                .filter(|(_, node)| node.u.is_some())
                .take(self.spec.capacity)
                .map(|(index, node)| (node, self.alpha(chain, index, ramp)))
                .collect();
            let mut previous: Option<[GpuVertex; 2]> = None;
            for i in 0..samples.len().saturating_sub(1) {
                let (prev, cur) = (samples[i].0, samples[i + 1]);
                let next = samples.get(i + 2).map_or(cur.0, |sample| sample.0);
                let pair = self.pair(prev, cur.0, next, cur.1, eye);
                if let Some([l0, r0]) = previous {
                    let [l1, r1] = pair;
                    out.extend_from_slice(&[l0, r0, l1, r0, l1, r1]);
                }
                previous = Some(pair);
            }
        }
    }

    /// `FatLine_vp` for one strip pair: the node's two vertices, at `cur` plus and
    /// minus the screen-space side times its width.
    fn pair(&self, prev: &Node, cur: &Node, next: &Node, alpha: f32, eye: Vec3) -> [GpuVertex; 2] {
        let to_eye = eye - cur.position;
        let before = (eye - prev.position).cross(prev.position - cur.position);
        let after = to_eye.cross(cur.position - next.position);
        let side = (before + after).normalize_or_zero();
        let u = cur.u.unwrap_or(0.0) * self.spec.u_per_unit;
        let colour = [TINT[0], TINT[1], TINT[2], alpha];
        let vertex = |sign: f32| GpuVertex {
            position: (cur.position + side * (cur.half_width * sign)).to_array(),
            normal: side.to_array(),
            colour,
            // **Chosen, not measured**: the vertex program writes `v = max(side, 0)`
            // and the strip's order of its two sides is the array's, not read.
            texcoord: [u, sign.max(0.0)],
            ..bytemuck::Zeroable::zeroed()
        };
        [vertex(-1.0), vertex(1.0)]
    }
}

#[cfg(test)]
mod tests;
