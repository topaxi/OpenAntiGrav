//! Turning a flyer's layered scene into the flat picture its card shows.
//!
//! **A card is a picture, not a scene.** Every flyer is composed for its own
//! camera (`camera1` in its `.vex`, [`oag_vex::camera`]) the way a still is
//! composed for a lens: the Fury cards are stacks of layers from 20 units
//! behind the card plane to 8 in front of it, and the elements of every card
//! slide to poses that overhang its frame. RPCS3 shows none of that depth and
//! none of that overhang - a Fury card turned on the screen keeps its wordmark
//! and its frame in the proportions a head-on render gives them, which a
//! rotated 3-D stack does not (the wordmark, 8 units nearer than the frame,
//! would slide sideways against it by 2 units at the settled yaw), and nothing
//! sits outside a rectangle that is the same on every flyer. So the card is
//! the camera's image, put on a flat rectangle: [`flatten`] is that image,
//! computed vertex by vertex, and [`clip`] cuts it to the rectangle.
//!
//! Every layer of a flyer is a quad at one depth, parallel to the image, so
//! its image is a scaled copy of it and the flattening is affine per
//! triangle: texture coordinates need no correction.
//!
//! **Why the pose is baked first.** A vertex under an `Anim Transform` is
//! stored in its node's own space and multiplied by the node's matrix in the
//! shader every frame, so its position only exists once a time is picked. The
//! card is shown at one moment, so [`bake`] picks it once, multiplies those
//! vertices through their node matrices and clears the node index.

use oag_core::math::{Mat4, Vec3};
use oag_mesh::mesh::{DrawCall, GpuVertex, Model};

/// Multiplies every animated vertex through its node's matrix at `seconds`
/// and marks it static.
///
/// The matrices are row-major with the translation in row 3, the convention
/// `oag_vex::vex` documents and `Model::sample_anim_nodes` answers in.
pub fn bake(model: &mut Model, seconds: f32) {
    let matrices = model.sample_anim_nodes(seconds);
    for vertex in &mut model.vertices {
        let Some(matrix) = vertex
            .xform
            .checked_sub(1)
            .and_then(|slot| matrices.get(slot as usize))
        else {
            continue;
        };
        let [x, y, z] = vertex.position;
        vertex.position = std::array::from_fn(|k| {
            x * matrix[k] + y * matrix[4 + k] + z * matrix[8 + k] + matrix[12 + k]
        });
        vertex.xform = 0;
    }
    for draw in model
        .draws
        .iter_mut()
        .chain(&mut model.alpha_tested_draws)
        .chain(&mut model.transparent_draws)
    {
        draw.moving = false;
    }
}

/// Replaces every vertex by where the camera at `camera_to_world` sees it,
/// in the units of the card the picture is shown on.
///
/// The camera is a pinhole with a vertical half-angle of `window_tan`
/// (tangent), and the card is `card_height` units tall: a point at the top of
/// the camera's window lands `card_height / 2` above the card's centre, so
/// `u = x / -z / window_tan * card_height / 2` in the camera's own space, and
/// likewise `v`. The depth is kept only as an ordering, a hundredth of a unit
/// per unit of distance, so the layers still sort the way the camera sees
/// them.
///
/// `stretch` widens the picture against its height, `1.0` for none.
///
/// `camera_to_world` is row-major with the translation in row 3 - the
/// convention `oag_vex::vex` documents - and a vertex at or behind the camera
/// is held a hair in front of it rather than sent through infinity.
pub fn flatten(
    model: &mut Model,
    camera_to_world: &[f32; 16],
    window_tan: f32,
    card_height: f32,
    stretch: f32,
) {
    let view = Mat4::from_cols_array(camera_to_world).inverse();
    for vertex in &mut model.vertices {
        let eye = view.transform_point3(Vec3::from(vertex.position));
        let depth = (-eye.z).max(1e-3);
        let scale = card_height / 2.0 / window_tan / depth;
        vertex.position = [eye.x * scale * stretch, eye.y * scale, -depth * DEPTH_ORDER];
    }
}

/// How far the flattened layers are spread in depth, per unit of their
/// distance from the camera. Far enough for the depth test to separate two
/// layers, near enough that no yaw shows it.
const DEPTH_ORDER: f32 = 0.01;

/// Cuts every triangle of every draw to `rect` (`[min_x, min_y, max_x,
/// max_y]`, in the vertices' own space), in place.
///
/// Sutherland-Hodgman against the four edges, fanned back into triangles.
/// Position, normal, colour and both coordinate sets interpolate; everything
/// else on a vertex is a per-draw constant (material roles, texture-animation
/// track, exponent) and is copied from the triangle's first corner. Draw
/// order, textures and blend state are untouched - only each draw's index
/// range moves.
pub fn clip(model: &mut Model, rect: [f32; 4]) {
    rebuild(model, |polygon, out| {
        let piece = clip_polygon(polygon.clone(), rect);
        if piece.len() >= 3 {
            out.push(piece);
        }
    });
}

/// Cuts every triangle of every draw to the union of `shape`'s triangles,
/// in place - the card's own outline, which a rectangle is not: the placeholder's
/// front face is a rectangle with a chamfered corner and a few notches.
///
/// `shape` is a partition (no two triangles overlap: the front face's twenty-five
/// triangles sum to the polygon's area), so a triangle cut against each of them
/// in turn is cut exactly once wherever it lies. Each triangle is wound
/// counter-clockwise first. Everything else is [`clip`]'s: attributes
/// interpolate, per-draw constants ride through.
pub fn clip_to_shape(model: &mut Model, shape: &[[[f32; 2]; 3]]) {
    let planes: Vec<[[f32; 3]; 3]> = shape
        .iter()
        .map(|triangle| edge_planes(*triangle))
        .collect();
    rebuild(model, |polygon, out| {
        for triangle in &planes {
            let piece = triangle
                .iter()
                .fold(polygon.clone(), |piece, plane| clip_by_plane(piece, *plane));
            if piece.len() >= 3 {
                out.push(piece);
            }
        }
    });
}

/// The three inward half-planes `a*x + b*y + c >= 0` of a triangle, wound
/// counter-clockwise first.
fn edge_planes(mut triangle: [[f32; 2]; 3]) -> [[f32; 3]; 3] {
    let [p, q, r] = triangle;
    if (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]) < 0.0 {
        triangle.swap(1, 2);
    }
    std::array::from_fn(|i| {
        let from = triangle[i];
        let to = triangle[(i + 1) % 3];
        let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
        [-dy, dx, dy * from[0] - dx * from[1]]
    })
}

/// One polygon cut to the side of `plane` (`[a, b, c]`, `a*x + b*y + c >= 0`)
/// that is inside.
fn clip_by_plane(polygon: Vec<GpuVertex>, plane: [f32; 3]) -> Vec<GpuVertex> {
    let side = |v: &GpuVertex| plane[0] * v.position[0] + plane[1] * v.position[1] + plane[2];
    let mut next = Vec::with_capacity(polygon.len() + 2);
    for (i, current) in polygon.iter().enumerate() {
        let previous = &polygon[(i + polygon.len() - 1) % polygon.len()];
        let (was, is) = (side(previous), side(current));
        if (was >= 0.0) != (is >= 0.0) {
            next.push(lerp_vertex(previous, current, was / (was - is)));
        }
        if is >= 0.0 {
            next.push(*current);
        }
    }
    next
}

/// The vertex a fraction `t` of the way from `from` to `to`, every
/// interpolated attribute carried there and the per-draw constants from
/// `from`.
fn lerp_vertex(from: &GpuVertex, to: &GpuVertex, t: f32) -> GpuVertex {
    let lerp = |a: f32, b: f32| a + (b - a) * t;
    let mut out = *from;
    out.position = std::array::from_fn(|k| lerp(from.position[k], to.position[k]));
    out.normal = std::array::from_fn(|k| lerp(from.normal[k], to.normal[k]));
    out.colour = std::array::from_fn(|k| lerp(from.colour[k], to.colour[k]));
    out.texcoord = std::array::from_fn(|k| lerp(from.texcoord[k], to.texcoord[k]));
    out.lightmap_texcoord =
        std::array::from_fn(|k| lerp(from.lightmap_texcoord[k], to.lightmap_texcoord[k]));
    out.sun_mask = lerp(from.sun_mask, to.sun_mask);
    out
}

/// Rebuilds every draw's triangles through `cut`, which turns one triangle's
/// polygon into the polygons that replace it.
fn rebuild(model: &mut Model, cut: impl Fn(&Vec<GpuVertex>, &mut Vec<Vec<GpuVertex>>)) {
    let mut vertices = Vec::with_capacity(model.vertices.len());
    let mut indices = Vec::with_capacity(model.indices.len());
    let source = std::mem::take(&mut model.vertices);
    let source_indices = std::mem::take(&mut model.indices);
    let mut apply = |draw: &mut DrawCall| {
        let first = u32::try_from(indices.len()).unwrap_or(u32::MAX);
        let range = draw.range.start as usize..draw.range.end as usize;
        for triangle in source_indices[range].as_chunks::<3>().0 {
            let polygon: Vec<GpuVertex> = triangle.iter().map(|&i| source[i as usize]).collect();
            let mut pieces = Vec::new();
            cut(&polygon, &mut pieces);
            for piece in pieces {
                let base = u32::try_from(vertices.len()).unwrap_or(u32::MAX);
                vertices.extend(piece.iter().copied());
                for k in 1..piece.len() - 1 {
                    let k = u32::try_from(k).unwrap_or(u32::MAX);
                    indices.extend([base, base + k, base + k + 1]);
                }
            }
        }
        draw.range = first..u32::try_from(indices.len()).unwrap_or(u32::MAX);
    };
    for draw in &mut model.draws {
        apply(draw);
    }
    for draw in &mut model.alpha_tested_draws {
        apply(draw);
    }
    for draw in &mut model.transparent_draws {
        apply(draw);
    }
    model.vertices = vertices;
    model.indices = indices;
}

/// How the card's reflection fades: the placeholder's own reflect surface
/// (`card_reflectShape`) runs from the card's bottom edge down half the
/// card's height, and its vertex alpha falls linearly from `top_alpha` at the
/// edge to nothing at the bottom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fade {
    /// The card's bottom edge, in the card's own units.
    pub edge_y: f32,
    /// How far below the edge the reflection reaches.
    pub depth: f32,
    /// The alpha at the edge.
    pub top_alpha: f32,
}

/// Adds the card's reflection: a mirrored copy of the part of the picture within
/// [`Fade::depth`] of the bottom edge, drawn alpha-blended with its vertex alpha
/// falling from [`Fade::top_alpha`] at the edge to zero at the depth.
///
/// One new alpha-blended draw per draw of the picture that has any triangle in
/// the band, with the source draw's texture and (reversed) winding.
pub fn reflect(model: &mut Model, fade: Fade) {
    let bottom = fade.edge_y - fade.depth;
    let mirrored = |v: &GpuVertex| {
        let mut out = *v;
        out.position[1] = 2.0 * fade.edge_y - v.position[1];
        out.normal[1] = -v.normal[1];
        out
    };
    let mut vertices = model.vertices.clone();
    let mut indices = model.indices.clone();
    let mut added = Vec::new();
    let sources: Vec<DrawCall> = model
        .draws
        .iter()
        .chain(&model.alpha_tested_draws)
        .chain(&model.transparent_draws)
        .cloned()
        .collect();
    for draw in sources {
        let first = u32::try_from(indices.len()).unwrap_or(u32::MAX);
        let range = draw.range.start as usize..draw.range.end as usize;
        for triangle in model.indices[range].as_chunks::<3>().0 {
            let mut polygon: Vec<GpuVertex> = triangle
                .iter()
                .rev()
                .map(|&i| mirrored(&model.vertices[i as usize]))
                .collect();
            for plane in [[0.0, 1.0, -bottom], [0.0, -1.0, fade.edge_y]] {
                polygon = clip_by_plane(polygon, plane);
                if polygon.is_empty() {
                    break;
                }
            }
            if polygon.len() < 3 {
                continue;
            }
            for v in &mut polygon {
                let along = ((v.position[1] - bottom) / fade.depth).clamp(0.0, 1.0);
                v.colour[3] *= fade.top_alpha * along;
            }
            let base = u32::try_from(vertices.len()).unwrap_or(u32::MAX);
            vertices.extend(polygon.iter().copied());
            for k in 1..polygon.len() - 1 {
                let k = u32::try_from(k).unwrap_or(u32::MAX);
                indices.extend([base, base + k, base + k + 1]);
            }
        }
        let end = u32::try_from(indices.len()).unwrap_or(u32::MAX);
        if end > first {
            added.push(DrawCall {
                range: first..end,
                blend: Some(oag_vex::vex::BlendClass::AlphaOver),
                blend_state: None,
                ..draw
            });
        }
    }
    model.vertices = vertices;
    model.indices = indices;
    model.transparent_draws.extend(added);
}

/// One polygon cut to `rect` on all four edges.
fn clip_polygon(mut polygon: Vec<GpuVertex>, rect: [f32; 4]) -> Vec<GpuVertex> {
    // `(axis, bound, keep_above)`: keep the side of `bound` on `axis` that
    // `keep_above` names.
    let edges = [
        (0, rect[0], true),
        (0, rect[2], false),
        (1, rect[1], true),
        (1, rect[3], false),
    ];
    for (axis, bound, keep_above) in edges {
        let inside = |v: &GpuVertex| {
            if keep_above {
                v.position[axis] >= bound
            } else {
                v.position[axis] <= bound
            }
        };
        let mut next = Vec::with_capacity(polygon.len() + 2);
        for (i, current) in polygon.iter().enumerate() {
            let previous = &polygon[(i + polygon.len() - 1) % polygon.len()];
            match (inside(previous), inside(current)) {
                (true, true) => next.push(*current),
                (true, false) => next.push(cross(previous, current, axis, bound)),
                (false, true) => {
                    next.push(cross(previous, current, axis, bound));
                    next.push(*current);
                }
                (false, false) => {}
            }
        }
        polygon = next;
        if polygon.is_empty() {
            break;
        }
    }
    polygon
}

/// Where the edge `from` to `to` crosses `bound` on `axis`, with every
/// interpolated attribute carried there. Per-draw constants come from `from`.
fn cross(from: &GpuVertex, to: &GpuVertex, axis: usize, bound: f32) -> GpuVertex {
    let span = to.position[axis] - from.position[axis];
    let t = if span == 0.0 {
        0.0
    } else {
        (bound - from.position[axis]) / span
    };
    let lerp = |a: f32, b: f32| a + (b - a) * t;
    let mut out = *from;
    out.position = std::array::from_fn(|k| lerp(from.position[k], to.position[k]));
    out.normal = std::array::from_fn(|k| lerp(from.normal[k], to.normal[k]));
    out.colour = std::array::from_fn(|k| lerp(from.colour[k], to.colour[k]));
    out.texcoord = std::array::from_fn(|k| lerp(from.texcoord[k], to.texcoord[k]));
    out.lightmap_texcoord =
        std::array::from_fn(|k| lerp(from.lightmap_texcoord[k], to.lightmap_texcoord[k]));
    out.sun_mask = lerp(from.sun_mask, to.sun_mask);
    // The axis value is the bound itself, not a rounded interpolation of it.
    out.position[axis] = bound;
    out
}

#[cfg(test)]
mod tests;
