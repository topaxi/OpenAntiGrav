//! Wipeout HD's Rocket smoke ribbon: a chain of world-space nodes, one pushed
//! per update while the rocket flies, each ageing over 1.85 s, swept into
//! three single-sided fins.
//!
//! Recovered from `RibbonEffects` pool 4 (`Ribbon_PushSample`
//! `0x002edc38`, `Ribbon_AgeSamples` `0x002ed7d8`,
//! `RibbonBuilder_StripSide` `0x002a47e8`, `RibbonBuilder_WriteVertexPair`
//! `0x002a4660`) and matched against a live RPCS3 dump: the alpha of all 695
//! interior nodes of five pauses is the law below to the byte. Evidence:
//! `docs/ghidra/functions/ps3-hdfury-eu/rocket-trail.md`.
//!
//! Unlike the engine trail ([`crate::exhaust::hd`]), the nodes stay where they
//! were laid and die on their own clock, so a ribbon outlives its rocket.
//!
//! Three inputs are **chosen, not measured**, and each is labelled where it
//! is used: the vertex colour's RGB (the original samples an unread lighting
//! volume, [`UNLIT_RGB`]), the jitter's distribution ([`JITTER`]), and the
//! pushed point (the rocket's own origin).

use std::collections::VecDeque;

use oag_core::math::Vec3;
use oag_core::rng::Rng;
use oag_mesh::mesh::GpuVertex;

/// How long a node lives, in seconds: config `0x00766c4c + 0x00`.
pub const LIFETIME: f32 = 1.85;

/// `u` per world unit of arc length: config `+0x04`.
pub const U_PER_UNIT: f32 = 0.03;

/// A node's half-width when it is laid: config `+0x08`. The dumped vertex
/// pair of the newest node is exactly twice this apart.
pub const HALF_WIDTH_BIRTH: f32 = 0.4;

/// A node's half-width as it dies: config `+0x0c`.
pub const HALF_WIDTH_DEATH: f32 = 2.0;

/// Nodes one ribbon draws at most: `RibbonBuilder_Alloc(140, 3)`. At 60
/// updates a second a ribbon holds about 111, so this never binds there.
pub const CAPACITY: usize = 140;

/// Fins per node: config `+0x14`, 60 degrees apart.
pub const FINS: usize = 3;

/// Ribbons alive at once: pool 4 holds nine elements. A rocket fired while
/// all nine are busy gets no ribbon, as the original's acquire fails.
pub const MAX_TRAILS: usize = 9;

/// The per-axis position jitter's amplitude: `rand() * 0.18`
/// (`DAT_008b487c`) added to each of x, y and z at the push.
///
/// **The scale is read; the distribution is chosen, not measured**: the
/// original's `rand()` is `(Libc_Rand() - 2^29) * 2^-29` and `Libc_Rand`'s
/// own range is unread, so this draws uniformly from `[-0.18, 0.18)`.
pub const JITTER: f32 = 0.18;

/// `constantAmbientColour` as the ribbon's draw patches it: `RibbonBuilder_Flush`
/// writes `(0.8, 0.8, 0.8, 1)` into engine parameter entry 10 itself, so the
/// circuit's own ambient never reaches the smoke.
pub const AMBIENT: f32 = 0.8;

/// The vertex colour's RGB where the original's comes from
/// `RibbonEffects_SampleLighting` (`0x002a4280`), a lookup into a lighting
/// volume this project has not read.
///
/// **Chosen, not measured.** White leaves the texture and [`AMBIENT`] to
/// decide the colour. The live values were `0xbaffff` near the grid down to
/// `0x2b5d7b` in shadow, so this is the bright end of what the original draws.
pub const UNLIT_RGB: [f32; 3] = [1.0, 1.0, 1.0];

/// Byte offset of the opacity table in `smoke_trails_opacity_ramp.tga`.
///
/// `RibbonEffects_LoadOpacityRamp` skips 0x14 bytes, two more than the TGA's
/// 18-byte header, so the engine's table starts at the image's third pixel and
/// ends on the footer's first two bytes. The live alphas match this offset and
/// not 18.
pub const RAMP_OFFSET: usize = 0x14;

/// The 256-entry alpha table a node's age indexes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpacityRamp([u8; 256]);

impl OpacityRamp {
    /// Reads the table the way the original does, or `None` if the file is
    /// too short to hold it.
    #[must_use]
    pub fn from_tga(bytes: &[u8]) -> Option<Self> {
        let table = bytes.get(RAMP_OFFSET..RAMP_OFFSET + 256)?;
        let mut out = [0u8; 256];
        out.copy_from_slice(table);
        Some(Self(out))
    }

    /// The table as authored, for a test or a report.
    #[must_use]
    pub fn table(&self) -> &[u8; 256] {
        &self.0
    }

    /// `OpacityRamp_Lookup` (`0x002c3788`): `table[(int)(x * 255.0)]`.
    #[must_use]
    pub fn lookup(&self, x: f32) -> u8 {
        let index = (f64::from(x) * 255.0) as i64;
        self.0[index.clamp(0, 255) as usize]
    }
}

/// One laid node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Node {
    pub position: Vec3,
    /// The rocket's row 0 when the node was laid.
    pub right: Vec3,
    /// The rocket's row 1.
    pub up: Vec3,
    /// Seconds left; drawn while above zero.
    pub life: f32,
    pub half_width: f32,
    /// `1 - t` at the last update, the ramp's index before the `* 255`. The
    /// alpha is looked up at draw time, so the ageing needs no ramp.
    pub fade: f32,
    /// Frozen at the node's first update; `None` until then.
    pub u: Option<f32>,
    pub rgb: [f32; 3],
}

/// One rocket's ribbon, oldest node first.
#[derive(Debug, Clone, Default)]
pub struct Trail {
    nodes: VecDeque<Node>,
}

impl Trail {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `Ribbon_PushSample`: lays a node at `position` with the rocket's basis,
    /// jittered by up to [`JITTER`] on each axis.
    pub fn push(&mut self, position: Vec3, right: Vec3, up: Vec3, rgb: [f32; 3], rng: &mut Rng) {
        let mut jitter = || (rng.next_f32() * 2.0 - 1.0) * JITTER;
        let offset = Vec3::new(jitter(), jitter(), jitter());
        self.nodes.push_back(Node {
            position: position + offset,
            right,
            up,
            life: LIFETIME,
            half_width: HALF_WIDTH_BIRTH,
            fade: 0.0,
            u: None,
            rgb,
        });
    }

    /// `Ribbon_AgeSamples`, oldest to newest. A node is freed when its life
    /// was already spent *before* this update, so one with a slightly
    /// negative life survives one update undrawn, as the dump shows.
    pub fn advance(&mut self, dt: f32) {
        self.nodes.retain(|node| node.life > 0.0);
        let rate = 1.0 / LIFETIME;
        let mut arc = 0.0f32;
        let mut previous: Option<Vec3> = None;
        for node in &mut self.nodes {
            let t = node.life * rate;
            node.life -= dt;
            node.half_width = t * (HALF_WIDTH_BIRTH - HALF_WIDTH_DEATH) + HALF_WIDTH_DEATH;
            node.fade = 1.0 - t;
            if let Some(previous) = previous {
                arc += (node.position - previous).length() * U_PER_UNIT;
            }
            if node.u.is_none() {
                node.u = Some(arc);
            }
            previous = Some(node.position);
        }
    }

    /// A node's alpha byte as the original's walk leaves it: the ramp at its
    /// fade, except that both ends of the whole chain - drawn or not - are
    /// cleared after the walk.
    #[must_use]
    pub fn alpha(&self, index: usize, ramp: &OpacityRamp) -> u8 {
        if index == 0 || index + 1 >= self.nodes.len() {
            return 0;
        }
        ramp.lookup(self.nodes[index].fade)
    }

    /// Whether every node has died, so the ribbon can be freed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The nodes, oldest first.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter()
    }

    /// Appends this ribbon's triangle list: newest node first, only nodes
    /// still alive, at most [`CAPACITY`] of them, and per segment per fin the
    /// `(a, a+1, b), (a+1, b, b+1)` pair `RibbonBuilder_Flush` indexes.
    pub fn extend_vertices(&self, out: &mut Vec<GpuVertex>, ramp: &OpacityRamp) {
        let drawn: Vec<(&Node, u8)> = self
            .nodes
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, node)| node.life > 0.0)
            .take(CAPACITY)
            .map(|(index, node)| (node, self.alpha(index, ramp)))
            .collect();
        for pair in drawn.windows(2) {
            let (a, b) = (
                fin_pairs(pair[0].0, pair[0].1),
                fin_pairs(pair[1].0, pair[1].1),
            );
            for fin in 0..FINS {
                let [a0, a1] = a[fin];
                let [b0, b1] = b[fin];
                out.extend_from_slice(&[a0, a1, b0, a1, b0, b1]);
            }
        }
    }
}

/// `RibbonBuilder_StripSide` for fin `k`: the side the fin spans and the
/// normal it faces, in the node's own rows (`right` = row 0, `up` = row 1).
///
/// **Read off the live vertex buffer**, three fins of one rocket against that
/// node's dumped rows: `side = cos * up + sin * right` and `normal = side x
/// forward` = `sin * up - cos * right`, at 0, 60 and 120 degrees. A scratch
/// interpreter run of the function printed the same set with row 0's sign
/// flipped; the dump is the original's own output, so it wins.
#[must_use]
pub fn fin_axes(k: usize, right: Vec3, up: Vec3) -> (Vec3, Vec3) {
    let angle = k as f32 * std::f32::consts::PI / FINS as f32;
    let (sin, cos) = angle.sin_cos();
    (up * cos + right * sin, up * sin - right * cos)
}

/// `RibbonBuilder_WriteVertexPair` for every fin of one node: `centre -
/// side * half_width` at `v = 1`, then `centre + side * half_width` at `v = 0`.
fn fin_pairs(node: &Node, alpha: u8) -> [[GpuVertex; 2]; FINS] {
    let colour = [
        node.rgb[0] * AMBIENT,
        node.rgb[1] * AMBIENT,
        node.rgb[2] * AMBIENT,
        f32::from(alpha) / 255.0,
    ];
    let u = node.u.unwrap_or(0.0);
    std::array::from_fn(|k| {
        let (side, normal) = fin_axes(k, node.right, node.up);
        let offset = side * node.half_width;
        let vertex = |position: Vec3, v: f32| GpuVertex {
            position: position.to_array(),
            normal: normal.to_array(),
            colour,
            texcoord: [u, v],
            ..bytemuck::Zeroable::zeroed()
        };
        [
            vertex(node.position - offset, 1.0),
            vertex(node.position + offset, 0.0),
        ]
    })
}

#[cfg(test)]
mod tests;
