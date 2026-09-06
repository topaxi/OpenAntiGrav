//! Where the start gantry stands: the mount the circuit's own geometry authors.
//!
//! `docs/rendering/start-gantry.md` recovers what
//! `Data\Environments\321_Go\321Go_StartFinish.vex` *does* - four glyphs walked
//! across a palette staircase by one authored `TEXOFFSET` track. It does not
//! place it, and for seven passes nothing did: `TrackStartup.xml`'s slot 8
//! names the model and carries no transform, and neither does the billboard
//! system that loads it, on Pulse or on HD.
//!
//! The placement is in the **track**. Every circuit models a small mounting
//! surface into `track.vex` and textures it `billboard8.tga` - an 8x8 stub
//! where every sibling texture is 64x64 - alongside `321backplate.tga`, the
//! countdown board's backing panel. Both textures sit on one `Mesh` node
//! (node 74 on `16_Track`, node 19 on `05_Track`), and on both circuits that
//! node stands ~165 units from the file's own `Start Position` and 10-15 units
//! above the road. See `docs/rendering/start-gantry.md`'s placement section.
//!
//! # What this module adds to that
//!
//! A centroid is a position, not a basis. This reads the mount's **plane**
//! out of the node's own vertices - principal axes of their covariance, so the
//! smallest-variance axis is the surface normal and the other two span the
//! panel - and its **extent** along each in-plane axis, which is what gives a
//! scale rather than a guess at one.
//!
//! Two things this deliberately does not decide:
//!
//! - **The normal's sign.** A principal axis is a line, not a direction; the
//!   eigenvector could as easily come back pointing into the scenery. The
//!   caller resolves it against something that knows which way the race runs -
//!   the track tangent at the start line - and [`Mount::matrix`] takes that
//!   direction rather than inventing one. See [`Mount::normal`].
//! - **Which of the two co-located surfaces to measure.** They are one unit
//!   apart, so for the *centre* the choice is noise; for the *basis* it is
//!   only noise if the two planes agree, which
//!   `gantry_mount_ground_truth` measures rather than assumes.
//!
//! Nothing here hardcodes a coordinate. The mount is re-derived per circuit
//! from that circuit's own geometry, so a circuit that authors no such node
//! yields `None` and its caller draws nothing.

use oag_core::math::{Mat3, Mat4, Quat, Vec3};

use crate::mesh::Model;

/// The countdown board's backing panel, in the track's own texture set.
///
/// Present on every Pulse circuit checked, and **not** one of
/// `321Go_StartFinish.vex`'s own six textures - so it is track geometry
/// authored for the gantry rather than part of the gantry.
pub const BACKPLATE_TEXTURE: &str = "321backplate.tga";

/// Slot 8's placeholder stub, the Pulse spelling of HD's `billboard8.gtf`.
///
/// 8x8 where every sibling is 64x64. Used as the fallback surface for a
/// circuit that authors the stub without the backplate.
pub const SLOT8_TEXTURE: &str = "billboard8.tga";

/// The mounting surface a circuit authors for its start gantry.
///
/// All of it is measured off the track's own vertices, in track space, after
/// the mesh build has composed the scene transforms - the same space
/// [`crate::mesh::DrawCall::bounds`] is in.
#[derive(Debug, Clone, Copy)]
pub struct Mount {
    /// The scene node the surface belongs to, for reporting.
    pub node: Option<u32>,
    /// Centroid of the surface's vertices.
    pub centre: Vec3,
    /// Unit normal of the surface's plane, **sign unresolved**.
    ///
    /// The smallest-variance principal axis. Its sign is arbitrary: the same
    /// plane admits both, and nothing in a covariance says which side the
    /// track is on. [`Mount::matrix`] takes the facing direction from the
    /// caller for exactly this reason.
    pub normal: Vec3,
    /// Unit in-plane axis closest to world up, pointing upward.
    pub up: Vec3,
    /// Unit in-plane axis across the surface, completing a right-handed basis
    /// with [`Self::up`] and [`Self::normal`].
    pub right: Vec3,
    /// Full extent along [`Self::right`], in track units.
    pub width: f32,
    /// Full extent along [`Self::up`], in track units.
    pub height: f32,
    /// Full extent along [`Self::normal`] - how far from flat the surface is.
    ///
    /// Small against [`Self::width`] is what makes the plane meaningful; a
    /// caller that finds this comparable to the other two has not found a
    /// panel and should not treat the normal as one.
    pub thickness: f32,
    /// How many vertices the measurement is over.
    pub vertices: usize,
    /// Mean of the surface's own authored vertex normals, normalised.
    ///
    /// A second, independent opinion on which side of the plane faces out,
    /// and unlike [`Self::normal`] it carries a sign - the artist authored
    /// one. Zero-length when the normals cancel, which is what a two-sided
    /// panel modelled back to back would give, so a caller must not rely on
    /// it alone. `gantry_mount_ground_truth` reports the angle between this
    /// and the track's own start-line forward rather than assuming they
    /// agree.
    pub outward: Vec3,
}

impl Mount {
    /// The model matrix that stands a gantry on this mount, facing `forward`.
    ///
    /// `forward` is the direction the *race* runs at the start line - the
    /// track tangent - and it resolves [`Self::normal`]'s sign and nothing
    /// else: the basis is the surface's own. `model_forward` is the axis the
    /// gantry model's own board faces in its own space, and `scale` is applied
    /// uniformly.
    ///
    /// The translation is [`Self::centre`] exactly. No nudge, no lift: if the
    /// gantry sits wrong, the mount is wrong and that is the thing to fix.
    #[must_use]
    pub fn matrix(&self, forward: Vec3, model_forward: Vec3, scale: f32) -> Mat4 {
        let normal = self.facing(forward);
        // The mount's own basis, re-orthogonalised around the resolved normal
        // so that `up` stays the surface's up rather than being recomputed
        // from a world axis the circuit may not agree with.
        let right = self.up.cross(normal).normalize_or_zero();
        let right = if right == Vec3::ZERO {
            self.right
        } else {
            right
        };
        let up = normal.cross(right).normalize_or_zero();
        let basis = Mat3::from_cols(right, up, normal);
        // The rotation that takes the model's own facing axis onto the mount's
        // normal, expressed in the mount's basis: a model that faces -Z needs
        // a half turn about its up axis before the basis applies.
        let align = Quat::from_rotation_arc(model_forward.normalize_or_zero(), Vec3::Z);
        Mat4::from_scale_rotation_translation(
            Vec3::splat(scale),
            Quat::from_mat3(&basis) * align,
            self.centre,
        )
    }

    /// [`Self::normal`] with its sign resolved to point along `forward`.
    #[must_use]
    pub fn facing(&self, forward: Vec3) -> Vec3 {
        if self.normal.dot(forward) < 0.0 {
            -self.normal
        } else {
            self.normal
        }
    }
}

/// The mount a track model authors, preferring the backplate over the stub.
///
/// `None` when neither texture is bound by any draw - a circuit this project
/// cannot place a gantry on, which is a thing to report rather than to paper
/// over with a coordinate from a circuit that does.
#[must_use]
pub fn mount(model: &Model) -> Option<Mount> {
    surface(model, BACKPLATE_TEXTURE).or_else(|| surface(model, SLOT8_TEXTURE))
}

/// The plane of every draw in `model` that binds a texture named `texture`.
///
/// Matched on the texture's own label, case-insensitively and on the file name
/// alone, because a track's texture labels are file names and their case is
/// not consistent across circuits.
#[must_use]
pub fn surface(model: &Model, texture: &str) -> Option<Mount> {
    let slots: Vec<usize> = model
        .textures
        .iter()
        .enumerate()
        .filter(|(_, t)| {
            t.as_ref()
                .is_some_and(|t| t.label.eq_ignore_ascii_case(texture))
        })
        .map(|(slot, _)| slot)
        .collect();
    if slots.is_empty() {
        return None;
    }

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut node = None;
    for draws in [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ] {
        for draw in draws {
            if !draw.texture.is_some_and(|slot| slots.contains(&slot)) {
                continue;
            }
            node = node.or(draw.node);
            for i in draw.range.clone() {
                let v = model.vertices[model.indices[i as usize] as usize];
                positions.push(Vec3::from(v.position));
                normals.push(Vec3::from(v.normal));
            }
        }
    }
    let mut mount = plane(node, &positions)?;
    mount.outward = normals
        .iter()
        .fold(Vec3::ZERO, |a, n| a + *n)
        .normalize_or_zero();
    Some(mount)
}

/// Drops every draw whose node stands **outside the mount's own panel** at
/// `seconds`, and says how many it dropped.
///
/// # Why a gantry needs this at all
///
/// `321Go_StartFinish.vex` is one object carrying every state the gantry ever
/// shows, as non-overlapping windows on a single timeline
/// (`docs/rendering/start-gantry.md`, "One asset, the whole race"). A state
/// that is not its turn is not hidden - it is **parked off to the side**:
/// `Final_Lap` and `Honey_Board` sit 33 units left of the aperture until they
/// slide in at 9.333 s, and the two chequered boards 43 units right until
/// 12.333 s. The countdown panel itself sits at the origin.
///
/// The original has somewhere to park them, because the aperture is a hole in
/// the track's own structure and the parked boards are behind it. This project
/// draws the model in open air, so without this the very first frame of a race
/// shows a full-size, perfectly legible `FINAL LAP` board hanging beside the
/// countdown - a state whose trigger is unrecovered, announced on lap one.
///
/// # The panel is the aperture, and the track authors its width
///
/// So the test is the mount's own width, measured off the circuit's geometry
/// by [`surface`]: a draw whose node centre is further than half of it from the
/// panel's centre is not on the board. That is a measurement, not a list of
/// node names - and it keeps the parts that *are* on the board at `seconds`
/// (the countdown panel and its backlight; the `Board`, `Text` and `Arrow`
/// dressing, which is parked 34 units **below** rather than aside and so
/// passes a horizontal test) without naming any of them.
///
/// `seconds` is when the parking is evaluated. A one-shot rather than a
/// per-frame cull because the retained states do not leave the panel and the
/// dropped ones do not enter it before
/// `oag_game::race::gantry::CLOCK_LIMIT`.
pub fn clip_to_panel(model: &mut Model, half_width: f32, seconds: f32) -> usize {
    let matrices = model.sample_anim_nodes(seconds);
    // The **midpoint of the draw's own x extent**, not the mean of its
    // vertices. A board's triangles are not evenly distributed across it -
    // `Final_Lap`'s glyphs crowd one end - and a vertex mean drifts far enough
    // towards the crowded side to move a parked board back onto the panel by a
    // unit or two. Which is exactly what it did: one of the `FINAL LAP` board's
    // three draws survived the clip on eleven of twelve circuits.
    let midpoint = |draw: &crate::mesh::DrawCall| -> Option<f32> {
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for i in draw.range.clone() {
            let v = model.vertices[model.indices[i as usize] as usize];
            let p = Vec3::from(v.position);
            let p = match v
                .xform
                .checked_sub(1)
                .and_then(|s| matrices.get(s as usize))
            {
                Some(m) => Mat4::from_cols_array(m).transform_point3(p),
                None => p,
            };
            lo = lo.min(p.x);
            hi = hi.max(p.x);
        }
        (lo <= hi).then_some(0.5 * (lo + hi))
    };
    // The model's own board plane is its XY plane - see
    // `gantry_mount_ground_truth`, which measures the glyph node's normal at
    // `(0, 0, 1)` - so "across the panel" is the model's own x.
    let outside =
        |draw: &crate::mesh::DrawCall| midpoint(draw).is_some_and(|x| x.abs() > half_width);
    let lists = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ];
    let total: usize = lists.iter().map(|l| l.len()).sum();
    let count = lists
        .into_iter()
        .flatten()
        .filter(|draw| outside(draw))
        .count();
    // **Clipping everything means the test is wrong, not the model.** The
    // measure is model-space x, which is "across the panel" only for a model
    // whose board plane is its own XY and whose origin is the aperture centre.
    // `321Go_StartFinish.vex` is both; another title's gantry need not be, and
    // the failure that would cause - the whole object silently deleted - looks
    // exactly like a circuit that authors no gantry. Drop nothing instead and
    // let the caller report it.
    if count == total {
        return 0;
    }
    model.draws.retain(|d| !outside(d));
    model.alpha_tested_draws.retain(|d| !outside(d));
    model.transparent_draws.retain(|d| !outside(d));
    count
}

/// The plane of every draw in `model` that came from one scene node.
///
/// The node-keyed counterpart of [`surface`], for a model whose interesting
/// piece has a *name* rather than a distinctive texture -
/// `321Go_StartFinish.vex`'s own `start_light_321go` glyph board, which is
/// what a mount's basis has to be matched against.
#[must_use]
pub fn node_plane(model: &Model, node: u32) -> Option<Mount> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for draws in [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ] {
        for draw in draws.iter().filter(|d| d.node == Some(node)) {
            for i in draw.range.clone() {
                let v = model.vertices[model.indices[i as usize] as usize];
                positions.push(Vec3::from(v.position));
                normals.push(Vec3::from(v.normal));
            }
        }
    }
    let mut mount = plane(Some(node), &positions)?;
    mount.outward = normals
        .iter()
        .fold(Vec3::ZERO, |a, n| a + *n)
        .normalize_or_zero();
    Some(mount)
}

/// The principal-axis frame of a set of points, or `None` for fewer than three.
fn plane(node: Option<u32>, positions: &[Vec3]) -> Option<Mount> {
    if positions.len() < 3 {
        return None;
    }
    let centre = positions.iter().fold(Vec3::ZERO, |a, p| a + *p) / positions.len() as f32;

    let mut covariance = [[0.0f32; 3]; 3];
    for p in positions {
        let d = *p - centre;
        for (r, row) in covariance.iter_mut().enumerate() {
            for (c, cell) in row.iter_mut().enumerate() {
                *cell += d[r] * d[c];
            }
        }
    }
    let n = positions.len() as f32;
    for row in &mut covariance {
        for cell in row {
            *cell /= n;
        }
    }

    let (values, vectors) = jacobi(covariance);
    // Ascending by eigenvalue: the smallest-variance axis is the normal, the
    // largest is the surface's long axis.
    let mut order = [0usize, 1, 2];
    order.sort_by(|a, b| values[*a].total_cmp(&values[*b]));
    let normal = vectors[order[0]];
    let mid = vectors[order[1]];
    let long = vectors[order[2]];

    // Of the two in-plane axes, the one with the larger vertical component is
    // this surface's up. A start-line board is wider than it is tall, so the
    // long axis is normally the horizontal one - but that is an expectation,
    // not a rule, and picking by verticality does not depend on it.
    let (mut up, mut right) = if mid.y.abs() > long.y.abs() {
        (mid, long)
    } else {
        (long, mid)
    };
    if up.y < 0.0 {
        up = -up;
    }
    // Right-handed with the raw normal; `Mount::matrix` re-derives it once the
    // sign is resolved, so this is the reported basis rather than the drawn one.
    if right.cross(up).dot(normal) < 0.0 {
        right = -right;
    }

    let extent = |axis: Vec3| {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for p in positions {
            let d = (*p - centre).dot(axis);
            lo = lo.min(d);
            hi = hi.max(d);
        }
        hi - lo
    };

    Some(Mount {
        node,
        centre,
        normal,
        up,
        right,
        width: extent(right),
        height: extent(up),
        thickness: extent(normal),
        vertices: positions.len(),
        outward: Vec3::ZERO,
    })
}

/// Eigenvalues and unit eigenvectors of a symmetric 3x3, by Jacobi rotation.
///
/// Written out rather than pulled in: `glam` has no symmetric eigensolver, and
/// three sweeps of a 3x3 Jacobi is a dozen lines. The renderer is exempt from
/// the simulation's determinism rules (`docs/architecture/determinism.md`), but
/// this is plain `f32` arithmetic anyway - it feeds a model matrix, not a state
/// hash.
fn jacobi(mut a: [[f32; 3]; 3]) -> ([f32; 3], [Vec3; 3]) {
    let mut v = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..32 {
        // The largest off-diagonal magnitude decides whether another sweep is
        // worth anything, and which pair to annihilate.
        let mut p = 0;
        let mut q = 1;
        let mut best = a[0][1].abs();
        for (i, j) in [(0usize, 2usize), (1, 2)] {
            if a[i][j].abs() > best {
                best = a[i][j].abs();
                p = i;
                q = j;
            }
        }
        if best <= 1e-9 {
            break;
        }
        let theta = 0.5 * (a[q][q] - a[p][p]) / a[p][q];
        let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
        let c = 1.0 / (t * t + 1.0).sqrt();
        let s = t * c;
        let mut rotated = a;
        for k in 0..3 {
            rotated[k][p] = c * a[k][p] - s * a[k][q];
            rotated[k][q] = s * a[k][p] + c * a[k][q];
        }
        let mut next = rotated;
        for k in 0..3 {
            next[p][k] = c * rotated[p][k] - s * rotated[q][k];
            next[q][k] = s * rotated[p][k] + c * rotated[q][k];
        }
        a = next;
        let mut vn = v;
        for k in 0..3 {
            vn[k][p] = c * v[k][p] - s * v[k][q];
            vn[k][q] = s * v[k][p] + c * v[k][q];
        }
        v = vn;
    }
    (
        [a[0][0], a[1][1], a[2][2]],
        [
            Vec3::new(v[0][0], v[1][0], v[2][0]).normalize_or_zero(),
            Vec3::new(v[0][1], v[1][1], v[2][1]).normalize_or_zero(),
            Vec3::new(v[0][2], v[1][2], v[2][2]).normalize_or_zero(),
        ],
    )
}

#[cfg(test)]
mod tests;
