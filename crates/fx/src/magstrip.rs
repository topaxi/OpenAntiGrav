//! The HD-lineage magstrip arc wake: `MagstripWake`, one object per craft,
//! nine electric arcs between the craft and the strip it is flying over.
//!
//! Recovered from the PS4 build - `MagstripWake_Update` (`0x012e1260`) and
//! `MagstripWake_Draw` (`0x012e2770`), decompiled again 2026-10-05 - and checked
//! for shape against the PS3 `EBOOT.elf`'s `MagstripWake.cpp` (`0x0010a0c0`).
//! Evidence and the confidence of each claim:
//! `docs/ghidra/functions/ps4-omega-eu/ships-effects.md`, "2026-10-05,
//! magstrip-wire-hd lane". **PS4 static reading, 65; HD's own arc pool
//! (`0x002bbd60` update, `0x002bb530` spawn, `0x002bc7b0` draw, 2026-10-05,
//! magstrip-hd-measure lane) agrees on life, scale, spread, shed radius and the
//! `0.85` smoothing, and supplies the jitter scales (85).**
//!
//! Two halves, the split [`crate::beam`] uses: this file is the state and the
//! geometry with no `wgpu` in it, and [`crate::beam::Pipeline`] (built with
//! [`crate::beam::pipeline::Style::MAGSTRIP`]) is the GPU half.
//!
//! # What the original does
//!
//! - **Nine arc slots** of `0xa0` bytes. While the craft is over a strip
//!   (`W+0xa0`), each tick spawns `1 + rand() % 3` arcs into free slots (life
//!   `<= 0`); a slot that is already live is never touched. Live arcs age
//!   whether or not the craft is still over the strip, so an arc outlives the
//!   strip by at most its own life.
//! - **A spawned arc** gets a life of `0.2 + 0.9u`, a frame of the 8-by-8 atlas
//!   (`rand() % 64`, stepped by one a tick), a contact-quad scale of
//!   `3.5 + 3.5u`, a spread of `0.8`, and five jitter terms. Below speed `200`
//!   everything but the frame is blended toward slow-speed constants by
//!   `t = clamp((speed - 10) / 190, 0, 1)`.
//! - **Its end point** lies `3..22` units from the craft along the track (and
//!   across it), on the road: the track sample's lifted position plus `2.6`
//!   along `down`. The original walks its AI-track data to get there.
//! - **Per tick** each live arc ages by `dt`, and smooths its contact scale,
//!   body brightness and contact brightness as `x = fresh + 0.85 x`; one that
//!   has fallen behind the craft is shed with probability
//!   `clamp((|end - craft|^2 - 368.64) * 0.0086685, 0, 1)`.
//! - **The picture**: from `start` (0.4 ahead of the anchor toward the end, in
//!   the craft's plane) to `end`, a camera-facing strip `1.6` wide in six
//!   quads sampling one row of the atlas cell, `1/48` of `v` a quad; the middle
//!   five points are pushed along the strip's width by their jitter term. The
//!   last quad's far edge is dimmer (`0.3`). A diamond contact quad, `scale`
//!   wide along the track's forward and lateral axes, lies at `end`.
//!
//! # Chosen, not measured
//!
//! No confidence score on any of these:
//!
//! - **`rand()`'s range.** Every constant (`0.9 / 2^30`, `3.26e-9 = 3.5 / 2^30`,
//!   `2^-29` around a `-s` offset) is written for `[0, 2^30)`, so a uniform draw
//!   in `[0, 1)` is used. The draws come from the seeded [`Rng`], not libc.
//! - **The end point's placement.** Our spline stands in for the AI-track walk:
//!   `ahead` metres along it, then across the road, clamped to its width.
//! - **Vertex alpha and depth** (see [`crate::beam::pipeline::Style::MAGSTRIP`]):
//!   `0xb2` is multiplied into the fragment as `beam.wgsl` does; whether
//!   `MagStripArc_fp` does is unread.
//! - **Brightness above 1.** The original packs `(uint)(x * 255)` into three
//!   bytes unclamped; here it is clamped.
//!
//! Not drawn: the two speed-driven ribbons offset `+/- 0.28 * clamp(speed - 40,
//! 0, 120)`. Whether they are the trail-ribbon class is unestablished (55).

use oag_core::{Rng, math::Vec3};
use oag_mesh::mesh::GpuVertex;

/// Arc slots, `W+0x98`'s `0x600`-byte pool of `0xa0`-byte arcs.
pub const ARCS: usize = 9;

/// Quads in one arc's body: five jittered segments, then the one that ends on
/// the road.
pub const BODY_QUADS: usize = 6;

/// Vertices one wake can emit into the atlas batch (triangle list).
pub const BODY_VERTICES: usize = ARCS * BODY_QUADS * 6;

/// Vertices one wake can emit into the contact batch (triangle list).
pub const CONTACT_VERTICES: usize = ARCS * 6;

/// Half the body strip's width: `auVar73 * 0.8` in `MagstripWake_Draw`.
pub const HALF_WIDTH: f32 = 0.8;

/// How far from the anchor the strip starts, toward the end point.
pub const START_REACH: f32 = 0.4;

/// Vertex alpha, the fixed `0xb2`.
pub const ALPHA: f32 = 178.0 / 255.0;

/// The dim far edge of the last quad: `intensity * 76.5` against `* 255`.
pub const SOFT: f32 = 0.3;

/// How far along `down` from the lifted sample the end point sits (`2.6`).
pub const END_DROP: f32 = 2.6;

/// Per-tick smoothing: `x = fresh + 0.85 x`.
pub const DECAY: f32 = 0.85;

/// `|end - craft|^2` below which a rear arc is never shed (`19.2^2`).
pub const SHED_RADIUS_SQUARED: f32 = 368.64;

/// The shed probability's slope.
pub const SHED_SLOPE: f32 = 0.008_668_517;

/// Stands in for `kIntensity`, the shader constant `MagstripWake_Draw` writes
/// from `W+0x5f8` (`DAT_020e52a0 + 0x1e0`, a tuning block the executable's image
/// leaves zero). The vertex greys are multiplied by it. **Chosen, not measured**:
/// with the greys alone the arcs vanish into the lit road once the scene is
/// linearised, which the original's picture does not.
pub const INTENSITY: f32 = 3.0;

/// The five jitter scales: `0.4 + 0.6 * sin(k * pi / 4)` for `k = 0..=4`, a bell
/// that leaves the two ends of the arc nearly still and swings the middle.
///
/// **Measured, HD `EBOOT.elf`, confidence 85**: the two class initialisers
/// (`0x002bd750`, `0x002bde18`) store five `_FSin(arg) * 0.6 + 0.4` results at
/// `0x00ad8984..0x00ad8998`, with the arguments `0`, `pi/4`, `pi/2`, `3pi/4`,
/// `pi` read from TOC slots `0x6378`, `0x6434`, `0x6438`, `0x643c`, `0x6440`;
/// the arc update `0x002bbd60` reads the array term for term. The PS4's
/// `DAT_02134210..20`, zero in its image, is the same run-time table (not
/// read there). See `docs/ghidra/functions/ps3-hdfury-eu/magstrip-wake.md`,
/// "2026-10-05, magstrip-hd-measure lane".
pub const JITTER_SCALE: [f32; 5] = [0.4, 0.824_264_1, 1.0, 0.824_264_1, 0.4];

/// The atlas is eight cells square.
const CELL: f32 = 0.125;

/// One quad's share of a cell's height: six to a cell.
const QUAD_V: f32 = CELL / BODY_QUADS as f32;

/// A uniform draw in `[0, 1)` - see "`rand()`'s range" above.
fn unit(rng: &mut Rng) -> f32 {
    (rng.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
}

/// One arc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arc {
    /// Seconds left; `<= 0` is a free slot (`+0x40`).
    pub life: f32,
    /// Where it lands (`+0x60`).
    pub end: Vec3,
    /// Contact quad scale (`+0xd0`).
    pub scale: f32,
    /// Spread of the jitter draws (`+0x84`).
    pub spread: f32,
    /// Body brightness (`+0x88`).
    pub intensity: f32,
    /// Contact quad brightness (`+0xd4`).
    pub glow: f32,
    /// The five jitter terms (`+0x70..0x84`).
    pub jitter: [f32; 5],
    /// Atlas frame, `0..64` (`+0xd8`).
    pub frame: u32,
    /// The contact quad's two axes: the track's forward and lateral at `end`.
    pub axes: [Vec3; 2],
}

impl Arc {
    const FREE: Self = Self {
        life: 0.0,
        end: Vec3::ZERO,
        scale: 0.0,
        spread: 0.0,
        intensity: 0.0,
        glow: 0.0,
        jitter: [0.0; 5],
        frame: 0,
        axes: [Vec3::X, Vec3::Z],
    };
}

/// What the wake reads of its craft each tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Craft {
    /// The `arc_anchor_point` locator in world space.
    pub anchor: Vec3,
    /// The craft's own origin (the scene node's translation).
    pub position: Vec3,
    /// The craft's nose.
    pub forward: Vec3,
    /// The craft's up (the anchor matrix's second row).
    pub up: Vec3,
    /// Speed, in the units `body+0x4b8` holds.
    pub speed: f32,
    /// The track's forward against the craft's: `true` when it flies the
    /// course the right way round (`dot(forward, tangent) >= 0`), which decides
    /// whether arcs are thrown ahead of it or behind.
    pub with_track: bool,
    /// The craft's lateral offset from the track centreline.
    pub lateral: f32,
}

/// A point on the track a given distance from the craft.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// The lifted sample position.
    pub position: Vec3,
    /// Along the track.
    pub forward: Vec3,
    /// Across it.
    pub lateral: Vec3,
    /// Into the road.
    pub down: Vec3,
    /// Road width to the left of the centreline.
    pub half_width_left: f32,
    /// Road width to the right.
    pub half_width_right: f32,
}

/// One craft's wake.
#[derive(Debug, Clone)]
pub struct Wake {
    arcs: [Arc; ARCS],
    rng: Rng,
}

impl Wake {
    /// An empty wake whose draws come from `seed`.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            arcs: [Arc::FREE; ARCS],
            rng: Rng::new(seed),
        }
    }

    /// The arcs, live or free.
    #[must_use]
    pub fn arcs(&self) -> &[Arc; ARCS] {
        &self.arcs
    }

    /// How many arcs are live.
    #[must_use]
    pub fn live(&self) -> usize {
        self.arcs.iter().filter(|arc| arc.life > 0.0).count()
    }

    /// One tick of `MagstripWake_Update`.
    ///
    /// `active` is the over-the-strip flag (`W+0xa0`): it gates spawning only.
    /// `walk(ahead)` places a point `ahead` units along the track from the
    /// craft - negative for behind - or `None` where there is no track.
    pub fn advance(
        &mut self,
        dt: f32,
        active: bool,
        craft: &Craft,
        walk: &dyn Fn(f32) -> Option<Placement>,
    ) {
        if active {
            let spawns = 1 + self.rng.below(3);
            for _ in 0..spawns {
                let Some(slot) = self.arcs.iter().position(|arc| arc.life <= 0.0) else {
                    break;
                };
                if let Some(arc) = self.spawn(craft, walk) {
                    self.arcs[slot] = arc;
                }
            }
        }
        for slot in 0..ARCS {
            if self.arcs[slot].life > 0.0 {
                self.age(slot, dt, craft);
            }
        }
    }

    fn spawn(&mut self, craft: &Craft, walk: &dyn Fn(f32) -> Option<Placement>) -> Option<Arc> {
        let rng = &mut self.rng;
        let t = ((craft.speed - 10.0) * (1.0 / 190.0)).clamp(0.0, 1.0);
        let slow = 1.0 - t;
        let life = 0.2 + 0.9 * unit(rng);
        let lateral_draw = unit(rng) - 0.5;
        let mut ahead = 0.55 + unit(rng);
        if !craft.with_track {
            ahead = -ahead;
        }
        let mut scale = 3.5 + 3.5 * unit(rng);
        let mut spread = 0.8;
        let (reach_base, reach_span) = if craft.speed < 200.0 {
            ahead = (unit(rng) - 0.5) * slow + t * ahead;
            spread = t * spread + slow * 0.6;
            scale *= t * 0.5 + 0.5;
            (slow * 3.0 + t * 15.0, slow * 2.0 + t * 7.0)
        } else {
            (15.0, 7.0)
        };
        let direction = Vec3::new(lateral_draw, 0.0, ahead).normalize_or_zero();
        let reach = reach_base + unit(rng) * reach_span;
        let offset = direction * reach;
        let placed = walk(offset.z)?;
        let across =
            (craft.lateral + offset.x).clamp(-placed.half_width_left, placed.half_width_right);
        let end = placed.position + placed.lateral * across + placed.down * END_DROP;
        let intensity = 0.125 + 0.575 * unit(rng);
        let glow = 0.05 + 0.65 * unit(rng);
        let frame = rng.below(64);
        let mut jitter = [0.0; 5];
        for (term, scale) in jitter.iter_mut().zip(JITTER_SCALE) {
            *term = spread * (2.0 * unit(rng) - 1.0) * scale;
        }
        Some(Arc {
            life,
            end,
            scale,
            spread,
            intensity,
            glow,
            jitter,
            frame,
            axes: [placed.forward, placed.lateral],
        })
    }

    fn age(&mut self, slot: usize, dt: f32, craft: &Craft) {
        let rng = &mut self.rng;
        let arc = &mut self.arcs[slot];
        arc.life -= dt;
        arc.frame = (arc.frame + 1) % 64;
        arc.scale = 0.525 * unit(rng) + 0.525 + arc.scale * DECAY;
        arc.intensity = 0.0862 * unit(rng) + 0.01875 + arc.intensity * DECAY;
        arc.glow = 0.0975 * unit(rng) + 0.0075 + arc.glow * DECAY;
        let behind = arc.end - craft.position;
        if behind.dot(craft.forward) < 0.0 {
            let shed =
                ((behind.length_squared() - SHED_RADIUS_SQUARED) * SHED_SLOPE).clamp(0.0, 1.0);
            if unit(rng) <= shed {
                arc.life = 0.0;
                return;
            }
        }
        if arc.life > 0.0 {
            let spread = arc.spread;
            let mut fresh = |rng: &mut Rng| spread * (2.0 * unit(rng) - 1.0);
            let mut draw = fresh(rng);
            for index in 0..arc.jitter.len() {
                if index % 2 == 1 {
                    // HD's `0x002bbd60` pairs terms (0,1) and (2,3): the odd
                    // term reuses its neighbour's draw unless `rand() & 7` is
                    // zero, and the even terms start on a fresh draw.
                    if rng.next_u32() & 7 == 0 {
                        draw = fresh(rng);
                    }
                } else if index > 0 {
                    draw = fresh(rng);
                }
                arc.jitter[index] = arc.jitter[index] * DECAY + draw * 0.15 * JITTER_SCALE[index];
            }
        }
    }

    /// Appends this tick's geometry: `atlas` takes the arc bodies, `contact`
    /// the quads where they land. Both are triangle lists in world space, with
    /// the strips widened across the view from `eye`.
    pub fn build(
        &self,
        craft: &Craft,
        eye: Vec3,
        atlas: &mut Vec<GpuVertex>,
        contact: &mut Vec<GpuVertex>,
    ) {
        for arc in self.arcs.iter().filter(|arc| arc.life > 0.0) {
            body(arc, craft, eye, atlas);
            contact_quad(arc, contact);
        }
    }
}

fn vertex(position: Vec3, grey: f32, uv: [f32; 2]) -> GpuVertex {
    let grey = grey.clamp(0.0, 1.0) * INTENSITY;
    GpuVertex {
        position: position.to_array(),
        normal: [0.0, 0.0, 1.0],
        colour: [grey, grey, grey, ALPHA],
        texcoord: uv,
        lit: 0.0,
        ..bytemuck::Zeroable::zeroed()
    }
}

fn quad(out: &mut Vec<GpuVertex>, corners: [GpuVertex; 4]) {
    let [a, b, c, d] = corners;
    out.extend([a, b, c, a, c, d]);
}

fn body(arc: &Arc, craft: &Craft, eye: Vec3, out: &mut Vec<GpuVertex>) {
    let aim = arc.end - craft.anchor;
    let flat = (aim - craft.up * aim.dot(craft.up)).normalize_or_zero();
    let start = craft.anchor + flat * START_REACH;
    let across = (start - eye).cross(start - arc.end).normalize_or_zero() * HALF_WIDTH;
    let (u0, v0) = (
        (arc.frame % 8) as f32 * CELL,
        (arc.frame / 8) as f32 * CELL + CELL,
    );
    let mut previous = start;
    for index in 0..BODY_QUADS {
        let last = index == BODY_QUADS - 1;
        let next = if last {
            arc.end
        } else {
            let t = (index + 1) as f32 / BODY_QUADS as f32;
            start * (1.0 - t) + arc.end * t + across * arc.jitter[index]
        };
        let (near_v, far_v) = (v0 - index as f32 * QUAD_V, v0 - (index + 1) as f32 * QUAD_V);
        let far_grey = if last {
            arc.intensity * SOFT
        } else {
            arc.intensity
        };
        quad(
            out,
            [
                vertex(previous - across, arc.intensity, [u0, near_v]),
                vertex(previous + across, arc.intensity, [u0 + CELL, near_v]),
                vertex(next + across, far_grey, [u0 + CELL, far_v]),
                vertex(next - across, far_grey, [u0, far_v]),
            ],
        );
        previous = next;
    }
}

fn contact_quad(arc: &Arc, out: &mut Vec<GpuVertex>) {
    let [forward, lateral] = arc.axes;
    let at = |axis: Vec3, sign: f32| arc.end + axis * (sign * arc.scale);
    quad(
        out,
        [
            vertex(at(forward, -1.0), arc.glow, [0.0, 0.0]),
            vertex(at(lateral, -1.0), arc.glow, [1.0, 0.0]),
            vertex(at(forward, 1.0), arc.glow, [1.0, 1.0]),
            vertex(at(lateral, 1.0), arc.glow, [0.0, 1.0]),
        ],
    );
}

#[cfg(test)]
mod tests;
