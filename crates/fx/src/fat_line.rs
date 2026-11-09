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
//! Wipeout HD's wingtip contrails are the same element class with a different
//! push and alpha (`VortexLine`, `docs/ghidra/functions/ps3-hdfury-eu/wingtip-vortices.md`):
//! see [`Vortex`] and [`Ribbon::push_vortex`].
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

/// What turns a [`Ribbon`] into HD's `VortexLine` (the wingtip contrail's element): its nodes are
/// born at the anchor and drift, its chains fan over `pi` rather than circling the anchor, its
/// alpha is `t * strength` with the strength rewritten by every push, and two nodes at each end of
/// a chain are clear (`VortexLine_PushChains` `0x002ac208`, `VortexLine_Update` `0x002ac0f0`,
/// `VortexLine_AlphaOverLife` `0x002abfb8`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vortex {
    /// Units per second a node moves away from the anchor along its chain's direction
    /// (`|node+0x40|`, read `2.500` live).
    pub drift: f32,
    /// The vertex colour's RGB (`vBaseColour`, white).
    pub colour: [f32; 3],
}

/// One laid node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Node {
    pub position: Vec3,
    /// World units per second the node moves by; zero for every element but a [`Vortex`].
    pub velocity: Vec3,
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
    /// `Some` for a `VortexLine`: the config and the element-wide opacity scale the last push
    /// left (`element+0x60`).
    vortex: Option<(Vortex, f32)>,
}

impl Ribbon {
    #[must_use]
    pub fn new(spec: Spec) -> Self {
        Self {
            spec,
            chains: vec![VecDeque::new(); spec.chains],
            phase: 0.0,
            vortex: None,
        }
    }

    /// An element of the wingtip contrail's class (`VortexLine_Construct` `0x002ac070`, strength
    /// starting at `1.0`).
    #[must_use]
    pub fn new_vortex(spec: Spec, vortex: Vortex) -> Self {
        Self {
            vortex: Some((vortex, 1.0)),
            ..Self::new(spec)
        }
    }

    /// `VortexLine_PushChains`: one node per chain, born **at** `position`, each drifting at
    /// `vortex.drift` along `-normalize(cos(theta) * row1 + sin(theta) * (row2 x row1))` with
    /// `theta = pi / chains * i + phase`, the phase loaded once for the push (a fan over `pi`,
    /// not a circle) and then stepping `phase_step` per chain. `left` mirrors the spin
    /// (`+phase` against `-phase`). `strength` becomes the whole element's opacity scale.
    ///
    /// The rows are the anchor matrix's as stored, **with the craft's scale in them**: the
    /// skew term `row2 x row1` carries it squared, which is what spaces the fan.
    ///
    /// # Panics
    ///
    /// If the ribbon was not built by [`Self::new_vortex`].
    pub fn push_vortex(
        &mut self,
        position: Vec3,
        row1: Vec3,
        row2: Vec3,
        left: bool,
        strength: f32,
    ) {
        let (vortex, scale) = self.vortex.as_mut().expect("a vortex element");
        *scale = strength;
        let step = PI / self.spec.chains as f32;
        let across = row2.cross(row1);
        let phase = if left { self.phase } else { -self.phase };
        for (i, chain) in self.chains.iter_mut().enumerate() {
            let theta = step * i as f32 + phase;
            let (sin, cos) = theta.sin_cos();
            let direction = (row1 * cos + across * sin).normalize_or_zero();
            chain.push_back(Node {
                position,
                velocity: direction * -vortex.drift,
                life: self.spec.lifetime,
                half_width: self.spec.width[0],
                fade: 0.0,
                u: None,
            });
            self.phase += self.spec.phase_step;
        }
    }

    /// `FatLine_PushChains`: one node per chain, each `radius` from `position` along
    /// `reference` turned about `axis` by `pi / chains * i + phase`, the phase then
    /// stepping. A node lives `scale * spec.lifetime`; the Missile's ribbons pass
    /// `f1 = 1.0` and the explosion's streaks `U(0.5, 1)`. `advance` still divides by
    /// `spec.lifetime`, so a node is born at `t = scale`, narrower and fainter than a
    /// full-life one (`missile-explosion.md`, "How a streak is drawn").
    ///
    /// `axis` is the anchor matrix's row 2 and `reference` its row 1. The turn is
    /// the Rodrigues rotation the vector code builds; its sine term's sign is
    /// **chosen, not measured**: `axis x reference` here, the right-handed turn.
    pub fn push(&mut self, position: Vec3, axis: Vec3, reference: Vec3, scale: f32) {
        let step = PI / self.spec.chains as f32;
        let across = axis.cross(reference);
        for (i, chain) in self.chains.iter_mut().enumerate() {
            let theta = step * i as f32 + self.phase;
            let (sin, cos) = theta.sin_cos();
            chain.push_back(Node {
                position: position + (reference * cos + across * sin) * self.spec.radius,
                velocity: Vec3::ZERO,
                life: self.spec.lifetime * scale,
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
                node.position += node.velocity * dt;
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
    fn alpha(&self, chain: &VecDeque<Node>, index: usize, ramp: Option<&OpacityRamp>) -> f32 {
        if index == 0 || index + 1 >= chain.len() {
            return 0.0;
        }
        let fade = chain[index].fade;
        if let Some((_, strength)) = self.vortex {
            // `VortexLine_Update` clears the second node from each end as well.
            if index == 1 || index + 2 >= chain.len() {
                return 0.0;
            }
            return (1.0 - fade) * strength;
        }
        match ramp {
            Some(ramp) if self.spec.ramp => f32::from(ramp.lookup(fade)) / 255.0,
            _ => fade,
        }
    }

    /// Appends the triangle list of every chain's strip as `FatLine_vp` draws it.
    pub fn extend_vertices(&self, out: &mut Vec<GpuVertex>, eye: Vec3, ramp: &OpacityRamp) {
        self.extend_with(out, eye, Some(ramp));
    }

    /// [`Self::extend_vertices`] for an element that reads no opacity ramp, a [`Vortex`].
    pub fn extend_vortex_vertices(&self, out: &mut Vec<GpuVertex>, eye: Vec3) {
        self.extend_with(out, eye, None);
    }

    fn extend_with(&self, out: &mut Vec<GpuVertex>, eye: Vec3, ramp: Option<&OpacityRamp>) {
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
        let [red, green, blue] = self.vortex.map_or(TINT, |(vortex, _)| vortex.colour);
        let colour = [red, green, blue, alpha];
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
