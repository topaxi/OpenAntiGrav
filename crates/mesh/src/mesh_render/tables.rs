//! The three per-model uniform tables `mesh.wesl` reads from bind group 3.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. They share
//! a shape deliberately - slot 0 is the identity or the absence, so the
//! overwhelming majority of geometry costs one indexed load and no branch -
//! and they differ in one way worth knowing: [`Emissives`] is written once at
//! build, where the other two sample a curve on the CPU every frame.

use crate::mesh::Model;

/// The texture-transform table `mesh.wesl` reads from bind group 3: one
/// `(scale, offset)` pair per entry of [`Model::anim_tracks`], already sampled
/// for this frame.
///
/// Slot 0 is the identity, which is what [`GpuVertex::anim`] `== 0` selects, so
/// the shader needs no branch for the overwhelming majority of vertices.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TexAnims {
    /// `[scale_u, scale_v, offset_u, offset_v]` per track.
    pub transform: [[f32; 4]; crate::mesh::ANIM_TRACK_LIMIT],
}

impl Default for TexAnims {
    fn default() -> Self {
        Self {
            transform: [[1.0, 1.0, 0.0, 0.0]; crate::mesh::ANIM_TRACK_LIMIT],
        }
    }
}

impl TexAnims {
    /// Samples every track of `model` at `seconds` and packs the table.
    ///
    /// `seconds` is the model's animation clock. The original gives each model
    /// its own - the boost plume's is its flare's life timer, reset at every
    /// reveal - but for **world meshes** it passes the race clock, which is
    /// what a track's scenery gets here. See
    /// `docs/ghidra/functions/psp-pulse-usa/texture-animation.md`, "The values
    /// gap is closed"; one mesh in that page's one-frame census was seen on a
    /// different clock, and which models get their own is not recovered.
    ///
    /// Every track wraps on its own authored period, so there is no shared
    /// phase to keep them in step and nothing to seam at a global wrap.
    ///
    /// Clamped to the table's size here as well as in the builder, because
    /// [`Model`] is a plain struct anyone can fill in and [`crate::mesh::merge`]
    /// concatenates track lists without re-checking the ceiling. A model past
    /// it animates its first [`crate::mesh::ANIM_TRACK_LIMIT`] `- 1` tracks and
    /// leaves the rest at identity, which is what the builder does too.
    #[must_use]
    pub fn sample(model: &Model, seconds: f32) -> Self {
        let mut out = Self::default();
        for (slot, track) in model
            .anim_tracks
            .iter()
            .take(crate::mesh::ANIM_TRACK_LIMIT - 1)
            .enumerate()
        {
            let (scale, offset) = track.sample(seconds);
            out.transform[slot + 1] = [scale[0], scale[1], offset[0], offset[1]];
        }
        out
    }
}

/// Size, in bytes, of the [`TexAnims`] uniform buffer.
pub const TEX_ANIMS_SIZE: u64 = std::mem::size_of::<TexAnims>() as u64;

/// The node-transform table `mesh.wesl` reads from bind group 4: one world
/// matrix per entry of [`Model::anim_nodes`], sampled for this frame.
///
/// Slot 0 is the identity, which is what [`GpuVertex::xform`] `== 0` selects,
/// so the 93% of a circuit's geometry that does not move costs one indexed load
/// and no branch - deliberately the same shape as [`TexAnims`].
///
/// 8 KiB at [`crate::mesh::NODE_ANIM_LIMIT`], which is inside the 64 KiB
/// uniform binding every wgpu backend guarantees.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct NodeAnims {
    /// One column-major world matrix per node. The layout
    /// `oag_vex::vex` already uses - see `mesh.wesl`'s own note.
    pub transform: [[f32; 16]; crate::mesh::NODE_ANIM_LIMIT],
}

impl Default for NodeAnims {
    fn default() -> Self {
        Self {
            transform: [oag_vex::vex::IDENTITY; crate::mesh::NODE_ANIM_LIMIT],
        }
    }
}

impl NodeAnims {
    /// Samples every `Anim Transform` of `model` at `seconds` and packs the
    /// table.
    ///
    /// `seconds` is the model's animation clock, the same one
    /// [`TexAnims::sample`] takes and for the same reason: the original passes
    /// the race clock to every world mesh's updater, and each node wraps on its
    /// own authored `LoopEnd`.
    #[must_use]
    pub fn sample(model: &Model, seconds: f32) -> Self {
        let mut out = Self::default();
        for (slot, matrix) in model.sample_anim_nodes(seconds).into_iter().enumerate() {
            out.transform[slot + 1] = matrix;
        }
        out
    }
}

/// Size, in bytes, of the [`NodeAnims`] uniform buffer.
pub const NODE_ANIMS_SIZE: u64 = std::mem::size_of::<NodeAnims>() as u64;

/// The additive glow table `mesh.wesl` reads from bind group 3: one entry per
/// [`Model::emissive`], indexed by [`crate::mesh::slots::material_index`].
///
/// **Written once at build, not per frame.** Every number in it is an authored
/// constant - the tint the sample is multiplied by, and the two floats that
/// remap the coordinate before `scene.time` is added. The clock is the only
/// moving part and the shader already has it, so there is nothing to re-upload.
/// That is the one way this differs from [`TexAnims`] and [`NodeAnims`], which
/// both sample a curve on the CPU each frame.
///
/// Slot 0 is "no glow", which is what a `material_index` of 0 selects, so the
/// vast majority of geometry costs one indexed load and no branch - the same
/// shape as the two tables above.
///
/// 2 KiB at [`crate::mesh::rcs::EMISSIVE_LIMIT`], inside the 64 KiB uniform
/// binding every wgpu backend guarantees.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Emissives {
    /// `rgb` the tint, `w` the coordinate offset `a`.
    pub tint_offset: [[f32; 4]; crate::mesh::rcs::EMISSIVE_LIMIT],
    /// `x` the coordinate scale `b`, `y` whether the clock moves this layer;
    /// the rest is padding a uniform array's 16-byte stride forces either way.
    pub scale: [[f32; 4]; crate::mesh::rcs::EMISSIVE_LIMIT],
}

impl Default for Emissives {
    fn default() -> Self {
        Self {
            tint_offset: [[0.0; 4]; crate::mesh::rcs::EMISSIVE_LIMIT],
            scale: [[0.0; 4]; crate::mesh::rcs::EMISSIVE_LIMIT],
        }
    }
}

impl Emissives {
    /// Packs a model's glow table. Slot 0 stays zero, which adds nothing.
    #[must_use]
    pub fn of(model: &Model) -> Self {
        let mut out = Self::default();
        for (slot, layer) in model
            .emissive
            .iter()
            .take(crate::mesh::rcs::EMISSIVE_LIMIT - 1)
            .enumerate()
        {
            out.tint_offset[slot + 1] = [layer.tint[0], layer.tint[1], layer.tint[2], layer.offset];
            out.scale[slot + 1] = [layer.scale, layer.rate, 0.0, 0.0];
        }
        out
    }
}

/// Size, in bytes, of the [`Emissives`] uniform buffer.
pub const EMISSIVES_SIZE: u64 = std::mem::size_of::<Emissives>() as u64;
