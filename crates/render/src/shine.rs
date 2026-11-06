//! The hull's **extra pass**: the batches a `.vex` marks `pass_mask & 0x2000`
//! drawn a second time under environment mapping with their material's second
//! texture - on a Pulse hull, `Data\Tex\envtest4bit.tga`, the grey ramp that
//! gives the yellow spine and the inner wings their gloss.
//!
//! Recovered from `FUN_0890db54` (the pass's batch loop) and
//! `Mesh_SetBatchDrawState` (`0x0890d994`), bracketed by
//! `Mesh_BeginTransparentPass` (`0x0890d904`), and read live on a race on
//! Talon's Junction: see `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`,
//! "The hull's extra pass", for the evidence. What the pass is:
//!
//! - **Which batches**: a batch with `pass_mask & 0x2000` inside a mesh whose
//!   own flag word has it, drawn with the texture at material `+0x08`
//!   ([`oag_vex::vex::Material::second_texture`]) instead of the one at `+0x04`.
//!   On Assegai that is `shipShape`'s three blended batches, both airbrakes'
//!   small one and `canopyShape`'s glass - six of the hull's fourteen.
//! - **The coordinates** are generated, not authored: `TEXMAPMODE` 2 with
//!   `LS0 = light 0`, `LS1 = light 1`, and the hull model's two light lists
//!   (`model+0x48`) hold the fixed world-space pair [`crate::texgen::ENV_BASIS_0`]
//!   and [`crate::texgen::ENV_BASIS_1`] - `model+0x1a8` was read as `1` on the
//!   live hull and its `+0x1ac..+0x1c0` held exactly those six floats. So the
//!   coordinates follow the ship's rotation and never the camera.
//! - **Lighting is off** and the texture is drawn as it is, white vertex colour.
//! - **Blending** is `Mesh_SetBatchDrawState`'s, chosen by header byte 3 bit
//!   `0x10`, which the *loader* sets: `Mesh_CountBatchesPerList` (`0x0890e7a8`)
//!   ORs it into every list-A batch with `0x2000` and without `0x0800`. The
//!   five hull-body batches therefore *replace* what is under them and write
//!   depth; the canopy (list B) is additive and writes none. The batch's own
//!   ordinary pass then follows, additively and at equal depth, so the pixel is
//!   `env + lit * texture` either way.
//!
//! # Measured against the original, 2026-10-01
//!
//! A recorded GE list of a live Metropia-grid hull (`scripts/psp-ge-dump.py`):
//!
//! - **Replace-then-add equals one additive redraw.** The five replace batches
//!   run `BLENDMODE` `0xaa` with `FIXA` white and `FIXB` black (`src` alone),
//!   depth writes on; the ordinary pass over the same batch runs the same
//!   blend with `FIXB` white (`src + dst`), depth writes off, depth function
//!   `EQUAL`. The pixel is `env + lit * texture` clamped once, which is what
//!   our opaque hull plus an additive redraw gives. The canopy is additive in
//!   both with the depth **test on** (function 6, as the ordinary passes), not
//!   off as an earlier page read it.
//! - **Fog is on, and its colour register is `0`** where the ordinary pass
//!   carries the circuit's: the pass fades to nothing with distance, not into
//!   the haze. The scene uniform is written with a black fog colour.
//!
//! # What is ours
//!
//! Chosen, not measured:
//!
//! - The redraw's depth function is `LessEqual` where the original's is the
//!   strict one; they differ only at equal depth, which the pass never writes.
//!
//! Measured (2026-10-01, three recorded GE lists, craft stowed): each flap's
//! `TEXMAPMODE` 2 PRIM carries the same world matrix as its ordinary twin and
//! not `shipShape`'s, so the pass rides the flap's own node and follows its
//! deflection: [`write`] swings the flap's vertices through the same
//! [`oag_mesh::mesh::Flap::swung`] the hull's base draw uses. The *deflected*
//! state itself was not recorded (stowed only); that the original's pass
//! follows is the structure of `Mesh_CompileExtraPass`, which replays inside
//! the mesh's own node, not a read of a deflected frame.
//!
//! # A circuit's own pass
//!
//! A circuit's `*_shinemap` batches carry the same bit and a chrome map as
//! their second texture (`07_chromemap_02.tga`, `07_env.tga`), and the same
//! mode-2 state, but the lights are not the fixed pair: in the same GE list all
//! 88 mode-2 circuit draws of a frame carry lights 0 and 1 equal to **rows 0 and
//! 1 of the view matrix** (confirmed at a second, yawed camera where rows and
//! columns differ), the matcap `Vex_UpdateLightLists_q` writes for a model whose
//! `+0x1a8` is 0. [`build_track`] and [`write_view`] draw it, animated nodes
//! included: a vertex normal reaches here already carried through its node's
//! rotation (`oag_mesh::mesh`), and an animated node's matrix turns it again per
//! frame. The 3 of those 88 draws whose world matrix is more than a scale are the
//! rotated nodes, and they are why that step exists.

use std::ops::Range;

use oag_core::math::{Mat4, Vec3};

use crate::texgen;
use oag_mesh::mesh::shine_pass::{self, plain};
use oag_mesh::mesh::{DrawCall, GpuVertex, Model};

/// Whether a race draws the pass at all: **on**, because the original draws it
/// in play (six batches of a live Assegai hull were read in the recorded GE
/// list, each under `TEXMAPMODE` 2).
pub const DRAWN: bool = true;

/// The redraw's blend: `src + dst` on colour, the destination's alpha kept.
///
/// The original's factors are `GU_FIX` white on both sides for the additive
/// canopy; for the replace batches the GE's `src` alone lands on an empty
/// framebuffer and the ordinary pass then adds - see the module docs. The
/// alpha channel is the bloom's glow mask in this renderer and the original
/// masks it on this pass (`Gu_PixelMask` `0xff000000`, read live), so it is
/// left as it was.
pub const BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// A circuit's extra pass: the `0x2000` batches of its static meshes, on a
/// vertex buffer of their own, and where each came from.
#[derive(Debug)]
pub struct TrackPass {
    /// Draws the shine batches only, under their second texture, unlit and
    /// white. Its vertices are the batches' own, re-indexed.
    pub model: Model,
    /// For each of `model.draws`, the index of the circuit draw it redraws in
    /// the circuit's own [`Model::draws`] - the key its section mask, its
    /// level-of-detail child and its frustum bound are looked up by.
    pub sources: Vec<usize>,
    /// Shine batches left undrawn because they sit outside the opaque list,
    /// whose pipeline this pass does not have.
    pub skipped: usize,
}

/// The extra pass of a circuit, `None` when it authors no drawable batch.
///
/// **Opaque batches only.** A batch in the cutout or blended list would need
/// that list's pipeline; it is counted in [`TrackPass::skipped`] so the loader
/// report can say what was left out, rather than drawing a guess. An animated
/// node's batch is kept with its `xform` and the model's [`Model::anim_nodes`],
/// so the vertex shader places it as it places the circuit's own, and
/// [`write_view`] turns its normal by the same node matrix.
#[must_use]
pub fn build_track(track: &Model) -> Option<TrackPass> {
    let mut remap: Vec<Option<u32>> = vec![None; track.vertices.len()];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut draws = Vec::new();
    let mut sources = Vec::new();
    let mut skipped = 0;
    for shine in &track.shine_draws {
        let source = track
            .draws
            .iter()
            .position(|d| d.range == shine.range && d.node == shine.node);
        let Some(source) = source else {
            skipped += 1;
            continue;
        };
        let first = indices.len() as u32;
        for &index in &track.indices[shine.range.start as usize..shine.range.end as usize] {
            let slot = &mut remap[index as usize];
            let mapped = *slot.get_or_insert_with(|| {
                vertices.push(track.vertices[index as usize]);
                vertices.len() as u32 - 1
            });
            indices.push(mapped);
        }
        draws.push(DrawCall {
            range: first..indices.len() as u32,
            ..shine.clone()
        });
        sources.push(source);
    }
    if draws.is_empty() {
        return None;
    }
    let mut model = Model::none(&format!("{} (shine pass)", track.label));
    model.vertices = vertices;
    model.indices = indices;
    model.draws = draws;
    // Only the textures a kept draw names stay on the GPU: the circuit's
    // other hundred are not this pass's to upload twice.
    let used: std::collections::HashSet<usize> =
        model.draws.iter().filter_map(|d| d.texture).collect();
    model.textures = track
        .textures
        .iter()
        .enumerate()
        .map(|(i, t)| used.contains(&i).then(|| t.clone()).flatten())
        .collect();
    model.anim_nodes = track.anim_nodes.clone();
    model.centre = track.centre;
    model.radius = track.radius;
    // The circuit's weighted batches (`Batch::weights_extra_pass_by_alpha`) paired
    // their environment slot with the batch's own texture in `lightmaps`; the hull's
    // extra pass is built the same way (`oag_mesh::mesh::shine_pass::build`). A track
    // model whose `lightmaps` the loader emptied draws the pass unweighted.
    let lightmaps = (0..model.textures.len())
        .map(|i| {
            used.contains(&i)
                .then(|| track.lightmaps.get(i).cloned().flatten())
                .flatten()
        })
        .collect();
    plain(&mut model);
    shine_pass::mask(&mut model, lightmaps);
    Some(TrackPass {
        model,
        sources,
        skipped,
    })
}

/// The texture coordinates of a [`build_track`] model, one per vertex, generated
/// for the camera `view` (world to view): the equation over the view matrix's
/// first two **rows**, which are the camera's right and up in world space, so
/// a normal facing right of the camera reaches the far end of `u`.
///
/// Read off a recorded GE list at two camera yaws: the circuit's `TEXMAPMODE`
/// 2 batches carry lights 0 and 1 equal to rows 0 and 1 of the view matrix's
/// rotation, not its columns, and a hull in the same frame is placed by the
/// same matrix at view-space `(0, -1.8, -11.5)`, centred and ahead.
///
/// One camera, one call. A frame that draws the circuit from several cameras keeps a
/// [`TrackNormals`] and calls [`TrackNormals::write_view`], which does the normals once.
pub fn write_view(model: &Model, out: &mut Vec<[f32; 2]>, view: Mat4, seconds: f32) {
    TrackNormals::default().write_view(model, out, view, seconds);
}

/// The unit world normals of a [`build_track`] model, kept between the cameras of a frame.
///
/// What [`write_view`] spends most of its time on is not the camera: each vertex's normal is
/// turned by its animated node and renormalised, and none of that reads the view. This is
/// that half, computed for a `seconds` once and reused by every camera drawn at it - a
/// split screen's second view and every view after - and for good when the circuit has no
/// animated node (the common case), where nothing it holds ever changes. An animated
/// vertex is turned the first time a view draws it at that clock value, so one no view draws
/// costs nothing. The result is the bits [`write_view`] always produced.
#[derive(Debug, Default)]
pub struct TrackNormals {
    units: texgen::UnitNormals,
    /// The `seconds` (as bits) `units` is current for; `None` before the first call.
    at: Option<u32>,
    /// Whether the model has no animated node, so `units` holds for any `seconds`.
    fixed: bool,
    /// How many times the clock moved `units` on.
    refreshed: u32,
    /// The vertices that ride a node, ascending, so a new clock value turns those and nothing
    /// else.
    animated: Vec<u32>,
    /// The nodes at `at`.
    nodes: Vec<[f32; 16]>,
    /// For each of `animated`, the `refreshed` it was last turned at.
    turned: Vec<u32>,
}

/// `v`'s unit normal in the circuit's space, under `nodes`.
///
/// An animated node's vertices are in the node's space; its matrix takes the normal to the
/// circuit's. Not an inverse transpose, as the vertex shader's own normal is not: the animated
/// meshes under a non-uniformly scaled node are the ones this skews.
///
/// Placed and mapped in one pass rather than copied into a placed list first. The arithmetic is
/// the two-pass form's to the bit: a placed normal is renormalised, then goes through the
/// identity the map's model matrix was, and is renormalised again.
fn turned_normal(v: &GpuVertex, nodes: &[[f32; 16]]) -> Vec3 {
    let identity = oag_core::math::Mat3::IDENTITY;
    let normal = match (v.xform as usize).checked_sub(1).and_then(|i| nodes.get(i)) {
        Some(m) => Vec3::new(
            m[0] * v.normal[0] + m[4] * v.normal[1] + m[8] * v.normal[2],
            m[1] * v.normal[0] + m[5] * v.normal[1] + m[9] * v.normal[2],
            m[2] * v.normal[0] + m[6] * v.normal[1] + m[10] * v.normal[2],
        )
        .normalize_or_zero(),
        None => Vec3::from_array(v.normal),
    };
    (identity * normal).normalize_or_zero()
}

impl TrackNormals {
    /// How many times the normals have been computed, as opposed to reused.
    #[must_use]
    pub fn refreshed(&self) -> u32 {
        self.refreshed
    }

    /// Brings the normals over `spans` up to `seconds`.
    fn refresh(&mut self, model: &Model, seconds: f32, spans: &[Range<usize>]) {
        let whole = self.units.len() == model.vertices.len();
        if self.at.is_none() || !whole || !(self.fixed || self.at == Some(seconds.to_bits())) {
            self.refreshed += 1;
            self.at = Some(seconds.to_bits());
            let count = self.nodes.len();
            self.nodes = model.sample_anim_nodes(seconds);
            self.fixed = self.nodes.is_empty();
            if self.fixed {
                self.animated.clear();
                // The identity is the model matrix the map takes for a circuit, kept as
                // a multiply so the coordinates are the interleaved form's to the bit.
                let identity = oag_core::math::Mat3::IDENTITY;
                texgen::unit_normals_by(&model.vertices, &mut self.units, |v| {
                    identity * Vec3::from_array(v.normal)
                });
                return;
            }
            // `animated` is read off the model once, with the whole list. A vertex that rides
            // no node has the same normal at every clock value.
            if !whole || count != self.nodes.len() {
                let nodes = &self.nodes;
                self.units
                    .refill(model.vertices.iter().map(|v| turned_normal(v, nodes)));
                self.animated.clear();
                self.animated.extend(
                    model
                        .vertices
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| {
                            (v.xform as usize)
                                .checked_sub(1)
                                .is_some_and(|i| i < nodes.len())
                        })
                        .map(|(i, _)| i as u32),
                );
                self.turned.clear();
                self.turned.resize(self.animated.len(), self.refreshed);
                return;
            }
        }
        if self.fixed {
            return;
        }
        for span in spans {
            let first = self
                .animated
                .partition_point(|&i| (i as usize) < span.start);
            let count = self.animated[first..].partition_point(|&i| (i as usize) < span.end);
            for slot in first..first + count {
                if self.turned[slot] != self.refreshed {
                    self.turned[slot] = self.refreshed;
                    let i = self.animated[slot] as usize;
                    self.units
                        .set(i, turned_normal(&model.vertices[i], &self.nodes));
                }
            }
        }
    }

    /// [`write_view`] for `model` at `seconds`, reusing the normals if they are current.
    pub fn write_view(&mut self, model: &Model, out: &mut Vec<[f32; 2]>, view: Mat4, seconds: f32) {
        let all = 0..model.vertices.len();
        self.write_view_spans(model, out, view, seconds, std::slice::from_ref(&all));
    }

    /// [`Self::write_view`] over `spans` of the vertices alone, for a view that draws only
    /// those ([`draw_vertices`]): what `out` holds elsewhere is left as it was. Most of a
    /// circuit's pass is out of any one view (`docs/rendering/frame-thread-cpu.md`, "The shine
    /// passes").
    pub fn write_view_spans(
        &mut self,
        model: &Model,
        out: &mut Vec<[f32; 2]>,
        view: Mat4,
        seconds: f32,
        spans: &[Range<usize>],
    ) {
        let right = Vec3::new(view.x_axis.x, view.y_axis.x, view.z_axis.x);
        let up = Vec3::new(view.x_axis.y, view.y_axis.y, view.z_axis.y);
        self.refresh(model, seconds, spans);
        out.resize(model.vertices.len(), [0.0; 2]);
        texgen::environment_texcoords_from_units(&self.units, spans, out, right, up);
    }
}

/// For each of `model`'s draws, the span of vertices it indexes: [`build_track`] numbers a
/// draw's vertices together, so the span is about the draw's own, longer only where it shares
/// a vertex with an earlier draw.
#[must_use]
pub fn draw_vertices(model: &Model) -> Vec<Range<usize>> {
    model
        .draws
        .iter()
        .map(|draw| {
            let indices = &model.indices[draw.range.start as usize..draw.range.end as usize];
            let first = indices.iter().copied().min().unwrap_or(0) as usize;
            let last = indices.iter().copied().max().map_or(0, |i| i as usize + 1);
            first..last
        })
        .collect()
}

/// Sorts `spans` and joins the ones that touch, so no vertex is computed twice.
pub fn merge_spans(spans: &mut Vec<Range<usize>>) {
    spans.sort_unstable_by_key(|span| span.start);
    let mut kept = 0;
    for next in 0..spans.len() {
        let span = spans[next].clone();
        if kept > 0 && span.start <= spans[kept - 1].end {
            spans[kept - 1].end = spans[kept - 1].end.max(span.end);
        } else {
            spans[kept] = span;
            kept += 1;
        }
    }
    spans.truncate(kept);
}

/// The vertices of `model` (a [`build`] result) with their texture coordinates
/// generated for a ship posed by `ship`, into `out`: the uvgen-2 equation over
/// the two fixed basis vectors, the ship's rotation and nothing of the camera.
///
/// **`flaps` are the two airbrake angles, left then right, radians** - the pass
/// rides its flap's node as the original's does, so each flap's vertices are
/// swung by [`oag_mesh::mesh::Flap::swung`], the same call the hull's base draw
/// makes, *before* the coordinates are generated: the swung normal is what the
/// glint reads. `[0.0, 0.0]` leaves every vertex where the file put it.
pub fn write(model: &Model, out: &mut Vec<GpuVertex>, ship: Mat4, flaps: [f32; 2]) {
    texgen::environment_map(
        &model.vertices,
        out,
        ship,
        texgen::ENV_BASIS_0,
        texgen::ENV_BASIS_1,
    );
    let mut moved = Vec::new();
    let mut mapped = Vec::new();
    for (flap, angle) in model.airbrakes.iter().zip(flaps) {
        let Some(flap) = flap else { continue };
        if angle == 0.0 {
            continue;
        }
        let Some(span) = flap.swung(&model.vertices, angle, &mut moved) else {
            continue;
        };
        texgen::environment_map(
            &moved,
            &mut mapped,
            ship,
            texgen::ENV_BASIS_0,
            texgen::ENV_BASIS_1,
        );
        out[span].copy_from_slice(&mapped);
    }
}

#[cfg(test)]
mod tests;
