//! Pulse's **caustic pass**: a batch whose header flags carry `0x04` is drawn a second time
//! with one of sixteen ripple frames, its texture coordinate generated from the vertex position,
//! tinted by a per-mesh byte (`docs/ghidra/functions/psp-pulse-usa/caustic-pass.md`,
//! `docs/rendering/water.md`, "Pulse: textures, and a caustic pass on Vostok Reef").
//!
//! This is the CPU half: which draws, their coordinates and tint, and the frame the clock picks.
//! The pass is drawn by `oag_raceplay`, as the circuit's own shine pass is.

use std::sync::Arc;

use super::{DrawCall, GpuVertex, Model, ModelTexture, shine_pass::plain};

/// Frames the clock steps through per second (`Texture_BindCausticFrame`: `track_time * 30`).
pub const FRAME_RATE: f32 = 30.0;

/// Frames in the loop; the index is `& 0xf`.
pub const FRAMES: usize = 16;

/// Texture units per world unit (`g_caustic_uv_scale`, `0x0890ddd8`'s matrix): one tile every
/// 71.4 units.
pub const UV_SCALE: f32 = 0.014;

/// The tint, red, green, blue (`g_caustic_tint`), before the mesh's byte scales it.
pub const TINT: [f32; 3] = [0.2, 1.0, 0.8];

/// One batch of the pass: its draw, and what its mesh node contributes.
#[derive(Debug, Clone)]
pub struct CausticDraw {
    /// The batch's own draw, the same index range.
    pub draw: DrawCall,
    /// The node's world translation, which the pass's matrix drops
    /// (`M.column3 = (0, 0, 0, 1)`) so the coordinate is the node-local position rotated by
    /// the node's own rotation.
    pub origin: [f32; 3],
    /// The mesh payload's byte at `+0x0e`, the tint's scale.
    pub scale: u8,
}

/// What a model carries of the pass: the flagged batches, and the frames once a loader has read
/// them (`frames` is empty until then, and the pass draws nothing without all sixteen).
#[derive(Debug, Clone, Default)]
pub struct Caustic {
    pub draws: Vec<CausticDraw>,
    pub frames: Vec<Arc<ModelTexture>>,
}

/// The frame drawn at world time `seconds`: `(int)(seconds * 30) & 0xf`.
#[must_use]
pub fn frame(seconds: f32) -> usize {
    ((seconds * FRAME_RATE) as i32 & (FRAMES as i32 - 1)) as usize
}

/// The disc's name of frame `index`: the pass's table holds the odd files, `save.01` to
/// `save.31` (`docs/ghidra/functions/psp-pulse-usa/loading-screen.md`, "The caustics are not
/// this"). Resolve it through `oag_pulse::read_image` (the PS2 disc keeps it as `.pct`).
#[must_use]
pub fn frame_name(index: usize) -> String {
    format!(r"Data\Tex\caustics\save.{:02}.mip", index * 2 + 1)
}

/// The pass's colour for a mesh whose byte is `scale`: `Gu_Ambient` takes
/// `(int)(tint * byte)` per channel in eight bits.
#[must_use]
pub fn colour(scale: u8) -> [f32; 4] {
    let f = f32::from(scale);
    let channel = |t: f32| f32::from((t * f) as u8) / 255.0;
    [channel(TINT[0]), channel(TINT[1]), channel(TINT[2]), 1.0]
}

/// The texture coordinate of a vertex at `position` (circuit space) under a node whose world
/// translation is `origin`: `scale(0.014) * quarter turn` of the position the node's rotation
/// gives, which looks straight down the vertical axis.
///
/// The quarter turn is about the horizontal axis, so `(x, y, z)` lands at `(x, -z, y)` and the
/// coordinate is the ground-plane position. Its sense is not read from the executable; it
/// mirrors the pattern, which has no handedness to show.
#[must_use]
pub fn uv(position: [f32; 3], origin: [f32; 3]) -> [f32; 2] {
    [
        UV_SCALE * (position[0] - origin[0]),
        -UV_SCALE * (position[2] - origin[2]),
    ]
}

/// A circuit's pass, ready for a drawable.
#[derive(Debug)]
pub struct Pass {
    /// Every frame's draws one after the other, each frame's the same ranges over the same
    /// vertices with its own picture; unlit, tinted, blended additively by the picture's alpha.
    pub model: Model,
    /// For each of `model.draws`, the circuit draw it redraws (the key its section mask and
    /// frustum bound are looked up by).
    pub sources: Vec<usize>,
    /// Draws of one frame, so frame `f` is `f * per_frame..(f + 1) * per_frame`.
    pub per_frame: usize,
    /// Flagged batches left undrawn: under an animated node, or outside the opaque list.
    pub skipped: usize,
}

/// The pass of `track` over its loaded frames, `None` when no flagged batch is drawable.
///
/// A batch under an animated node is left out: its vertices are in the anchor's space and the
/// pass's matrix is the node's own. A batch in the cutout or blended list is left out because
/// this pass has the opaque list's pipeline only (as `oag_render::shine::build_track`). Both
/// are counted in [`Pass::skipped`].
#[must_use]
pub fn build(track: &Model) -> Option<Pass> {
    let frames = &track.caustic.frames;
    if frames.len() != FRAMES {
        return None;
    }
    let mut remap: Vec<Option<u32>> = vec![None; track.vertices.len()];
    let mut vertices: Vec<GpuVertex> = Vec::new();
    let mut indices = Vec::new();
    let mut draws = Vec::new();
    let mut sources = Vec::new();
    let mut skipped = 0;
    for flagged in &track.caustic.draws {
        let source = track
            .draws
            .iter()
            .position(|d| d.range == flagged.draw.range && d.node == flagged.draw.node);
        let Some(source) = source.filter(|_| !flagged.draw.moving) else {
            skipped += 1;
            continue;
        };
        let first = indices.len() as u32;
        let colour = colour(flagged.scale);
        for &index in
            &track.indices[flagged.draw.range.start as usize..flagged.draw.range.end as usize]
        {
            let slot = &mut remap[index as usize];
            let mapped = *slot.get_or_insert_with(|| {
                let mut vertex = track.vertices[index as usize];
                vertex.texcoord = uv(vertex.position, flagged.origin);
                vertex.colour = colour;
                // The caustic frames are not selected by the base texture's level input.
                vertex.texcoord2[1] = 0.0;
                // `plain` whitens a lit vertex; the pass's colour is the tint alone.
                vertex.lit = 0.0;
                vertices.push(vertex);
                vertices.len() as u32 - 1
            });
            indices.push(mapped);
        }
        draws.push(DrawCall {
            range: first..indices.len() as u32,
            texture: None,
            ..flagged.draw.clone()
        });
        sources.push(source);
    }
    if draws.is_empty() {
        return None;
    }
    let per_frame = draws.len();
    let sources = sources.repeat(FRAMES);
    let draws: Vec<DrawCall> = (0..FRAMES)
        .flat_map(|frame| {
            draws.iter().map(move |draw| DrawCall {
                texture: Some(frame),
                ..draw.clone()
            })
        })
        .collect();
    let mut model = Model::none(&format!("{} (caustic pass)", track.label));
    model.vertices = vertices;
    model.indices = indices;
    model.draws = draws;
    model.textures = frames.iter().cloned().map(Some).collect();
    model.centre = track.centre;
    model.radius = track.radius;
    model.per_batch_cull = track.per_batch_cull;
    plain(&mut model);
    Some(Pass {
        model,
        sources,
        per_frame,
        skipped,
    })
}

#[cfg(test)]
mod tests;
