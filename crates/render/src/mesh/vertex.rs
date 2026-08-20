//! [`GpuVertex`]: one vertex in the layout `mesh.wgsl` declares.
//!
//! Split out of `mesh.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.
//!
//! **The field order in here is the attribute order in
//! `mesh_render::build`'s `vertex_attr_array!`**, and the two have to be
//! changed together - see [`GpuVertex::lightmap_texcoord`] for what happens
//! when they drift.

/// A vertex as the mesh shader expects it.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub colour: [f32; 4],
    pub texcoord: [f32; 2],
    /// 1.0 to apply the viewer's light rig, 0.0 for geometry that is prelit.
    ///
    /// Track batches carry vertex colours *and* normals, and their colours are
    /// baked lighting. Lighting them again multiplies two lighting terms and the
    /// track comes out nearly black. Ship batches have no vertex colour, so they
    /// need the rig. This is the viewer's own choice, not the game's: the GE
    /// decides per draw from state we have not recovered.
    pub lit: f32,
    /// Which of [`Model::anim_tracks`] transforms this vertex's texture
    /// coordinate, plus one; `0` for a surface that does not animate.
    ///
    /// The shader looks the entry up and applies `uv * scale + offset`, the
    /// GE's own `TEXSCALE`/`TEXOFFSET` arithmetic - see
    /// `docs/ghidra/functions/psp-pulse-usa/texture-animation.md`. The curve
    /// itself is the material's authored keyframe block, sampled on the CPU
    /// once per frame per track rather than per vertex.
    ///
    /// An index rather than the scalar V rate this used to be, because the
    /// authored data is not a scalar V rate. `16_Track` alone carries 17
    /// distinct tracks, and they scroll in U as often as in V - the
    /// `col_display7` displays this project animated downwards actually run
    /// sideways - some diagonally, and the flicker panels step between held
    /// values rather than sliding at all. One `u32` carries all of that for
    /// the same four bytes per vertex the rate cost.
    pub anim: u32,
    /// Where a **lightmap** is sampled, for the surfaces that carry one.
    ///
    /// A second, separate coordinate set, because a lightmap is an atlas: its
    /// coordinates place a surface's own patch inside one texture for the whole
    /// circuit, and they have nothing to do with where the diffuse tiles. On
    /// Wipeout HD it is the `lightmapUV` attribute the chunk's vertex
    /// declaration names outright; every other title leaves it at zero and
    /// binds a white lightmap, so the multiply in `mesh.wgsl` is the identity.
    ///
    /// **Field order here is attribute order, and that is load-bearing.**
    /// `wgpu::vertex_attr_array!` lays offsets out in the order the locations
    /// are written, not in field order, so a field inserted *above* an existing
    /// one silently shifts every later attribute's offset - which is exactly
    /// what putting this after `texcoord` did: the diffuse coordinates stayed
    /// right and `lit` and `anim` came out of the wrong bytes, painting the
    /// circuit in flat white. New attributes go on the end, which is why
    /// [`Self::xform`] follows this rather than sitting beside its sibling
    /// [`Self::anim`].
    pub lightmap_texcoord: [f32; 2],
    /// Which of [`Model::anim_nodes`] *moves* this vertex, plus one; `0` for
    /// geometry the file places statically.
    ///
    /// The sibling of [`Self::anim`], and deliberately the same shape: that one
    /// animates the texture coordinate, this one the position. A vertex under
    /// an `Anim Transform` is baked in that node's own space and multiplied by
    /// the node's world matrix in the shader, because the matrix changes every
    /// frame and the bake happens once.
    ///
    /// Declared last rather than next to `anim` for the reason
    /// [`Self::lightmap_texcoord`] gives: this struct's order *is* the
    /// attribute array's order.
    ///
    /// See `oag_formats::vex::anim_anchors` for the split, and
    /// `docs/rendering/scenery-animation.md` for why the class cannot simply be
    /// folded into the bake.
    pub xform: u32,
    /// HD's sun-occlusion mask, `1.0` where nothing is known to occlude it.
    ///
    /// **Not `colour.a`.** That channel is already spoken for twice over - the
    /// PSP/PS2 boost plume's baked alpha falloff, and the bloom pass's glow
    /// mask in `mesh.wgsl`'s fragment output - and HD's mask means neither.
    /// `oag_formats::rcsmodel::Mesh::vertex_light`'s fourth component for a
    /// chunk that carries a colour set, `1.0` (unmasked) for one that does
    /// not and for every non-HD title, which never reads this field at all.
    /// See `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "The sun is real
    /// and it is masked".
    pub sun_mask: f32,
}
