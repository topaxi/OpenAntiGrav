//! Posing a flyer card at a moment and cutting it to a rectangle.
//!
//! **Why the card is cut at all.** A flyer's elements slide in from outside
//! the card and stop at poses that overhang it - one node's last key puts a
//! bar 90 units left of the card, another's a slanted strip 160 to its right -
//! and every settled RPCS3 frame shows a card with a clean rectangular body and
//! nothing beyond it. Whatever the original clips with is in the native
//! `Flyer` widget class, unread. What is measured is where the body stops:
//! a planar homography between a head-on render of the card and two settled
//! frames puts its edges at card-local `x` of about -48.3 (the stripes start
//! cut mid-hatch there), and `y` from -28.7 to +33.7 (the top and bottom edges
//! the frames show), which is a 0.835 scale of the authored 115 by 74.8
//! `bgplane` about a point 2.5 units above its centre. [`CARD_RECT`] is that
//! rectangle, **chosen, not measured** as a mechanism.
//!
//! **Why the pose is baked first.** A vertex under an `Anim Transform` is
//! stored in its node's own space and multiplied by the node's matrix in the
//! shader every frame, so its card-local position only exists once a time is
//! picked. The card is shown settled, so [`bake`] picks
//! [`super::settled_seconds`] once, multiplies those vertices through their
//! node matrices and clears the node index; [`clip`] then cuts the result in
//! card-local space with nothing left to move it.

use oag_render::mesh::{DrawCall, GpuVertex, Model};

/// The card body in card-local units, `[min_x, min_y, max_x, max_y]`.
///
/// **Chosen, not measured** - see the module doc for what was measured and
/// what was not. The right edge (`48.3`) is the left one mirrored: every
/// reference frame is cropped before it.
pub const CARD_RECT: [f32; 4] = [-48.3, -28.7, 48.3, 33.7];

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
