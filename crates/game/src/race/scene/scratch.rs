//! [`Scratch`]: the per-frame vertex lists a race frame is assembled into.
//!
//! Split out of `race/scene.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// The per-frame vertex lists [`Scene`] hands to its two effect pipelines.
///
/// One owner rather than four locals, so the buffers survive the frame that
/// filled them - see [`Scene::scratch`].
#[derive(Debug, Default)]
pub(crate) struct Scratch {
    /// Engine flares, HD sprite quads and projectile billboards: the
    /// `exhaust::Pipeline`'s sprite buffer.
    pub(crate) vertices: Vec<oag_render::mesh::GpuVertex>,
    /// The engine ribbons, the same pipeline's second buffer.
    pub(crate) trail: Vec<oag_render::mesh::GpuVertex>,
    /// The particle pipeline's two blend classes.
    pub(crate) additive: Vec<oag_render::mesh::GpuVertex>,
    pub(crate) alpha: Vec<oag_render::mesh::GpuVertex>,
    /// Which weapon pads currently hand out a pickup, one per pad node.
    pub(crate) pads_ready: Vec<bool>,
    /// The span of vertices whichever recolour or reshape is running has just
    /// built, on its way to a `write_buffer`.
    ///
    /// **One buffer for all five of them** - the airbrake flaps, the shield
    /// shell, the cockpit sphere, each weapon pad and each speed pad -
    /// because [`Scene::render`] runs them one after another and none of
    /// them reads what the last one wrote. Five buffers would be five
    /// high-water marks kept alive to save nothing.
    ///
    /// Each of the five cleared and refilled it every frame before this
    /// existed: 85 KiB a frame for the pads alone on Talon's Junction, plus a
    /// pair of flap writes on every frame of every race and a whole shell's
    /// vertices for every craft with its shield up.
    ///
    /// Each caller clears it itself rather than trusting the state it is
    /// handed, so a span is never written from another mesh's leftovers.
    pub(crate) recoloured: Vec<oag_render::mesh::GpuVertex>,
}

impl Scratch {
    /// Empties every list while keeping what they have already grown to.
    pub(crate) fn clear(&mut self) {
        self.vertices.clear();
        self.trail.clear();
        self.additive.clear();
        self.alpha.clear();
        self.pads_ready.clear();
        self.recoloured.clear();
    }
}
