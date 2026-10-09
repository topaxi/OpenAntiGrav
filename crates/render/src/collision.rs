//! Turns a track's decoded collision nodes into something the mesh renderer can
//! draw.
//!
//! This is a **debug view of what the physics world actually contains**, not an
//! art path. `docs/formats/collision.md` documents the format and
//! `oag_gameplay::collision_world` turns the same nodes into colliders; this
//! module is the third consumer of that decode and the only one you can look at.
//!
//! Two things it is deliberately not:
//!
//! - It does not depend on `oag-gameplay` or `oag-physics`. Everything here
//!   comes out of `oag_vex::collision`, so the viewer does not drag a
//!   gameplay crate in to draw a triangle. The one rule it does copy from
//!   `collision_world` is that [`SurfaceKind::Cage`] is not collidable, so by
//!   default it is not drawn either - the picture is what physics sees, not what
//!   the file holds. Pass `include_cage` to see the rest.
//! - It does not do collision *response*. Nothing here reads friction.
//!
//! ## Why the default is wireframe
//!
//! The mesh pipeline writes depth, compares `Less`, and the shader forces alpha
//! to 1.0. Solid walls therefore occlude everything inside them, and a solid
//! render of a track's collision is a picture of the outside of a box. The whole
//! point of this view is to check that the walls enclose the driveable ribbon,
//! so the default draws triangle outlines and you can see through them.
//!
//! The outlines are made by insetting each triangle towards its own centroid and
//! filling the ring between the two, which needs no `POLYGON_MODE_LINE` device
//! feature and so works on every backend, including the headless capture path.
//! Interior edges are drawn twice, once from each adjacent face; that is
//! cosmetic and not worth a de-duplication pass.

use anyhow::{Context, Result};
use oag_vex::collision::{self, CollisionNode, SurfaceKind};

use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model};

/// How the soup is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Style {
    /// Triangle outlines, so geometry behind a wall stays visible.
    #[default]
    Wireframe,
    /// Filled, flat-shaded triangles.
    Solid,
}

/// Outline width as a fraction of the model's *mean edge length*.
///
/// Not a fraction of the bounding radius, which was the first attempt and came
/// out wrong: a track is hundreds of units long and tens wide, so a width scaled
/// to its longest axis draws sub-pixel lines that render as dots. Scaling to the
/// mesh's own triangle size instead makes the outline readable at whatever zoom
/// shows the triangles at all, on a small test corridor and a whole track alike.
///
/// It is still one width for the whole model rather than per triangle, so a big
/// floor triangle and a small wall triangle draw the same weight of line.
const OUTLINE_FRACTION: f32 = 0.14;

/// An outline may never eat more than this fraction of the way to the centroid,
/// or a triangle smaller than the line width inverts and renders as noise.
const OUTLINE_MAX_INSET: f32 = 0.35;

/// Loads a track's collision nodes from a `.vex` model inside an archive.
///
/// The same `.vex` [`crate::track::load`] reads: a track file carries the
/// spline and the collision soup side by side.
pub fn load(spec: &str, name: &str) -> Result<(Vec<CollisionNode>, String)> {
    let data = oag_mesh::mesh::read_blob(spec, name)?;
    let nodes = collision::from_vex(&data)
        .map_err(|e| anyhow::anyhow!("{name}: {e}"))
        .context("decoding collision nodes")?;
    Ok((nodes, name.to_string()))
}

/// Whether a class contributes to the collision world.
///
/// `Cage` is the only `false`, and it is not a gap: the original parses cage
/// nodes and branches past them. This mirrors `oag_gameplay::collision::
/// surface_for` returning `None`, restated rather than imported so that the
/// renderer does not depend on a gameplay crate.
#[must_use]
pub fn is_collidable(kind: SurfaceKind) -> bool {
    kind != SurfaceKind::Cage
}

/// The debug colour for a collision class.
///
/// Chosen to be told apart at a glance and away from the track ribbon's own
/// palette, which is pastel per section plus yellow and green strips.
#[must_use]
pub fn colour_for(kind: SurfaceKind) -> [f32; 4] {
    match kind {
        SurfaceKind::Wall => [1.0, 0.25, 0.2, 1.0],
        // Wall-red pulled towards orange: it collides as a wall and the two are
        // meant to be told apart on a circuit that ships both.
        SurfaceKind::TrackWall => [1.0, 0.6, 0.15, 1.0],
        SurfaceKind::Floor => [0.35, 0.6, 1.0, 1.0],
        SurfaceKind::Reset => [1.0, 0.3, 0.95, 1.0],
        SurfaceKind::MagFloor => [0.4, 1.0, 0.45, 1.0],
        // Drawn only on request, and dim, because it is not collidable.
        SurfaceKind::Cage => [0.45, 0.45, 0.45, 1.0],
    }
}

/// An axis-aligned box, or `None` for an empty set of points.
pub type Aabb = ([f32; 3], [f32; 3]);

/// What one collision class contributes to a track.
#[derive(Clone, Copy, Debug)]
pub struct KindStats {
    pub kind: SurfaceKind,
    pub nodes: usize,
    pub meshes: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub bounds: Option<Aabb>,
}

/// Per-class counts and extents, in [`SurfaceKind::ALL`] order.
///
/// Every class appears, including ones with nothing in them, so a track that
/// ships no `Reset` volumes says so rather than leaving the reader guessing
/// whether the decode dropped them.
#[must_use]
pub fn stats(nodes: &[CollisionNode]) -> Vec<KindStats> {
    SurfaceKind::ALL
        .iter()
        .map(|&kind| {
            let mut out = KindStats {
                kind,
                nodes: 0,
                meshes: 0,
                vertices: 0,
                triangles: 0,
                bounds: None,
            };
            for node in nodes.iter().filter(|n| n.kind == kind) {
                out.nodes += 1;
                for mesh in &node.geometry.meshes {
                    out.meshes += 1;
                    out.vertices += mesh.vertices.len();
                    out.triangles += mesh.triangles.len();
                    grow(&mut out.bounds, &mesh.vertices);
                }
            }
            out
        })
        .collect()
}

/// The box enclosing every vertex of the classes `keep` accepts.
#[must_use]
pub fn bounds_of(nodes: &[CollisionNode], keep: impl Fn(SurfaceKind) -> bool) -> Option<Aabb> {
    let mut out = None;
    for node in nodes.iter().filter(|n| keep(n.kind)) {
        for mesh in &node.geometry.meshes {
            grow(&mut out, &mesh.vertices);
        }
    }
    out
}

/// The box enclosing a built model's vertices.
///
/// [`bounds_of`]'s counterpart for geometry that only exists as a [`Model`]:
/// the track ribbon is built rather than decoded, so there is no
/// [`CollisionNode`] to measure and it is measured after the fact. Here beside
/// the node-side one because comparing the two boxes is the whole point of
/// either - see `oag-view --collision --with-spline`.
#[must_use]
pub fn bounds_of_model(model: &Model) -> Option<Aabb> {
    let mut out = None;
    for vertex in &model.vertices {
        grow(&mut out, std::slice::from_ref(&vertex.position));
    }
    out
}

/// Whether `inner` sits entirely inside `outer`, allowing `slack` either side.
#[must_use]
pub fn contains(outer: Aabb, inner: Aabb, slack: f32) -> bool {
    (0..3).all(|i| outer.0[i] - slack <= inner.0[i] && inner.1[i] <= outer.1[i] + slack)
}

fn grow(box_: &mut Option<Aabb>, points: &[[f32; 3]]) {
    for p in points {
        match box_ {
            None => *box_ = Some((*p, *p)),
            Some((lo, hi)) => {
                for i in 0..3 {
                    lo[i] = lo[i].min(p[i]);
                    hi[i] = hi[i].max(p[i]);
                }
            }
        }
    }
}

/// Builds a drawable model from decoded collision nodes.
///
/// Vertices are emitted per triangle rather than shared, because the shading is
/// flat: a shared vertex would have to average the normals of faces that meet at
/// a hard edge, which is exactly the edge this view exists to show.
#[must_use]
pub fn build_model(
    label: &str,
    nodes: &[CollisionNode],
    style: Style,
    include_cage: bool,
) -> Model {
    let keep = |kind: SurfaceKind| include_cage || is_collidable(kind);

    // One outline width for the whole model, so it has to be measured before
    // any geometry is emitted.
    let inset = mean_edge_length(nodes, keep) * OUTLINE_FRACTION;

    let mut vertices: Vec<GpuVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut mesh_count = 0;

    for node in nodes.iter().filter(|n| keep(n.kind)) {
        let colour = colour_for(node.kind);
        for mesh in &node.geometry.meshes {
            mesh_count += 1;
            for tri in &mesh.triangles {
                let Some(corners) = corners_of(mesh, *tri) else {
                    continue;
                };
                let normal = face_normal(&corners);
                match style {
                    Style::Solid => {
                        emit_face(&mut vertices, &mut indices, &corners, normal, colour);
                    }
                    Style::Wireframe => {
                        emit_outline(&mut vertices, &mut indices, &corners, normal, colour, inset);
                    }
                }
            }
        }
    }

    let (centre, radius) = frame(&vertices);
    // Untextured, so there is nothing to split the buffer on. Bounds use the
    // box's own circumscribing sphere (the diagonal), not `radius` above,
    // which is only half the longest single axis and would under-cover the
    // corners of a non-cubic box.
    let bounds_radius = vertices
        .iter()
        .map(|v| {
            (0..3)
                .map(|i| (v.position[i] - centre[i]).powi(2))
                .sum::<f32>()
        })
        .fold(0.0f32, f32::max)
        .sqrt();
    let draws = vec![DrawCall {
        moving: false,
        // Synthetic: no batch, so no recovered blend class.
        blend: None,
        blend_state: None,
        // Synthetic: no mesh, so no derived layer.
        layer: oag_vex::vex::LAYER_DEFAULT,
        culled: false,
        range: 0..indices_len(&indices),
        texture: None,
        bounds: Bounds {
            centre,
            radius: bounds_radius,
        },
        node: None,
        chunk: None,
        // Synthetic: no batch, so no authored alpha-test reference.
        alpha_test_ref: None,
    }];
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: label.to_string(),
        vertices,
        indices,
        draws,
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: Vec::new(),
        lightmaps: Vec::new(),
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_colour_factor: Vec::new(),
        material_variants: Vec::new(),
        material_anim: Vec::new(),
        shine_draws: Vec::new(),

        vertex_colour_is_light: false,
        stamps_glow: false,
        glow_by_texel: false,

        flame: None,
        absorb_shell: false,
        alpha_test_ref: None,
        centre,
        radius,
        mesh_count,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: Vec::new(),
    }
}

/// Mean triangle edge length over every drawn triangle, or 1.0 if there are
/// none.
///
/// The mean rather than the median: a median needs a sort, and the outline width
/// only has to be the right order of magnitude. Degenerate triangles contribute
/// zero-length edges and pull it down slightly, which is harmless.
fn mean_edge_length(nodes: &[CollisionNode], keep: impl Fn(SurfaceKind) -> bool) -> f32 {
    let mut total = 0.0f64;
    let mut edges = 0u64;
    for node in nodes.iter().filter(|n| keep(n.kind)) {
        for mesh in &node.geometry.meshes {
            for tri in &mesh.triangles {
                let Some(c) = corners_of(mesh, *tri) else {
                    continue;
                };
                for k in 0..3 {
                    let a = c[k];
                    let b = c[(k + 1) % 3];
                    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                    total += f64::from((d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt());
                    edges += 1;
                }
            }
        }
    }
    if edges == 0 {
        return 1.0;
    }
    #[allow(clippy::cast_possible_truncation)]
    let mean = (total / edges as f64) as f32;
    mean.max(f32::MIN_POSITIVE)
}

fn indices_len(indices: &[u32]) -> u32 {
    u32::try_from(indices.len()).unwrap_or(u32::MAX)
}

/// The three world-space corners of a triangle, or `None` if an index is out of
/// range.
///
/// The format guarantees in-range indices - `docs/formats/collision.md` checked
/// all 1.78 million of them - so this only ever skips on a corrupt file, and it
/// skips rather than panicking because a viewer that dies on one bad triangle
/// cannot show you the bad triangle.
fn corners_of(mesh: &oag_vex::collision::CollisionMesh, tri: [u16; 3]) -> Option<[[f32; 3]; 3]> {
    Some([
        *mesh.vertices.get(usize::from(tri[0]))?,
        *mesh.vertices.get(usize::from(tri[1]))?,
        *mesh.vertices.get(usize::from(tri[2]))?,
    ])
}

fn face_normal(c: &[[f32; 3]; 3]) -> [f32; 3] {
    let u = [c[1][0] - c[0][0], c[1][1] - c[0][1], c[1][2] - c[0][2]];
    let v = [c[2][0] - c[0][0], c[2][1] - c[0][1], c[2][2] - c[0][2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len > 1e-12 {
        [n[0] / len, n[1] / len, n[2] / len]
    } else {
        // A degenerate triangle has no normal. Point it up rather than
        // producing NaNs that would poison the whole draw.
        [0.0, 1.0, 0.0]
    }
}

fn vertex(position: [f32; 3], normal: [f32; 3], colour: [f32; 4], lit: f32) -> GpuVertex {
    GpuVertex {
        position,
        normal,
        colour,
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: oag_mesh::mesh::slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    }
}

/// Direction the two-sided shading term is measured against.
///
/// Any direction not parallel to an axis works; this one is chosen so that a
/// floor, a left wall and a right wall all come out different.
const SHADE_DIR: [f32; 3] = [0.38, 0.84, 0.39];

fn emit_face(
    vertices: &mut Vec<GpuVertex>,
    indices: &mut Vec<u32>,
    corners: &[[f32; 3]; 3],
    normal: [f32; 3],
    colour: [f32; 4],
) {
    // Shaded here rather than by the light rig, and from `abs` of the dot
    // product, because **collision winding does not reliably face outwards**.
    // Through the rig a back-facing triangle comes out black, and half a track's
    // collision would silently disappear into the background - which is exactly
    // the kind of thing this view exists to rule out, not to cause.
    let facing = (normal[0] * SHADE_DIR[0] + normal[1] * SHADE_DIR[1] + normal[2] * SHADE_DIR[2])
        .abs()
        .clamp(0.0, 1.0);
    let shade = 0.45 + 0.55 * facing;
    let shaded = [
        colour[0] * shade,
        colour[1] * shade,
        colour[2] * shade,
        colour[3],
    ];

    let base = vertices.len() as u32;
    for c in corners {
        vertices.push(vertex(*c, normal, shaded, 0.0));
    }
    indices.extend([base, base + 1, base + 2]);
}

/// Emits the ring between a triangle and an inset copy of itself.
fn emit_outline(
    vertices: &mut Vec<GpuVertex>,
    indices: &mut Vec<u32>,
    corners: &[[f32; 3]; 3],
    normal: [f32; 3],
    colour: [f32; 4],
    inset: f32,
) {
    let centroid = [
        (corners[0][0] + corners[1][0] + corners[2][0]) / 3.0,
        (corners[0][1] + corners[1][1] + corners[2][1]) / 3.0,
        (corners[0][2] + corners[1][2] + corners[2][2]) / 3.0,
    ];

    let base = vertices.len() as u32;
    for c in corners {
        let to_centre = [centroid[0] - c[0], centroid[1] - c[1], centroid[2] - c[2]];
        let len = (to_centre[0] * to_centre[0]
            + to_centre[1] * to_centre[1]
            + to_centre[2] * to_centre[2])
            .sqrt();
        // Cap the inset so a triangle smaller than the line width shrinks to a
        // thinner outline instead of turning inside out.
        let step = inset.min(len * OUTLINE_MAX_INSET);
        let inner = if len > 1e-12 {
            [
                c[0] + to_centre[0] / len * step,
                c[1] + to_centre[1] / len * step,
                c[2] + to_centre[2] / len * step,
            ]
        } else {
            *c
        };
        // Unlit: an outline is a marker, and shading it would fade the far side
        // of the geometry just where the enclosure check needs to read it.
        vertices.push(vertex(*c, normal, colour, 0.0));
        vertices.push(vertex(inner, normal, colour, 0.0));
    }

    // Outer/inner pairs sit at base + 2k and base + 2k + 1; quad each edge.
    for k in 0..3u32 {
        let a = base + k * 2;
        let b = base + ((k + 1) % 3) * 2;
        indices.extend([a, b, a + 1, a + 1, b, b + 1]);
    }
}

/// Bounding-sphere centre and radius, the two things the camera needs.
fn frame(vertices: &[GpuVertex]) -> ([f32; 3], f32) {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for v in vertices {
        for i in 0..3 {
            lo[i] = lo[i].min(v.position[i]);
            hi[i] = hi[i].max(v.position[i]);
        }
    }
    if vertices.is_empty() {
        return ([0.0; 3], 1.0);
    }
    let centre = [
        (lo[0] + hi[0]) * 0.5,
        (lo[1] + hi[1]) * 0.5,
        (lo[2] + hi[2]) * 0.5,
    ];
    let radius = (0..3)
        .map(|i| (hi[i] - lo[i]) * 0.5)
        .fold(0.0f32, f32::max)
        .max(0.001);
    (centre, radius)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_vex::collision::{CollisionGeometry, CollisionMesh};

    fn mesh() -> CollisionMesh {
        CollisionMesh {
            vertices: vec![[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [0.0, 0.0, 4.0]],
            triangles: vec![[0, 1, 2]],
            vertex_scalars: vec![1.0, 1.0, 1.0],
            chunks: Vec::new(),
        }
    }

    fn node(kind: SurfaceKind, meshes: usize) -> CollisionNode {
        CollisionNode {
            kind,
            node_index: 0,
            name: None,
            geometry: CollisionGeometry {
                version: 0,
                meshes: (0..meshes).map(|_| mesh()).collect(),
            },
        }
    }

    /// A cage is parsed and then branched past by the original, so the default
    /// picture must not contain one - otherwise the view shows a wall the
    /// physics world does not have.
    #[test]
    fn cage_is_absent_by_default_and_present_on_request() {
        let nodes = [node(SurfaceKind::Cage, 1)];
        let hidden = build_model("t", &nodes, Style::Solid, false);
        assert!(hidden.vertices.is_empty());
        let shown = build_model("t", &nodes, Style::Solid, true);
        assert_eq!(shown.vertices.len(), 3);
        assert!(!is_collidable(SurfaceKind::Cage));
    }

    /// Zero nodes is a real input - every `.vex` on the Pure disc, and any
    /// Pulse file that is not a track - so the model has to come back empty
    /// rather than half-built. The draw call is still emitted, with an empty
    /// range, because a caller that hands the model to a renderer needs the
    /// shape of it either way; the emptiness is what the render paths and
    /// `oag-view` test, `wgpu::Buffer::slice` panicking on a zero-length
    /// buffer.
    #[test]
    fn no_nodes_gives_an_empty_model() {
        for style in [Style::Solid, Style::Wireframe] {
            let model = build_model("t", &[], style, true);
            assert!(model.vertices.is_empty());
            assert!(model.indices.is_empty());
        }
    }

    #[test]
    fn a_solid_triangle_becomes_exactly_one_triangle() {
        let model = build_model("t", &[node(SurfaceKind::Wall, 1)], Style::Solid, false);
        assert_eq!(model.indices.len(), 3);
        assert_eq!(model.vertices.len(), 3);
    }

    /// Six vertices and three quads per triangle, and every index in range -
    /// the ring is built by hand and an off-by-one here draws garbage rather
    /// than failing.
    #[test]
    fn a_wireframe_triangle_becomes_a_closed_ring() {
        let model = build_model("t", &[node(SurfaceKind::Wall, 1)], Style::Wireframe, false);
        assert_eq!(model.vertices.len(), 6);
        assert_eq!(model.indices.len(), 18);
        let n = model.vertices.len() as u32;
        assert!(model.indices.iter().all(|&i| i < n));
    }

    /// The inset must never cross the centroid: a triangle turned inside out
    /// renders as noise exactly where the geometry is smallest.
    #[test]
    fn the_outline_never_inverts_a_small_triangle() {
        let mut tiny = node(SurfaceKind::Wall, 1);
        tiny.geometry.meshes[0].vertices =
            vec![[0.0, 0.0, 0.0], [1e-3, 0.0, 0.0], [0.0, 0.0, 1e-3]];
        // A big second node forces a line width far larger than the tiny one.
        let mut big = node(SurfaceKind::Wall, 1);
        big.geometry.meshes[0].vertices =
            vec![[0.0, 0.0, 0.0], [1000.0, 0.0, 0.0], [0.0, 0.0, 1000.0]];
        let model = build_model("t", &[tiny, big], Style::Wireframe, false);
        assert!(
            model
                .vertices
                .iter()
                .all(|v| v.position.iter().all(|c| c.is_finite()))
        );
        // The tiny triangle's inner ring must still be inside its outer ring.
        let centroid = 1e-3 / 3.0;
        for pair in model.vertices[..6].chunks(2) {
            let outer = pair[0].position;
            let inner = pair[1].position;
            let d_out = (outer[0] - centroid).hypot(outer[2] - centroid);
            let d_in = (inner[0] - centroid).hypot(inner[2] - centroid);
            assert!(d_in <= d_out + 1e-9, "inset crossed the centroid");
        }
    }

    #[test]
    fn stats_report_every_class_even_when_empty() {
        let s = stats(&[node(SurfaceKind::Wall, 2)]);
        assert_eq!(s.len(), SurfaceKind::ALL.len());
        let wall = s
            .iter()
            .find(|k| k.kind == SurfaceKind::Wall)
            .expect("wall");
        assert_eq!((wall.nodes, wall.meshes, wall.triangles), (1, 2, 2));
        let floor = s
            .iter()
            .find(|k| k.kind == SurfaceKind::Floor)
            .expect("floor");
        assert_eq!(floor.nodes, 0);
        assert!(floor.bounds.is_none());
    }

    #[test]
    fn bounds_cover_the_geometry_and_containment_is_directional() {
        let outer = bounds_of(&[node(SurfaceKind::Wall, 1)], is_collidable).expect("bounds");
        assert_eq!(outer, ([0.0, 0.0, 0.0], [4.0, 0.0, 4.0]));
        let inner = ([1.0, 0.0, 1.0], [2.0, 0.0, 2.0]);
        assert!(contains(outer, inner, 0.0));
        assert!(!contains(inner, outer, 0.0));
    }

    /// A degenerate triangle must not put a NaN into the buffer: one NaN
    /// position is enough to make a whole draw disappear on some drivers.
    #[test]
    fn a_degenerate_triangle_produces_no_nan() {
        let mut n = node(SurfaceKind::Floor, 1);
        n.geometry.meshes[0].vertices = vec![[1.0, 2.0, 3.0]; 3];
        let model = build_model("t", &[n], Style::Solid, false);
        assert!(
            model
                .vertices
                .iter()
                .all(|v| v.normal.iter().all(|c| c.is_finite()))
        );
    }
}
