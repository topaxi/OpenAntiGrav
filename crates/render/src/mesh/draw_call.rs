//! One unit of submitted geometry, and the sphere it is culled against.
//!
//! Split out of [`super`] when that file crossed the size ratchet. These two are
//! the natural piece to move: everything else there *builds* a
//! [`Model`](super::Model), and these are what a build produces one of per
//! material run.

use oag_formats::vex;

/// A world-space bounding sphere, for frustum culling.
///
/// A sphere rather than an oriented box: cheap to test
/// ([`oag_core::math::frustum::Frustum::intersects_sphere`]), and a batch's
/// own vertices already give a centre and radius for free while they are
/// being iterated to build [`GpuVertex`]s, with no separate pass needed.
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub centre: [f32; 3],
    pub radius: f32,
}

/// A run of indices sharing one texture.
#[derive(Debug, Clone)]
pub struct DrawCall {
    pub range: std::ops::Range<u32>,
    /// Index into [`Model::textures`], or `None` for untextured.
    pub texture: Option<usize>,
    /// World-space bounds of this draw call's own vertices, for frustum
    /// culling - not the whole model's, which would defeat the point on a
    /// track where one `Model` is the entire circuit.
    pub bounds: Bounds,
    /// Whether this batch is drawn single-sided, from its own `pass_mask`.
    ///
    /// **The original culls most of its geometry and this crate culled none of
    /// it.** Measured on the PSP disc: `01_Track` has 1,632 of 1,734 batches
    /// culled, `16_Track` 1,613 of 2,079, and `Assegai\Ship.vex` 12 of 14 -
    /// but `shipboost.vex` culls none of its four, so it is genuinely per
    /// batch and not a global setting. It matters most on **transparent**
    /// batches, where a back face is not hidden by the depth test but blended
    /// a second time: 119 of `01_Track`'s 142 transparent batches are
    /// single-sided in the original, so drawing them two-sided doubles their
    /// contribution. See [`oag_formats::vex::Batch::is_culled`].
    pub culled: bool,
    /// Which blend equation this batch asked for, or `None` when it is not a
    /// transparent batch at all.
    ///
    /// **The three bits inside `is_transparent()`'s `0x0700` are not
    /// interchangeable, and this crate drew all of them with one equation
    /// until it was recovered.** `0x100` is an ordinary alpha blend, `0x200`
    /// is additive and source-alpha weighted, `0x400` is unblended. Carried
    /// per draw call rather than per model because a single mesh mixes them.
    /// See [`oag_formats::vex::Batch::blend_class`].
    pub blend: Option<vex::BlendClass>,
    /// The blend equation this batch's **own file** authors, when the file
    /// authors one rather than naming a class.
    ///
    /// Pulse's batches carry a three-way class in `pass_mask & 0x0700` and that
    /// is all they carry, so [`Self::blend`] is the whole story there and this
    /// is `None`. A Wipeout HD material carries a source and a destination
    /// factor instead - see `oag_formats::rcsmodel::Blend::Factors` - and the
    /// two families do not nest: `0001`/`0001` is unweighted additive and
    /// `0001`/`0303` is premultiplied alpha, and neither is any member of
    /// `vex::BlendClass`. Folding them onto the nearest member is what this
    /// field exists to stop, so when it is `Some` it **overrides**
    /// [`Self::blend`] as the equation, while `blend` still says the draw is
    /// transparent at all.
    pub blend_state: Option<wgpu::BlendState>,
    /// The **render layer** this draw's mesh is in - the top twelve bits of the
    /// sort key the original submits it to its one render queue with.
    ///
    /// **Lower draws first**, and [`Model::sort_by_layer`] is what acts on it.
    /// See [`oag_formats::vex::mesh_layer`] for the derivation and
    /// `docs/rendering/draw-order.md` for the ordering model as a whole. A draw
    /// with no mesh behind it - a ribbon, a collision overlay, a PS3 chunk -
    /// carries [`oag_formats::vex::LAYER_DEFAULT`], which is uniform and so
    /// leaves the stable sort holding its list in the order it was built.
    pub layer: u32,
    /// Index of the scene-tree node this draw call came from, into the
    /// `vex::nodes` of the file the model was built from, or `None` for
    /// synthetic geometry (ribbons, collision overlays, fixtures).
    ///
    /// This is what lets `oag_render::pvs` place a draw call in its *authored*
    /// section - the section node sharing its ancestor group - instead of
    /// guessing from world-space bounds. Only meaningful while the model maps
    /// to one file; [`merge`] keeps the per-source values, which are ambiguous
    /// across sources.
    pub node: Option<u32>,
}
