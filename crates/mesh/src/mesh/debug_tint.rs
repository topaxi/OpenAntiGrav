//! A diagnostic that paints each unit of the blended list its own flat colour.
//!
//! **Off unless `OAG_DEBUG_TINT_BLENDED` is set, and never a rendering decision.** The
//! blended list (list B: trees, translucent floor patches, glass) writes no depth, so what
//! lies over what is decided by draw order alone; a tree drawn over a nearer one is
//! plain to see when the two are different colours and invisible when both are foliage.
//! `docs/rendering/draw-order.md`, "Trees overlapping trees".
//!
//! The value names the unit the colour changes at:
//!
//! - `draw` - one colour per [`Model::transparent_draws`] entry (one batch of one mesh, the unit
//!   the original's queue key orders);
//! - `tree` - one colour per connected group of triangles inside a draw (vertices welded by
//!   position), which is what a player calls one tree.
//!
//! The vertex colour is replaced and the light switched off, and the texture of every blended
//! draw is whitened with its alpha kept, so the silhouette survives and the colour is the
//! flat one. A texture that is already on the GPU cannot be whitened here and is left as it
//! is, so the hue then multiplies the picture.

use super::{DrawCall, GpuVertex, Model, ModelTexture, Texels};
use std::sync::{Arc, OnceLock};

/// What `OAG_DEBUG_TINT_BLENDED` asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// One colour per blended draw.
    Draw,
    /// One colour per connected triangle group inside a draw.
    Tree,
}

/// The unit asked for in the environment, read once.
#[must_use]
pub fn requested() -> Option<Unit> {
    static UNIT: OnceLock<Option<Unit>> = OnceLock::new();
    *UNIT.get_or_init(|| {
        let value = std::env::var("OAG_DEBUG_TINT_BLENDED").ok()?;
        match value.trim() {
            "" | "0" => None,
            "tree" => Some(Unit::Tree),
            _ => Some(Unit::Draw),
        }
    })
}

/// Colour number `n`: a golden-ratio walk round the hue wheel at full saturation, so numbers
/// that are neighbours in the list are far apart on the wheel.
fn colour(n: usize) -> [f32; 3] {
    let hue = (n as f32 * 0.618_034).fract() * 6.0;
    let sector = hue as u32;
    let f = hue - sector as f32;
    match sector {
        0 => [1.0, f, 0.0],
        1 => [1.0 - f, 1.0, 0.0],
        2 => [0.0, 1.0, f],
        3 => [0.0, 1.0 - f, 1.0],
        4 => [f, 0.0, 1.0],
        _ => [1.0, 0.0, 1.0 - f],
    }
}

impl Model {
    /// Paints each unit of [`Self::transparent_draws`] its own flat colour, per [`requested`].
    /// Returns how many colours were handed out.
    pub fn debug_tint_blended(&mut self, unit: Unit) -> usize {
        let mut handed = 0usize;
        let mut whitened: Vec<usize> = Vec::new();
        for (index, draw) in self.transparent_draws.iter().enumerate() {
            if let Some(texture) = draw.texture
                && !whitened.contains(&texture)
            {
                whitened.push(texture);
            }
            let groups = match unit {
                Unit::Draw => vec![(draw.range.clone(), index)],
                Unit::Tree => tree_groups(draw, &self.indices, &self.vertices, handed),
            };
            for (range, colour) in groups {
                handed = handed.max(colour + 1);
                let [r, g, b] = self::colour(colour);
                for &at in &self.indices[range.start as usize..range.end as usize] {
                    let vertex = &mut self.vertices[at as usize];
                    let alpha = vertex.colour[3];
                    vertex.colour = [r, g, b, alpha];
                    vertex.lit = 0.0;
                }
            }
        }
        for texture in whitened {
            if let Some(Some(picture)) = self.textures.get(texture) {
                if let Some(white) = whitened_copy(picture) {
                    self.textures[texture] = Some(Arc::new(white));
                }
            }
        }
        handed
    }
}

/// `(index range, colour number)` for each connected group of `draw`'s triangles, numbered from
/// `first`. A group is a run of triangles whose welded vertices are linked, so the ranges are
/// not contiguous in general; every triangle becomes a range of its own, three indices wide.
fn tree_groups(
    draw: &DrawCall,
    indices: &[u32],
    vertices: &[GpuVertex],
    first: usize,
) -> Vec<(std::ops::Range<u32>, usize)> {
    let weld = |at: u32| {
        let p = vertices[at as usize].position;
        [
            (p[0] * 64.0).round() as i64,
            (p[1] * 64.0).round() as i64,
            (p[2] * 64.0).round() as i64,
        ]
    };
    let mut parent: Vec<usize> = Vec::new();
    let mut seen: std::collections::HashMap<[i64; 3], usize> = std::collections::HashMap::new();
    fn find(parent: &mut [usize], mut at: usize) -> usize {
        while parent[at] != at {
            parent[at] = parent[parent[at]];
            at = parent[at];
        }
        at
    }
    let triangles = (draw.range.start..draw.range.end).step_by(3);
    let mut owners = Vec::new();
    for start in triangles.clone() {
        let mut owner: Option<usize> = None;
        for corner in 0..3 {
            let key = weld(indices[(start + corner) as usize]);
            let node = *seen.entry(key).or_insert_with(|| {
                parent.push(parent.len());
                parent.len() - 1
            });
            match owner {
                None => owner = Some(node),
                Some(first_node) => {
                    let (a, b) = (find(&mut parent, first_node), find(&mut parent, node));
                    parent[a] = b;
                }
            }
        }
        owners.push(owner.unwrap_or(0));
    }
    let mut numbered: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    triangles
        .zip(owners)
        .map(|(start, owner)| {
            let root = find(&mut parent, owner);
            let next = first + numbered.len();
            let colour = *numbered.entry(root).or_insert(next);
            (start..start + 3, colour)
        })
        .collect()
}

/// `picture` with every texel's colour set to white and its alpha kept, or `None` when the texels
/// are not on the CPU.
fn whitened_copy(picture: &ModelTexture) -> Option<ModelTexture> {
    fn white(bytes: &[u8]) -> Vec<u8> {
        bytes
            .chunks_exact(4)
            .flat_map(|texel| [255, 255, 255, texel[3]])
            .collect()
    }
    let texels = match &picture.texels {
        Texels::Rgba8(bytes) => Texels::Rgba8(white(bytes)),
        Texels::Chain(levels) => Texels::Chain(levels.iter().map(|l| white(l)).collect()),
        _ => return None,
    };
    Some(ModelTexture {
        label: picture.label.clone(),
        width: picture.width,
        height: picture.height,
        texels,
        mip_count: picture.mip_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::{Bounds, DEFAULT_SPECULAR_EXPONENT, slots};
    use std::sync::Arc;

    fn vertex(x: f32, y: f32) -> GpuVertex {
        GpuVertex {
            quake: 0.0,
            position: [x, y, 0.0],
            normal: [0.0, 1.0, 0.0],
            colour: [0.2, 0.4, 0.6, 0.5],
            texcoord: [0.0; 2],
            lightmap_texcoord: [0.0; 2],
            lit: 1.0,
            anim: 0,
            xform: 0,
            sun_mask: 1.0,
            slots: slots::DEFAULT,
            specular_exponent: DEFAULT_SPECULAR_EXPONENT,
            glow: 0.0,
            texcoord2: [0.0; 2],
            tangent: 0,
        }
    }

    fn draw(range: std::ops::Range<u32>) -> DrawCall {
        DrawCall {
            range,
            texture: Some(0),
            bounds: Bounds {
                centre: [0.0; 3],
                radius: 1.0,
            },
            moving: false,
            culled: false,
            blend: None,
            blend_state: None,
            depth_point: None,
            layer: 0,
            alpha_test_ref: None,
            writes_depth: false,
            node: None,
            chunk: None,
        }
    }

    /// Three triangles in two draws: the first draw's two are far apart, the second's is one
    /// of its own. Vertices 0-2, 3-5 and 6-8 are those triangles.
    fn fixture() -> Model {
        let mut model = Model::none("tint");
        let corners = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        for shift in [0.0, 10.0, 20.0] {
            for (x, y) in corners {
                model.vertices.push(vertex(x + shift, y));
            }
        }
        model.indices = (0..9).collect();
        model.transparent_draws = vec![draw(0..6), draw(6..9)];
        model.textures = vec![Some(Arc::new(ModelTexture {
            label: "picture".into(),
            width: 1,
            height: 1,
            texels: Texels::Rgba8(vec![10, 20, 30, 77]),
            mip_count: None,
        }))];
        model
    }

    fn colours(model: &Model) -> Vec<[f32; 3]> {
        (0..3)
            .map(|triangle| {
                let c = model.vertices[triangle * 3].colour;
                [c[0], c[1], c[2]]
            })
            .collect()
    }

    /// Each draw is one colour, two draws are two colours, the vertex alpha and the texture's
    /// alpha survive, the light is off, and the picture is white.
    #[test]
    fn a_draw_is_one_flat_colour_and_the_alpha_is_kept() {
        let mut model = fixture();
        assert_eq!(model.debug_tint_blended(Unit::Draw), 2);
        let c = colours(&model);
        assert_eq!(c[0], c[1], "both triangles of the first draw");
        assert_ne!(c[0], c[2], "the second draw is another colour");
        assert!(
            model
                .vertices
                .iter()
                .all(|v| v.lit == 0.0 && v.colour[3] == 0.5)
        );
        let Some(Some(picture)) = model.textures.first() else {
            panic!("the picture is still there");
        };
        let Texels::Rgba8(texels) = &picture.texels else {
            panic!("still straight RGBA8");
        };
        assert_eq!(texels, &[255, 255, 255, 77]);
    }

    /// `tree` splits a draw by connected triangles: the first draw's two separate triangles are
    /// two colours, and the colours run on into the next draw.
    #[test]
    fn a_tree_is_a_connected_group_of_triangles() {
        let mut model = fixture();
        assert_eq!(model.debug_tint_blended(Unit::Tree), 3);
        let c = colours(&model);
        assert_ne!(c[0], c[1]);
        assert_ne!(c[1], c[2]);
        assert_ne!(c[0], c[2]);
        // A triangle that shares an edge with the first one joins its group.
        let mut model = fixture();
        model.vertices[3] = vertex(1.0, 0.0);
        model.vertices[4] = vertex(0.0, 1.0);
        model.vertices[5] = vertex(1.0, 1.0);
        model.debug_tint_blended(Unit::Tree);
        let c = colours(&model);
        assert_eq!(c[0], c[1]);
    }
}
