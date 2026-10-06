//! The ghost ship: a hull drawn from a recorded pose as a translucent,
//! proximity-faded copy of itself, with static stamped into the glow mask.
//!
//! Recovered from Pulse's PSP executable - `MeshNode_Ghost_Draw`
//! (`0x08910fe0`) and its submit, `MeshNode_Ghost_Submit` (`0x08910320`). The
//! addresses, the per-pass state table and the evidence are on
//! `docs/ghidra/functions/psp-pulse-usa/ghost.md`; this module implements that
//! reading and cites it.
//!
//! # What the original draws
//!
//! The team's ordinary ship model, three times, all unlit:
//!
//! 1. **A depth lay.** Depth test and write on, no colour, and the ordinary
//!    [`oag_mesh::mesh::glow::BASE`] stamped into the glow mask. It is what makes
//!    the next two passes see only the ghost's nearest surface, so a ghost
//!    never shows its own back faces through itself.
//! 2. **The hull, cross-faded.** Depth `EQUAL` against that lay, each batch's
//!    own texture, alpha test `GREATER 0`, and a fixed-factor blend
//!    `out = tex * k + dst * (1 - k)` with the same `k` on red, green and blue
//!    and the glow mask left alone. Not additive: a translucent hull.
//! 3. **Static into the glow mask.** Depth `EQUAL`, no colour, and where
//!    `Data\Tex\staticglow.mip` projected in screen space has alpha over
//!    `0x80`, the proximity's `glow_ref` stamped into the mask. What a player
//!    sees of this is the bloom picking up a noisy mask, so a source whose
//!    bloom is not Pulse's draws nothing for it.
//!
//! `k` and `glow_ref` come from [`fade`], the distance from the player's hull
//! to the ghost.
//!
//! # What is ours
//!
//! - **Nothing is drawn inside 5 units**, where the original still lays
//!   depth and stamps the mask under an invisible hull. Chosen, not measured:
//!   a craft driving through its own ghost would otherwise have the ghost's
//!   depth cut into the exhaust and the particles drawn after it.
//! - **The ghost draws last in the scene pass**, after the exhaust, sparks
//!   and particles; the original sorts it by its own key, `0x4d000000`, whose
//!   place among the others was not read. Chosen, not measured.
//! - **The velocity target** carries the hull's own motion from the depth lay,
//!   so the surface the depth buffer describes and the one the velocity
//!   buffer describes are the same - the rule `mesh_render::Velocity` states.
//! - **Every title draws Pulse's recipe.** Pure, HD and 2048 each have a
//!   `MeshNode_Ghost` class or an equivalent, not read; on them the hull and
//!   the fade are Pulse's and the static stamp is skipped, since their glow
//!   mask is not Pulse's. Chosen, not measured.

use oag_core::Rng;

use oag_mesh::mesh::GpuVertex;

/// `MeshNode_Ghost_Draw`'s far weight, `0.55` (`0x3f0ccccd`): what `k` is from
/// 15 units out.
pub const FAR_WEIGHT: f32 = 0.55;

/// Inside this distance the ghost is fully transparent: `5.0`.
pub const NEAR: f32 = 5.0;

/// How far past [`NEAR`] the ramp runs before it jumps to the far values:
/// `10.0`.
pub const RAMP: f32 = 10.0;

/// The ramp's divisor, `20.0` - twice [`RAMP`], so the ramp only ever reaches
/// half of [`FAR_WEIGHT`] before the jump. That is the code, not a misread;
/// see `ghost.md`, "the proximity law".
pub const RAMP_DIVISOR: f32 = 20.0;

/// The static texture's tiling across the screen, `(4.0, 1.8)` - the globals
/// at `0x08abf4c4`/`0x08abf4c8`, never written.
pub const STATIC_SCALE: [f32; 2] = [4.0, 1.8];

/// The static texel alpha a stamp needs to clear: alpha test `GREATER 0x80`.
pub const STATIC_ALPHA_REF: u8 = 0x80;

/// How strongly the ghost draws at one distance from the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fade {
    /// The cross-fade weight `k`, `0.0..=0.55`.
    pub weight: f32,
    /// The value pass 3 stamps into the glow mask, `0..=255`.
    pub glow: u8,
}

/// `MeshNode_Ghost_Draw`'s proximity law, confidence 90.
///
/// `x = max(0, d - 5)`; under `x < 10` the weight is `x * 0.55 / 20` and the
/// stamp `int(x * 255 / 20)`, otherwise `0.55` and `255`. Discontinuous at
/// 15 units, as the original is.
#[must_use]
pub fn fade(distance: f32) -> Fade {
    let x = (distance - NEAR).max(0.0);
    if x < RAMP {
        Fade {
            weight: x * FAR_WEIGHT / RAMP_DIVISOR,
            glow: (x * 255.0 / RAMP_DIVISOR) as u8,
        }
    } else {
        Fade {
            weight: FAR_WEIGHT,
            glow: 255,
        }
    }
}

/// A fresh static offset pair, `0.00..=0.99` in steps of `0.01` each:
/// `MeshNode_Ghost_RandomiseStatic`'s `(rand() % 100) * 0.01`.
///
/// Off a render-side generator, as the original's is off libc `rand` rather
/// than the race's: a picture is not simulation state.
pub fn static_offsets(rng: &mut Rng) -> [f32; 2] {
    [rng.below(100) as f32 * 0.01, rng.below(100) as f32 * 0.01]
}

/// What one ghost draw needs from the hull it draws: the geometry and
/// material bind groups a ship's own drawable already holds.
///
/// Borrowed rather than rebuilt, so a ghost costs no second upload of the
/// hull. The bind groups must be built against
/// [`oag_mesh::mesh_render::material_bind_group_layout`], which every mesh drawable's
/// are.
#[derive(Debug)]
pub struct Hull<'a> {
    /// The hull's vertex buffer, [`GpuVertex`] layout.
    pub vertices: &'a wgpu::Buffer,
    /// Its `u32` index buffer.
    pub indices: &'a wgpu::Buffer,
    /// Its material bind groups; slot 0 is the white fallback.
    pub textures: &'a [wgpu::BindGroup],
    /// Every batch to draw, as an index range and the texture ordinal its
    /// material names.
    pub draws: Vec<(std::ops::Range<u32>, Option<usize>)>,
}

/// The per-frame values a ghost draws with.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    /// The camera's view-projection.
    pub view_projection: oag_core::math::Mat4,
    /// Where the ghost is drawn.
    pub model: oag_core::math::Mat4,
    /// Last tick's `view_projection * model`, for the velocity target.
    pub prev_mvp: oag_core::math::Mat4,
    /// This frame's [`fade`].
    pub fade: Fade,
    /// This frame's [`static_offsets`].
    pub offsets: [f32; 2],
}

/// The uniforms `ghost.wesl` reads.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_projection: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    prev_mvp: [[f32; 4]; 4],
    /// `offset_u`, `offset_v`, `glow_ref / 255`, base stamp `/ 255`.
    params: [f32; 4],
    /// `scale_u`, `scale_v`, alpha ref `/ 255`, unused.
    scale: [f32; 4],
}

/// The three pipelines, one uniform buffer and the static texture.
#[derive(Debug)]
pub struct Pipeline {
    depth: wgpu::RenderPipeline,
    hull: wgpu::RenderPipeline,
    stamp: Option<wgpu::RenderPipeline>,
    uniforms: wgpu::Buffer,
    uniform_bind: wgpu::BindGroup,
    static_bind: wgpu::BindGroup,
    stamps_glow: bool,
}

mod pipeline;

#[cfg(test)]
mod tests;

impl Pipeline {
    /// Writes this frame's uniforms. Call before the pass that draws.
    pub fn write(&self, queue: &wgpu::Queue, frame: &Frame) {
        let uniforms = Uniforms {
            view_projection: frame.view_projection.to_cols_array_2d(),
            model: frame.model.to_cols_array_2d(),
            prev_mvp: frame.prev_mvp.to_cols_array_2d(),
            params: [
                frame.offsets[0],
                frame.offsets[1],
                f32::from(frame.fade.glow) / 255.0,
                f32::from(oag_mesh::mesh::glow::BASE) / 255.0,
            ],
            scale: [
                STATIC_SCALE[0],
                STATIC_SCALE[1],
                f32::from(STATIC_ALPHA_REF) / 255.0,
                0.0,
            ],
        };
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Draws the three passes over `hull`, at `fade`'s weight. Nothing at all
    /// inside [`NEAR`] - see the module doc.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, hull: &Hull<'_>, fade: Fade) -> u32 {
        if fade.weight <= 0.0 || hull.textures.is_empty() {
            return 0;
        }
        let k = f64::from(fade.weight);
        pass.set_blend_constant(wgpu::Color {
            r: k,
            g: k,
            b: k,
            a: k,
        });
        pass.set_bind_group(0, &self.uniform_bind, &[]);
        pass.set_bind_group(2, &self.static_bind, &[]);
        pass.set_vertex_buffer(0, hull.vertices.slice(..));
        pass.set_index_buffer(hull.indices.slice(..), wgpu::IndexFormat::Uint32);
        let mut drawn = 0;
        let passes = [Some(&self.depth), Some(&self.hull), self.stamp.as_ref()];
        for pipeline in passes.into_iter().flatten() {
            pass.set_pipeline(pipeline);
            for (range, texture) in &hull.draws {
                let slot = texture.map_or(0, |t| t + 1).min(hull.textures.len() - 1);
                pass.set_bind_group(1, &hull.textures[slot], &[]);
                pass.draw_indexed(range.clone(), 0, 0..1);
                drawn += 1;
            }
        }
        drawn
    }

    /// Whether this pipeline stamps the glow mask - Pulse's bloom - or leaves
    /// it alone.
    #[must_use]
    pub fn stamps_glow(&self) -> bool {
        self.stamps_glow
    }
}

/// The vertex layout `ghost.wesl` reads: [`GpuVertex`], of which it uses the
/// position and the texture coordinate.
pub(crate) const STRIDE: u64 = std::mem::size_of::<GpuVertex>() as u64;

// `ghost.wesl`'s `Uniforms` is three matrices and two `vec4`s; a field added
// on one side only fails here rather than as a garbled picture.
const _: () = assert!(std::mem::size_of::<Uniforms>() == 3 * 64 + 32);
