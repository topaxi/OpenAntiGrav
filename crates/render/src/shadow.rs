//! The `blob` shadow tier: one ground-aligned quad per craft.
//!
//! Step 3 of [`docs/rendering/shadows.md`](../../../docs/rendering/shadows.md)'s
//! plan, and only that tier. `original` - Pulse's `Dynamic Shadow Occluder`
//! hulls, HD's shadow-map jobs - is step 5 and nothing here is it.
//!
//! # What is the disc's and what is ours
//!
//! **The shape is the disc's on Wipeout HD.** Nine `ambient_shadow.gtf`, one
//! per team, 128x64, single-channel, decoded already
//! ([`gtf.md`](../../../docs/formats/gtf.md)); read in Morton order each is
//! that team's craft in soft silhouette. This module samples that texture and
//! nothing else on a title that ships one.
//!
//! **The falloff is ours, and only where a title ships no such asset**, which
//! is every title but HD. That is the narrow case `CLAUDE.md`'s never-invent
//! rule allows - a substitute for the missing asset alone. It is never an
//! override: a title *with* the asset never sees it, and the whole tier is
//! off by default.
//!
//! **Also ours, and each says so where it is written**: the height fade
//! ([`fade`]), the lift off the surface ([`LIFT`]), and where in the frame the
//! quad is drawn. No title in the lineage draws a blob at all - `blob` `0x3e0`
//! and `textureBlob` `0x3df` are authored zero times across all 415 `.vex`
//! files on the Pulse disc - so there is no original behaviour to be faithful
//! to here, and none is claimed.
//!
//! # Two halves, deliberately
//!
//! [`quad`] and [`fade`] are pure arithmetic with tests, the same split
//! [`oag_fx::exhaust`] and [`crate::shield`] use: the placement can be wrong in
//! a way a screenshot does not show, so the part that decides where a shadow
//! goes is testable on a machine with no graphics driver.

use oag_core::math::{Mat4, Vec3};
use oag_vex::shadow_occluder::Occluder;

use oag_mesh::mesh::GpuVertex;

/// How far off the surface the quad is lifted, in world units.
///
/// **Ours, and a fudge rather than a reading**: a shadow laid on the track
/// z-fights the ribbon it is drawn over. Small against a craft (the hulls run
/// 4-6 units long) and large against the depth buffer's resolution at race
/// distances, which is the whole window it has to sit in.
///
/// **0.15, from 0.05 on 2026-10-05, and measured against the picture.** The
/// road the player sees stands a little proud of the collision floor the
/// shadow is cast onto, by different amounts along the circuit. At 0.05, in
/// a Zone capture of Pulse's default circuit late in a lap, the shadow lost up
/// to 70 % of its pixels on single ticks (3102 changed pixels against a steady
/// 10-11 thousand) and was back the tick after; at 0.15 the dips are gone
/// (10610 at worst), and 0.3 and 0.6 look the same, so this is the smallest of
/// those tried. See `docs/rendering/shadows.md`, "The shadow blinked".
///
/// Applied along the *surface normal*, not world up, so it keeps its meaning
/// on a banked corner and inside a loop.
pub const LIFT: f32 = 0.15;

/// The largest number of quads [`Pipeline`] holds: one per grid slot.
///
/// Eight, matching `oag_gameplay::MAX_SHIPS`. Stated as a literal rather than
/// imported because this crate depends on no gameplay crate and must not - see
/// `just check-deps`.
pub const MAX_QUADS: usize = 8;

/// Six vertices per quad, two triangles, as [`quad`] builds them.
pub const MAX_QUAD_VERTICES: usize = MAX_QUADS * 6;

/// The longest ring on the Pulse disc, rounded up.
///
/// Measured: a hull's silhouette ring runs 3 to 39 vertices across all 129
/// hulls and six directions (`shadow_occluder_records.rs`).
const MAX_RING: usize = 40;

/// At most two rings per craft - `Data.wad#597` `shadow_lodShape` is the one
/// hull that gives two, and nothing on the disc gives three.
const MAX_RINGS_PER_CRAFT: usize = 2;

/// Room for every craft's projected hull: a ring fans into one triangle per
/// edge.
pub const MAX_HULL_VERTICES: usize = MAX_QUADS * MAX_RINGS_PER_CRAFT * MAX_RING * 3;

/// What [`Pipeline`]'s one vertex buffer holds: the quads, then the hulls.
pub const MAX_VERTICES: usize = MAX_QUAD_VERTICES + MAX_HULL_VERTICES;

/// How far above its own ride height a craft keeps any shadow at all, as a
/// multiple of that height.
///
/// **Ours.** A craft in normal flight hovers at its ride height and its shadow
/// has to be at full strength there, so the fade cannot start at the ground -
/// measured on Talon's Junction, a resting craft sits `4.00` units over the
/// floor against a `4.125` spring target, which under a fade that started at
/// zero left it at strength `0.030`: drawn, and invisible. Three ride heights
/// is where a craft is unambiguously airborne rather than hovering.
pub const FADE_REACH: f32 = 3.0;

/// How dark a craft's shadow is at `height` above the surface, given the
/// `ride` height its own hover spring holds it at.
///
/// **Ours, both halves.** Full strength anywhere up to `ride` - which is where
/// a craft in normal flight lives, so this is the case a player sees almost
/// always - then linear to nothing at `ride * FADE_REACH`, so a craft thrown
/// off a jump loses its shadow smoothly instead of it vanishing on the tick
/// the cast misses. Nothing in any title's binary says what a blob should do
/// here, because no title draws one.
#[must_use]
pub fn fade(height: f32, ride: f32) -> f32 {
    if ride <= 0.0 {
        return 0.0;
    }
    let airborne = (height - ride).max(0.0);
    (1.0 - airborne / (ride * (FADE_REACH - 1.0))).clamp(0.0, 1.0)
}

/// Where one craft's shadow goes and how dark it is.
///
/// Built by the caller, which is the side that owns the collision world and
/// can cast the ray - see `oag_raceplay::shadow`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// The point on the surface the craft's downward cast hit, **before**
    /// [`LIFT`] is applied.
    pub contact: Vec3,
    /// The hit surface's normal, normalised. The quad lies in its plane.
    pub normal: Vec3,
    /// The craft's own forward axis. Projected into the surface plane to give
    /// the quad's long axis, so a shadow turns with the craft rather than with
    /// the camera.
    pub forward: Vec3,
    /// Half the hull's length, along the projected [`Self::forward`].
    pub half_length: f32,
    /// Half the hull's width, across it.
    pub half_width: f32,
    /// [`fade`]'s result: how dark this shadow is, `0.0..=1.0`.
    pub strength: f32,
    /// Which of [`Pipeline`]'s silhouettes this craft wears, as an index into
    /// the slice the pipeline was built with.
    pub silhouette: usize,
}

/// Two triangles laid on the surface under one craft.
///
/// The quad's axes come from the *surface*, not the camera: `forward` is
/// projected into the hit plane and the cross product completes the basis, so
/// a craft on a banked corner casts a shadow that lies in the road rather than
/// standing up out of it.
///
/// **The texture is mapped to the hull's own footprint**, whatever aspect that
/// is. On Wipeout HD that stretches a 128x64 image over a rectangle whose
/// length/width ratio is the craft's `<Misc>` dimensions and not 2:1 - a
/// deliberate choice rather than an oversight: the silhouette was authored for
/// the hull it belongs to, so the hull's own numbers are what it should cover.
/// Fitting the quad to the texture's aspect instead would size a craft's
/// shadow from an image dimension, which is not a fact about the craft.
///
/// A degenerate basis - `forward` parallel to `normal`, which a craft standing
/// exactly on its nose would give - falls back to any perpendicular axis
/// rather than producing NaNs.
#[must_use]
pub fn quad(placement: &Placement) -> [GpuVertex; 6] {
    let normal = placement.normal.normalize_or_zero();
    let projected = placement.forward - normal * placement.forward.dot(normal);
    let forward = if projected.length_squared() > 1e-8 {
        projected.normalize()
    } else {
        // Any axis in the plane will do: with the craft pointing straight into
        // the ground there is no meaningful heading to align to, and a NaN
        // basis would put the whole quad at the origin.
        normal.any_orthonormal_vector()
    };
    let right = forward.cross(normal).normalize_or_zero();
    let centre = placement.contact + normal * LIFT;
    let long = forward * placement.half_length;
    let across = right * placement.half_width;

    let corner = |sx: f32, sy: f32, u: f32, v: f32| GpuVertex {
        position: (centre + across * sx + long * sy).to_array(),
        normal: normal.to_array(),
        // Black, with the fade in the alpha: the shader returns black and the
        // alpha-over blend turns it into `dst * (1 - a)`. Only the alpha is
        // read - see `shadow.wgsl`.
        colour: [0.0, 0.0, 0.0, placement.strength],
        texcoord: [u, v],
        // Not lit by the mesh rig: a shadow is not a surface.
        lit: 0.0,
        ..bytemuck::Zeroable::zeroed()
    };
    // `v` runs with the craft's forward axis, so the silhouette's nose points
    // where the craft does - the textures are authored nose-up.
    let bl = corner(-1.0, -1.0, 0.0, 1.0);
    let br = corner(1.0, -1.0, 1.0, 1.0);
    let tl = corner(-1.0, 1.0, 0.0, 0.0);
    let tr = corner(1.0, 1.0, 1.0, 0.0);
    [bl, br, tl, br, tr, tl]
}

/// How dark a projected hull is drawn, before [`fade`] scales it.
///
/// **Ours, and the one number in this tier with no evidence behind it.** The
/// hull, its silhouette and the direction it projects along are all the
/// original's; what a stencil shadow volume is *darkened by* is decided by the
/// pass that fills it, and that pass has not been read. A hull rasterized at
/// full alpha is a black hole in the road rather than a shadow - the first
/// capture of this tier showed exactly that - so this is a number picked to
/// read as a shadow. **It is not derived from anything**: HD's own
/// `ambient_shadow.gtf` peaks at `212/255`, but that is a different title's
/// asset for a different mechanism and is not evidence for this one.
pub const HULL_DARKNESS: f32 = 0.35;

/// One craft's authored hull, and where its shadow lands.
///
/// The `original` tier's counterpart to [`Placement`], and deliberately a
/// separate type: a blob is a texture on a rectangle and this is geometry
/// projected onto a plane, with none of the same inputs.
#[derive(Debug, Clone, Copy)]
pub struct Cast<'a> {
    /// The hull, in its own model space -
    /// `Data\Ships\<Team>\Ship.vex`'s `Dynamic Shadow Occluder` node.
    pub hull: &'a Occluder,
    /// Model to world for the craft the hull belongs to.
    pub model: Mat4,
    /// The **local** axis to project along, before [`Self::model`] rotates it:
    /// `oag_pulse::shadow::AUTHORED_AXIS` on Pulse.
    ///
    /// A local axis and not a light, which is what the original does - see
    /// that constant, and `shadow-occluder.md` for the read.
    pub axis: Vec3,
    /// Where the craft's own downward cast hit the surface.
    pub contact: Vec3,
    /// That surface's normal.
    pub normal: Vec3,
    /// [`fade`]'s result, as for a blob.
    pub strength: f32,
}

/// Fans one craft's projected hull into triangles, appending to `out`.
///
/// **What the original does and what this does instead, stated plainly.**
/// `Shadow_RenderOccluderVolume` extrudes a stencil shadow volume from the
/// same silhouette and lets the stencil test decide what is inside it. This
/// rasterizes the volume's *ground cap* directly: the silhouette projected
/// along the same direction onto the plane the craft's own downward cast
/// found. For an attached occluder the original's own far cap is a ground
/// plane too - it reads a height off the parent and divides by the direction's
/// vertical component rather than casting a ray - so the polygon is the same
/// one. **What it loses is shadowing on anything that is not that plane**: a
/// craft passing under a bridge does not darken the bridge, and one craft does
/// not shadow another.
///
/// Returns how many rings it drew. Zero means the shadow would have been cast
/// *away* from the surface - up a wall, or behind the craft on a near-vertical
/// plane - which is skipped rather than drawn folded over.
pub fn hull_triangles(cast: &Cast, out: &mut Vec<GpuVertex>) -> usize {
    let normal = cast.normal.normalize_or_zero();
    // The direction in world space: the authored local axis through the
    // craft's own rotation, as `Shadow_RenderOccluderVolume` does it.
    let direction = cast.model.transform_vector3(cast.axis).normalize_or_zero();
    let facing = direction.dot(normal);
    // Parallel to the surface, or pointing away from it: there is no
    // intersection to draw, and forcing one puts a shadow behind the craft.
    if facing > -1e-3 {
        return 0;
    }
    let mut rings = 0;
    for ring in cast.hull.outline(cast.axis.to_array()) {
        let projected: Vec<Vec3> = ring
            .iter()
            .map(|slot| {
                let local = cast
                    .hull
                    .vertices
                    .get(usize::from(*slot))
                    .copied()
                    .unwrap_or([0.0; 3]);
                let world = cast.model.transform_point3(Vec3::from_array(local));
                // Where the ray from this vertex meets the contact plane.
                let travel = (cast.contact - world).dot(normal) / facing;
                world + direction * travel
            })
            .collect();
        if projected.len() < 3 || projected.iter().any(|p| !p.is_finite()) {
            continue;
        }
        let lift = normal * LIFT;
        let vertex = |at: Vec3| GpuVertex {
            position: (at + lift).to_array(),
            normal: normal.to_array(),
            colour: [0.0, 0.0, 0.0, cast.strength * HULL_DARKNESS],
            // Centre of the solid texture: coverage is 1 everywhere on it, so
            // the shape is the polygon rather than an image. See
            // `Pipeline::new`.
            texcoord: [0.5, 0.5],
            lit: 0.0,
            ..bytemuck::Zeroable::zeroed()
        };
        // **Ear clipping, not a fan**, and the difference is the whole picture.
        // A craft's silhouette is not convex - it has a long thin nose whose
        // outline doubles back on itself as a sliver - so a fan from the ring's
        // centroid produces overlapping and inverted triangles. Measured on
        // Assegai's own hull: a 22-edge ring enclosing 27 square units drew as
        // a crumpled star a couple of units across. Ear clipping fills exactly
        // the ring's interior, once.
        let (right, up) = (
            normal.any_orthonormal_pair().0,
            normal.any_orthonormal_pair().1,
        );
        let flat: Vec<[f32; 2]> = projected
            .iter()
            .map(|point| [point.dot(right), point.dot(up)])
            .collect();
        for [a, b, c] in ear_clip(&flat) {
            out.push(vertex(projected[a]));
            out.push(vertex(projected[b]));
            out.push(vertex(projected[c]));
        }
        rings += 1;
    }
    rings
}

/// Lays a projected hull on the road that is actually under it, rather than on
/// the single plane the craft's own cast found.
///
/// [`hull_triangles`] projects every vertex onto one plane, the plane of the
/// one collision triangle the craft's downward ray hit. A hull is several
/// units across and a road is not flat, so away from that triangle the plane
/// stands off the surface, and **which** triangle the ray hit changes tick to
/// tick as the craft crosses seams. Measured on Pulse's default circuit in
/// Zone, a point of the polygon ended up buried under the road by up to 0.7
/// units - against a [`LIFT`] of 0.05 - on more than a third of the ticks of
/// a late-lap window, and by nothing on the ticks between, which is a shadow
/// whose edge blinks.
///
/// `floor` answers "how far above this point, along the surface normal, is the
/// road?". Each triangle from `first` on is split once at its edge midpoints,
/// because a flat triangle over a concave road is buried between its own
/// vertices however well they are placed (a fitted triangle still measured
/// 0.12 buried at its centre), and every vertex is then moved to [`LIFT`]
/// above the road. A point `floor` has no answer for stays where it was; how
/// far `floor` looks is its own bound, so that a deck overhead is not mistaken
/// for the road.
///
/// A shared edge's midpoint is the same point in both triangles it belongs to,
/// so the surface it makes has no crack.
pub fn conform_to_floor(
    vertices: &mut Vec<GpuVertex>,
    first: usize,
    mut floor: impl FnMut(Vec3, Vec3) -> Option<f32>,
) {
    let coarse: Vec<GpuVertex> = vertices.drain(first..).collect();
    let mut place = |vertex: GpuVertex| {
        let at = Vec3::from_array(vertex.position);
        let normal = Vec3::from_array(vertex.normal);
        match floor(at, normal) {
            Some(above) => GpuVertex {
                position: (at + normal * (above + LIFT)).to_array(),
                ..vertex
            },
            None => vertex,
        }
    };
    let middle = |a: &GpuVertex, b: &GpuVertex| GpuVertex {
        position: (Vec3::from_array(a.position).midpoint(Vec3::from_array(b.position))).to_array(),
        ..*a
    };
    for triangle in coarse.chunks_exact(3) {
        let [a, b, c] = [&triangle[0], &triangle[1], &triangle[2]];
        let (ab, bc, ca) = (middle(a, b), middle(b, c), middle(c, a));
        for corners in [[*a, ab, ca], [ab, *b, bc], [ca, bc, *c], [ab, bc, ca]] {
            vertices.extend(corners.map(&mut place));
        }
    }
}

/// Triangulates a simple polygon by ear clipping, in the plane it lies in.
///
/// Returns index triples into `points`. **Ear clipping rather than a fan**
/// because a silhouette ring is not convex: see [`hull_triangles`], which is
/// the only caller and carries the measurement that settled it.
///
/// A polygon that runs out of ears - which a self-intersecting one does, and
/// no hull on the disc produces - has its remainder emitted as a fan rather
/// than dropped: some fill is closer to right than none, and the alternative
/// is a shadow with a bite out of it.
fn ear_clip(points: &[[f32; 2]]) -> Vec<[usize; 3]> {
    let mut out = Vec::new();
    if points.len() < 3 {
        return out;
    }
    let cross = |o: [f32; 2], a: [f32; 2], b: [f32; 2]| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    // The ring's own winding: an ear is a corner that turns the same way the
    // polygon does, and the two windings do occur - a silhouette comes out
    // wound by the faces that produced it.
    let area: f32 = (0..points.len())
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            a[0] * b[1] - b[0] * a[1]
        })
        .sum();
    let sign = if area < 0.0 { -1.0 } else { 1.0 };

    let mut remaining: Vec<usize> = (0..points.len()).collect();
    while remaining.len() > 3 {
        let before = remaining.len();
        let mut at = 0;
        while at < remaining.len() {
            let previous = remaining[(at + remaining.len() - 1) % remaining.len()];
            let current = remaining[at];
            let next = remaining[(at + 1) % remaining.len()];
            let (a, b, c) = (points[previous], points[current], points[next]);
            // Convex in the polygon's own winding, and empty: no other vertex
            // of the ring inside the candidate ear.
            if cross(a, b, c) * sign <= 0.0 {
                at += 1;
                continue;
            }
            let inside = remaining.iter().any(|other| {
                if *other == previous || *other == current || *other == next {
                    return false;
                }
                let p = points[*other];
                let (u, v, w) = (
                    cross(a, b, p) * sign,
                    cross(b, c, p) * sign,
                    cross(c, a, p) * sign,
                );
                u >= 0.0 && v >= 0.0 && w >= 0.0
            });
            if inside {
                at += 1;
                continue;
            }
            out.push([previous, current, next]);
            remaining.remove(at);
            break;
        }
        if remaining.len() == before {
            // No ear anywhere: fan what is left rather than leaving a hole.
            break;
        }
    }
    for at in 1..remaining.len().saturating_sub(1) {
        out.push([remaining[0], remaining[at], remaining[at + 1]]);
    }
    out
}

/// The alpha-over blend a shadow darkens with.
///
/// [`oag_fx::psys::BLEND_ALPHA_OVER`] by another name, and deliberately that
/// one rather than a second copy of the same equation: `src.a * src.rgb +
/// (1 - src.a) * dst.rgb` with `src.rgb` black is exactly `dst * (1 - a)`.
pub const BLEND: wgpu::BlendState = oag_fx::psys::BLEND_ALPHA_OVER;

/// One craft's silhouette, as pixels.
///
/// Coverage is in the **red** channel - high is shadow - which is the shape
/// Wipeout HD's own `ambient_shadow.gtf` decodes to: a single-channel `B8`
/// texture whose `remap` broadcasts the stored byte across rgb and forces
/// alpha opaque. [`Silhouette::falloff`] matches that convention so both paths
/// sample the same channel.
#[derive(Debug, Clone)]
pub struct Silhouette {
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
    /// `width * height * 4` bytes, RGBA8.
    pub rgba: Vec<u8>,
}

impl Silhouette {
    /// A one-texel image that is coverage everywhere.
    ///
    /// What the `original` tier's projected hulls sample: their shape is the
    /// polygon, so the texture has to contribute nothing. A one-texel bind is
    /// cheaper than a shader branch and keeps both tiers on one pipeline.
    #[must_use]
    pub fn solid() -> Self {
        Self {
            width: 1,
            height: 1,
            rgba: vec![0xff; 4],
        }
    }

    /// A soft elliptical falloff, for a title that ships no silhouette of its
    /// own.
    ///
    /// **Ours, and a substitute for a missing asset only.** Every title but
    /// Wipeout HD is in this case: `blob` `0x3e0` and `textureBlob` `0x3df`
    /// are authored zero times on the Pulse disc, so there is nothing to play
    /// instead of this. It is never drawn on a title that ships the real
    /// thing, and the whole tier is off by default.
    ///
    /// Square, because the quad's own rectangle carries the hull's shape - see
    /// [`quad`] - and a squared falloff so the edge reaches zero rather than
    /// clipping, the same curve `exhaust::FlareTexture::placeholder` uses.
    #[must_use]
    pub fn falloff(size: u32) -> Self {
        let size = size.max(1);
        let mut rgba = Vec::with_capacity((size * size * 4) as usize);
        let centre = (size as f32 - 1.0) / 2.0;
        for y in 0..size {
            for x in 0..size {
                let dx = (x as f32 - centre) / centre.max(1.0);
                let dy = (y as f32 - centre) / centre.max(1.0);
                let d = (dx * dx + dy * dy).sqrt().min(1.0);
                let coverage = ((1.0 - d) * (1.0 - d) * 255.0) as u8;
                // Broadcast across rgb with opaque alpha, which is what HD's
                // own remap produces - so `shadow.wgsl` reads one channel for
                // both paths rather than branching on where the pixels came
                // from.
                rgba.extend_from_slice(&[coverage, coverage, coverage, 0xff]);
            }
        }
        Self {
            width: size,
            height: size,
            rgba,
        }
    }

    /// Uploads this image as its own bind group.
    fn bind(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        label: &str,
    ) -> wgpu::BindGroup {
        let size = wgpu::Extent3d {
            width: self.width.max(1),
            height: self.height.max(1),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            &self.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.width.max(1) * 4),
                rows_per_image: Some(self.height.max(1)),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Clamped, not repeated: the quad is exactly the silhouette's extent,
        // and a sample that ran off the edge under repeat would put the nose
        // of one shadow at the tail of the next.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    }
}

/// The blob pass: one pipeline, one vertex buffer, one bind group per
/// silhouette.
///
/// Depth-tested and **not** depth-writing, the same state the exhaust and the
/// particles draw with and for the same reason: a shadow is blended, so a
/// depth write would let it occlude the very geometry it is meant to sit on.
#[derive(Debug)]
pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// One per distinct silhouette, indexed by [`Placement::silhouette`], with
    /// [`Silhouette::solid`] appended at the end for the hull path.
    silhouettes: Vec<wgpu::BindGroup>,
    /// Where that appended solid one sits.
    solid: usize,
    vertices: wgpu::Buffer,
    /// `(silhouette, first vertex, count)` per run, as the last
    /// [`Pipeline::upload`] grouped them.
    runs: Vec<(usize, u32, u32)>,
}

impl Pipeline {
    /// Builds the pipeline and uploads every silhouette.
    ///
    /// `format` must be the target the caller's render pass writes and
    /// `sample_count` must match its multisample state - see
    /// `mesh_render::build`. An empty `silhouettes` builds a pipeline that
    /// draws nothing, which is what a title with no shadow texture and no
    /// fallback gets.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        silhouettes: &[Silhouette],
        sample_count: u32,
        velocity: oag_mesh::mesh_render::Velocity,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow silhouette"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow"),
            bind_group_layouts: &[Some(&layout), Some(&texture_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2,
                        4 => Float32
                    ],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                // The velocity target, where the race has one, rides along
                // write-masked empty: the quad is rebuilt from scratch every
                // frame with no vertex correspondence to reproject, and it
                // writes no depth - so the velocity at its pixels stays the
                // road's behind it. The same reasoning `exhaust::Pipeline`
                // gives for its own.
                targets: &{
                    let mut targets = vec![Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(BLEND),
                        // Colour only: the bloom's glow mask lives in the
                        // alpha channel, and a shadow has no business
                        // brightening anything. See `mesh_render::GlowMask`.
                        write_mask: wgpu::ColorWrites::COLOR,
                    })];
                    targets.extend(velocity.target(true));
                    targets
                },
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                // Two-sided: the quad's winding follows a basis built from the
                // surface normal and the craft's heading, which flips as a
                // craft rolls through a loop. The alternative is a shadow that
                // disappears upside down.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: oag_mesh::mesh_render::DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow uniforms"),
            size: oag_mesh::mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow uniforms"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow vertices"),
            size: (MAX_VERTICES * std::mem::size_of::<GpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // The caller's, then one appended solid texel: the `original` tier's
        // polygons carry their own shape and need the sampler to contribute
        // nothing. Appended rather than prepended so a `Placement`'s index
        // still means what the caller passed.
        let solid = silhouettes.len();
        let silhouettes = silhouettes
            .iter()
            .chain(std::iter::once(&Silhouette::solid()))
            .enumerate()
            .map(|(index, image)| {
                image.bind(
                    device,
                    queue,
                    &texture_layout,
                    &format!("shadow silhouette {index}"),
                )
            })
            .collect();

        Self {
            pipeline,
            uniforms,
            bind_group,
            silhouettes,
            solid,
            vertices,
            runs: Vec::new(),
        }
    }

    /// Uploads this frame's camera matrix and quads.
    ///
    /// Placements are grouped by [`Placement::silhouette`] so each texture is
    /// one draw call - eight craft on a grid of eight teams is eight draws of
    /// six vertices, and a field sharing one livery is one draw. A placement
    /// naming a silhouette this pipeline was not built with, or one faded to
    /// nothing, is **dropped rather than substituted**: a shadow wearing
    /// another team's outline is worse than no shadow.
    pub fn upload(
        &mut self,
        queue: &wgpu::Queue,
        view_projection: &[[f32; 4]; 4],
        placements: &[Placement],
        hulls: &[GpuVertex],
    ) {
        let mut block = [[0.0f32; 4]; 8];
        block[..4].copy_from_slice(view_projection);
        block[4] = [1.0, 0.0, 0.0, 0.0];
        block[5] = [0.0, 1.0, 0.0, 0.0];
        block[6] = [0.0, 0.0, 1.0, 0.0];
        block[7] = [0.0, 0.0, 0.0, 1.0];
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&block));

        self.runs.clear();
        let mut vertices: Vec<GpuVertex> = Vec::with_capacity(MAX_VERTICES);
        // The blob quads first, grouped so each texture is one draw; then the
        // `original` tier's polygons, which all sample the solid texel and are
        // therefore one draw however many craft cast one.
        for index in 0..self.solid {
            let first = vertices.len() as u32;
            for placement in placements
                .iter()
                .filter(|p| p.silhouette == index && p.strength > 0.0)
                .take(MAX_QUADS.saturating_sub(vertices.len() / 6))
                .take(MAX_QUAD_VERTICES / 6)
            {
                vertices.extend(quad(placement));
            }
            let count = vertices.len() as u32 - first;
            if count > 0 {
                self.runs.push((index, first, count));
            }
        }
        let first = vertices.len() as u32;
        let room = MAX_VERTICES - vertices.len();
        // Truncated at a whole triangle: a partial one is a stray wedge across
        // the road, which is worse than a shadow that stops.
        let kept = hulls.len().min(room) / 3 * 3;
        vertices.extend_from_slice(&hulls[..kept]);
        if kept > 0 {
            self.runs.push((self.solid, first, kept as u32));
        }
        if !vertices.is_empty() {
            queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices));
        }
    }

    /// Draws into a pass the caller already opened.
    ///
    /// **After the track and before the hulls.** After, because the road's
    /// depth has to be in the buffer for the quad's depth test to keep it off
    /// the geometry behind; before, because the hulls are opaque and write
    /// depth, so a craft drawn afterwards covers its own shadow where it
    /// overlaps it rather than the shadow drawing over the craft.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.runs.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        for (silhouette, first, count) in &self.runs {
            pass.set_bind_group(1, &self.silhouettes[*silhouette], &[]);
            pass.draw(*first..*first + *count, 0..1);
        }
    }

    /// How many blob quads the last [`Pipeline::upload`] kept.
    ///
    /// An observable for the tests and the loader report: a pass that uploads
    /// nothing and a pass that draws nothing produce the same picture, so the
    /// difference needs something to read.
    #[must_use]
    pub fn quads(&self) -> usize {
        self.runs
            .iter()
            .filter(|(index, _, _)| *index != self.solid)
            .map(|(_, _, count)| *count as usize / 6)
            .sum()
    }

    /// How many hull triangles it kept, the same way.
    #[must_use]
    pub fn hull_triangles(&self) -> usize {
        self.runs
            .iter()
            .filter(|(index, _, _)| *index == self.solid)
            .map(|(_, _, count)| *count as usize / 3)
            .sum()
    }
}

pub mod map;
pub mod occlusion;
pub mod self_shadow;

#[cfg(test)]
mod tests;
