//! [`Emissive`]: the additive glow one Wipeout HD material carries.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. What decides
//! *which* materials get one is `super::rcs::emissive`, and the shader half is
//! `mesh_render::Emissives`.

/// One material's additive glow layer: what Wipeout HD's emissive family
/// samples from unit 1, and how it moves.
///
/// The fragment programs compute `unit1(u, (v + a) * b + time) * tint` and add
/// it to the albedo, gated by the diffuse alpha - see
/// [`slots::ADD_SECOND`](slots::ADD_SECOND) and, for the microcode,
/// `docs/formats/rcsmaterial.md`. All four numbers are authored per material
/// and none can be defaulted away: over 311 records the disc carries **37
/// distinct tints** (only 108 of them white) and **34 distinct rates**.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Emissive {
    /// The float3 the sample is multiplied by, parameter `0xe8bcd7f5`. No
    /// preimage yet; `crates/render/examples/hd_param_names.rs` is the sweep.
    pub tint: [f32; 3],
    /// `a`, parameter `0x78256a45`: what is added to `v` before the scale.
    pub offset: f32,
    /// `b`, parameter `0x78787596`: what that sum is scaled by. The clock is
    /// added *after* it, so the surface scrolls one texture unit per second
    /// whatever `b` is, and `b` is what sets how much of the texture that is.
    pub scale: f32,
}
