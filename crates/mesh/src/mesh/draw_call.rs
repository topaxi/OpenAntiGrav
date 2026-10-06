//! One unit of submitted geometry, and the sphere it is culled against.
//!
//! Split out of [`super`] when that file crossed the size ratchet. These two are
//! the natural piece to move: everything else there *builds* a
//! [`Model`](super::Model), and these are what a build produces one of per
//! material run.

use oag_vex::vex;

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
    /// Whether this draw's geometry is moved per frame by an `Anim Transform`,
    /// which makes [`Self::bounds`] a statement about time zero only.
    ///
    /// Set, the draw skips the **frustum** test, because it has no bound it
    /// could trust. A moving object's world-space extent is its whole authored
    /// path, and on the widest node measured that is 5,000 units, so a bounding
    /// sphere honest enough to be safe would swallow most of the circuit and
    /// cull nothing anyway. The authored **PVS mask still applies**: it keys
    /// on the draw's node, not on its position, and the original's own frame
    /// omits the moving meshes its sections hide (`oag_render::pvs::visible`).
    ///
    /// 474 of a circuit's ~6,800 meshes carry it, so what this costs is about
    /// 7% of the geometry drawn past the frustum - against a moving object
    /// vanishing when the test disagrees with where it actually is. See
    /// `docs/rendering/scenery-animation.md`.
    pub moving: bool,
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
    /// contribution. See [`oag_vex::vex::Batch::is_culled`].
    pub culled: bool,
    /// Which blend equation this batch asked for, or `None` when it is not a
    /// transparent batch at all.
    ///
    /// **The three bits inside `is_transparent()`'s `0x0700` are not
    /// interchangeable, and this crate drew all of them with one equation
    /// until it was recovered.** `0x100` is an ordinary alpha blend, `0x200`
    /// is additive and source-alpha weighted, `0x400` is unblended. Carried
    /// per draw call rather than per model because a single mesh mixes them.
    /// See [`oag_vex::vex::Batch::blend_class`].
    pub blend: Option<vex::BlendClass>,
    /// The blend equation this batch's **own file** authors, when the file
    /// authors one rather than naming a class.
    ///
    /// Pulse's batches carry a three-way class in `pass_mask & 0x0700` and that
    /// is all they carry, so [`Self::blend`] is the whole story there and this
    /// is `None`. A Wipeout HD material carries a source and a destination
    /// factor instead - see `oag_rcs::rcsmodel::Blend::Factors` - and the
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
    /// See [`oag_vex::vex::mesh_layer`] for the derivation and
    /// `docs/rendering/draw-order.md` for the ordering model as a whole. A draw
    /// with no mesh behind it - a ribbon, a collision overlay, a PS3 chunk -
    /// carries [`oag_vex::vex::LAYER_DEFAULT`], which is uniform and so
    /// leaves the stable sort holding its list in the order it was built.
    pub layer: u32,
    /// The alpha-test reference this batch's own file authors, normalised to
    /// `0.0..=1.0`, or `None` for a draw whose file authors none.
    ///
    /// **`Some` only on `Model::alpha_tested_draws`**, and only for a `.vex`
    /// batch: `oag_vex::vex::Batch::alpha_test_reference` derives it from
    /// `pass_mask` and `header_flags`, and the corpora author three values
    /// between them - `0`, `0x10` and `0x7f`, the census in
    /// `crates/vex/tests/alpha_test_reference_ground_truth.rs`. Carried per
    /// draw call rather than per model for the reason [`Self::blend`] is: a
    /// single mesh mixes them, and a circuit is one `Model`.
    ///
    /// **The `0` is why `mesh.wesl` discards at `<=` and not `<`.** The GE's
    /// test is `GU_GREATER`, so a reference of `0` keeps every texel above
    /// fully transparent and discards the rest; under `<` it discards nothing
    /// at all, and Pure's `Speedup Pad` draws as a solid square.
    ///
    /// `None` keeps the pipeline-level reference, which is `Model::alpha_test_ref`
    /// where a Wipeout HD material authored one and `mesh.wesl`'s own
    /// `ALPHA_TEST_THRESHOLD` otherwise. So the two families compose without
    /// either overriding the other: HD authors per material and lands on the
    /// pipeline, PSP/PS2 author per batch and land here.
    pub alpha_test_ref: Option<f32>,
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
    /// Index of the `.rcsmodel` chunk this draw call came from, in file order,
    /// or `None` for anything that is not PS3 chunk geometry - or, in the 2048
    /// lineage's container (Vita 2048, Omega), the **mesh object** its submesh
    /// belongs to, which is what that lineage's `.pvs` addresses.
    ///
    /// **The join key for Wipeout HD's authored PVS.** `track.pvs` carries one
    /// bit per chunk per cell, addressed by exactly this index - see
    /// [`oag_rcs::hd_pvs`] and `docs/formats/hd-pvs.md`. It is the PS3
    /// counterpart of [`Self::node`]: the same idea that a draw call should be
    /// culled by the association the artists authored rather than by one
    /// guessed from its world-space bounds, which `crate::pvs`' module docs
    /// argue at length for the PSP.
    pub chunk: Option<u32>,
}
