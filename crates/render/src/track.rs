//! Turns a decoded track spline into something the mesh renderer can draw.
//!
//! The point is to *see* whether the decode is right. A wrong spline does not
//! look slightly off; it looks like scribble. Rendering the driveable ribbon
//! rather than the track's art meshes also means this needs nothing from the
//! scene graph, so it works before `Transform` nodes are decoded.
//!
//! Three things are drawn:
//!
//! - the **track surface**, from each control point's half-widths, coloured by
//!   `section_id` so the visibility partition is visible too
//! - the **racing line**, a narrow strip along `racing_line`
//! - the **AI corridor** edges, as two more narrow strips
//!
//! The surface ribbon uses the positions as stored, which sit on the track
//! surface. The racing line and corridor strips are drawn
//! [`HOVER_LIFT`](track::HOVER_LIFT) above it, which is where the load pass puts
//! them and therefore where ships fly. Seeing that gap in the picture is the
//! point: it is the one transform the loader applies to the geometry.
//!
//! Widths are laid out along `lateral`, with negative offsets to the left. That
//! the negative side is *left* rather than right is a convention this
//! visualisation assumes; nothing here depends on it being that way round, and a
//! mirrored guess would only swap the two widths.

use anyhow::{Context, Result, bail};
use oag_vex::track::{self, AiTrack, Sample};
use oag_vex::vex;

use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model};

/// Curve samples per control-point interval.
///
/// Four is enough that the B-spline reads as a curve rather than a polyline at
/// screen scale, and keeps a whole track well under a million vertices.
const STEPS_PER_SEGMENT: usize = 4;

/// Half-width of the racing-line and corridor strips, in world units.
const STRIP_HALF_WIDTH: f32 = 0.6;

/// Loads a track's spline graph from a `.vex` model inside an archive.
pub fn load(spec: &str, name: &str) -> Result<(AiTrack, String)> {
    let data = oag_mesh::mesh::read_blob(spec, name)?;
    if !vex::has_magic(&data) {
        bail!("{name} is not a .vex file (no VEXX magic)");
    }

    let nodes = vex::nodes(&data).context("walking the node tree")?;
    let node =
        track::find_node(&data, &nodes).with_context(|| format!("{name} has no WO Track node"))?;

    let payload = data
        .get(node.payload())
        .context("the WO Track payload runs past the end of the file")?;
    let ai = track::parse(payload).map_err(|e| anyhow::anyhow!("{name}: {e}"))?;

    // The check that settled the layout, kept as a runtime assertion rather
    // than a comment: a parse that is one structure out cannot come out even.
    if ai.encoded_len() != payload.len() {
        bail!(
            "{name}: decoded {} bytes but the payload is {} -- the layout is wrong",
            ai.encoded_len(),
            payload.len()
        );
    }

    Ok((ai, name.to_string()))
}

/// Builds a drawable ribbon from a spline graph.
#[must_use]
pub fn build_model(label: &str, ai: &AiTrack) -> Model {
    let mut vertices: Vec<GpuVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for path in &ai.paths {
        let samples = sample_path(path);
        // Surface: left edge to right edge, coloured per section.
        strip(&mut vertices, &mut indices, &samples, |s| {
            (
                -s.half_width_left,
                s.half_width_right,
                0.0,
                section_colour(s.section_id),
            )
        });
        // The authored racing line.
        strip(&mut vertices, &mut indices, &samples, |s| {
            (
                s.racing_line - STRIP_HALF_WIDTH,
                s.racing_line + STRIP_HALF_WIDTH,
                track::HOVER_LIFT,
                [1.0, 0.85, 0.1, 1.0],
            )
        });
        // The AI corridor, which the load pass forces to straddle the racing
        // line by at least 0.1 either side.
        let corridor: [fn(&Sample) -> f32; 2] = [|s| s.ai_bound_left, |s| s.ai_bound_right];
        for edge in corridor {
            strip(&mut vertices, &mut indices, &samples, move |s| {
                let at = edge(s);
                (
                    at - STRIP_HALF_WIDTH * 0.5,
                    at + STRIP_HALF_WIDTH * 0.5,
                    track::HOVER_LIFT,
                    [0.2, 0.9, 0.6, 1.0],
                )
            });
        }
    }

    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for v in &vertices {
        for i in 0..3 {
            lo[i] = lo[i].min(v.position[i]);
            hi[i] = hi[i].max(v.position[i]);
        }
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

    // One draw call: the ribbon is untextured, so there is nothing to split on.
    // Bounds are the box's own circumscribing sphere - the diagonal, not
    // `radius` above, which is only half the longest single axis and would
    // under-cover the corners of a non-cubic box.
    let draws = vec![DrawCall {
        moving: false,
        // Synthetic: no batch, so no recovered blend class.
        blend: None,
        blend_state: None,
        // Synthetic: no mesh, so no derived layer.
        layer: vex::LAYER_DEFAULT,
        culled: false,
        range: 0..indices.len() as u32,
        texture: None,
        bounds: Bounds {
            centre,
            radius: (0..3)
                .map(|i| (hi[i] - centre[i]).powi(2))
                .sum::<f32>()
                .sqrt(),
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
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: Vec::new(),
        mesh_count: ai.paths.len(),
    }
}

/// Samples one path along its whole length.
fn sample_path(path: &track::Path) -> Vec<Sample> {
    let mut out = Vec::with_capacity(path.points.len() * STEPS_PER_SEGMENT);
    for segment in 0..path.points.len() {
        for step in 0..STEPS_PER_SEGMENT {
            let t = step as f32 / STEPS_PER_SEGMENT as f32;
            if let Some(s) = path.sample(segment, t) {
                out.push(s);
            }
        }
    }
    out
}

/// A point at a lateral `offset` from the centre line, raised by `lift`.
fn edges_of(sample: &Sample, offset: f32, lift: f32) -> [f32; 3] {
    let mut out = [0.0; 3];
    for (k, axis) in out.iter_mut().enumerate() {
        // `down` points into the surface, so subtracting raises.
        *axis = sample.pos[k] + sample.lateral[k] * offset - sample.down[k] * lift;
    }
    out
}

/// Emits a triangle strip whose two edges come from `edges`.
///
/// `edges` returns `(left offset, right offset, height above the surface,
/// colour)`, all in the sample's own frame.
fn strip(
    vertices: &mut Vec<GpuVertex>,
    indices: &mut Vec<u32>,
    samples: &[Sample],
    edges: impl Fn(&Sample) -> (f32, f32, f32, [f32; 4]),
) {
    if samples.len() < 2 {
        return;
    }
    let base = vertices.len() as u32;
    for s in samples {
        let (left, right, lift, colour) = edges(s);
        // The surface faces the way the frame says is up, which is `-down`.
        let normal = [-s.down[0], -s.down[1], -s.down[2]];
        vertices.push(GpuVertex {
            position: edges_of(s, left, lift),
            normal,
            colour,
            texcoord: [0.0, 0.0],
            lightmap_texcoord: [0.0, 0.0],
            lit: 1.0,
            anim: 0,
            xform: 0,
            sun_mask: 1.0,
            slots: oag_mesh::mesh::slots::DEFAULT,
            specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
            glow: 0.0,
            texcoord2: [0.0, 0.0],
        });
        vertices.push(GpuVertex {
            position: edges_of(s, right, lift),
            normal,
            colour,
            texcoord: [1.0, 0.0],
            lightmap_texcoord: [0.0, 0.0],
            lit: 1.0,
            anim: 0,
            xform: 0,
            sun_mask: 1.0,
            slots: oag_mesh::mesh::slots::DEFAULT,
            specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
            glow: 0.0,
            texcoord2: [0.0, 0.0],
        });
    }
    for i in 0..samples.len() as u32 - 1 {
        let a = base + i * 2;
        indices.extend([a, a + 1, a + 2, a + 2, a + 1, a + 3]);
    }
}

/// A distinct colour per visibility section, so the partition is visible.
///
/// A golden-ratio hue step keeps neighbouring sections far apart in hue no
/// matter how many there are.
fn section_colour(section: u8) -> [f32; 4] {
    let hue = (f32::from(section) * 0.618_034).fract();
    let (r, g, b) = hsv_to_rgb(hue, 0.45, 0.9);
    [r, g, b, 1.0]
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    match i as i32 % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal straight path, four points along +x.
    fn straight() -> AiTrack {
        let counts = [6usize];
        let reserved = track::RESERVED_LEN;
        let paths_at = track::HEADER_LEN + reserved;
        let junctions_at = paths_at + track::PATH_LEN;
        let points_at = junctions_at + track::JUNCTION_LEN;
        let mut out = vec![0u8; points_at + counts[0] * track::POINT_LEN];
        out[0..4].copy_from_slice(&track::MAGIC.to_le_bytes());
        out[4..8].copy_from_slice(&0x105u32.to_le_bytes());
        out[8..12].copy_from_slice(&1u32.to_le_bytes());
        out[12..16].copy_from_slice(&1u32.to_le_bytes());
        out[paths_at..paths_at + 4].copy_from_slice(&(counts[0] as u32).to_le_bytes());
        for at in [paths_at + 0x0c, paths_at + 0x10] {
            out[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
        }
        for k in 0..4 {
            let at = junctions_at + k * 4;
            out[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
        }
        for k in 0..counts[0] {
            let at = points_at + k * track::POINT_LEN;
            let put = |out: &mut Vec<u8>, off: usize, v: f32| {
                out[at + off..at + off + 4].copy_from_slice(&v.to_le_bytes());
            };
            put(&mut out, 0x00, k as f32 * 5.0);
            put(&mut out, 0x10, 1.0);
            put(&mut out, 0x24, -1.0);
            put(&mut out, 0x38, 1.0);
            put(&mut out, 0x44, 10.0);
            put(&mut out, 0x48, 10.0);
            put(&mut out, 0x54, 0.0);
        }
        track::parse(&out).expect("parse")
    }

    #[test]
    fn a_straight_path_builds_a_flat_ribbon() {
        let model = build_model("straight", &straight());
        assert!(!model.vertices.is_empty());
        assert_eq!(model.indices.len() % 3, 0);
        // Flat in y apart from the hover lift on the line and corridor.
        let ys: Vec<f32> = model.vertices.iter().map(|v| v.position[1]).collect();
        let lo = ys.iter().copied().fold(f32::MAX, f32::min);
        let hi = ys.iter().copied().fold(f32::MIN, f32::max);
        assert!((hi - lo - track::HOVER_LIFT).abs() < 1e-4, "{lo} to {hi}");
    }

    /// The surface strip must span the declared width, not some fraction of it.
    #[test]
    fn the_ribbon_is_as_wide_as_the_track() {
        let model = build_model("straight", &straight());
        let zs: Vec<f32> = model.vertices.iter().map(|v| v.position[2]).collect();
        let lo = zs.iter().copied().fold(f32::MAX, f32::min);
        let hi = zs.iter().copied().fold(f32::MIN, f32::max);
        assert!((hi - lo - 20.0).abs() < 1e-3, "width came out {}", hi - lo);
    }

    #[test]
    fn every_index_is_in_range() {
        let model = build_model("straight", &straight());
        let n = model.vertices.len() as u32;
        assert!(model.indices.iter().all(|&i| i < n));
    }
}
