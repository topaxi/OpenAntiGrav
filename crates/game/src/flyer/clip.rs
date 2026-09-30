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
use oag_render::mesh::{DrawCall, GpuVertex, Model};

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
/// `camera_to_world` is row-major with the translation in row 3 - the
/// convention `oag_vex::vex` documents - and a vertex at or behind the camera
/// is held a hair in front of it rather than sent through infinity.
pub fn flatten(model: &mut Model, camera_to_world: &[f32; 16], window_tan: f32, card_height: f32) {
    let view = Mat4::from_cols_array(camera_to_world).inverse();
    for vertex in &mut model.vertices {
        let eye = view.transform_point3(Vec3::from(vertex.position));
        let depth = (-eye.z).max(1e-3);
        let scale = card_height / 2.0 / window_tan / depth;
        vertex.position = [eye.x * scale, eye.y * scale, -depth * DEPTH_ORDER];
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
    let mut vertices = Vec::with_capacity(model.vertices.len());
    let mut indices = Vec::with_capacity(model.indices.len());
    let source = std::mem::take(&mut model.vertices);
    let source_indices = std::mem::take(&mut model.indices);
    let mut cut = |draw: &mut DrawCall| {
        let first = u32::try_from(indices.len()).unwrap_or(u32::MAX);
        let range = draw.range.start as usize..draw.range.end as usize;
        for triangle in source_indices[range].as_chunks::<3>().0 {
            let corners = triangle.map(|i| source[i as usize]);
            let polygon = clip_polygon(corners.to_vec(), rect);
            if polygon.len() < 3 {
                continue;
            }
            let base = u32::try_from(vertices.len()).unwrap_or(u32::MAX);
            vertices.extend(polygon.iter().copied());
            for k in 1..polygon.len() - 1 {
                let k = u32::try_from(k).unwrap_or(u32::MAX);
                indices.extend([base, base + k, base + k + 1]);
            }
        }
        draw.range = first..u32::try_from(indices.len()).unwrap_or(u32::MAX);
    };
    for draw in &mut model.draws {
        cut(draw);
    }
    for draw in &mut model.alpha_tested_draws {
        cut(draw);
    }
    for draw in &mut model.transparent_draws {
        cut(draw);
    }
    model.vertices = vertices;
    model.indices = indices;
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
