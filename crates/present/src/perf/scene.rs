//! What one frame's culling did, as the overlay reads it.
//!
//! Lives here rather than in `race` so the meter can take it without this crate
//! knowing a race exists; the renderer's callers fill it in.

/// What one frame's frustum culling did, for the performance overlay.
///
/// Plain counts rather than anything richer: the point is to show the effect
/// of `--lod` and frustum culling is having, not to profile the
/// renderer.
#[derive(Debug, Clone, Copy, Default)]
pub struct SceneStats {
    pub draws_submitted: u32,
    pub draws_culled: u32,
    pub triangles: u32,
    /// Whether the motion-blur chain was actually encoded this frame.
    ///
    /// **Not a statistic, and it is here because it has to travel the same
    /// path.** A caller that claimed a `PassTimer` slot for the blur needs to
    /// know whether the chain wrote the timestamps into it: at `off`, and on a
    /// frame whose scratch targets have not been built yet, `MotionBlur::render`
    /// encodes nothing, and a claimed-but-unwritten slot never comes back. Four
    /// of those end measurement for the run - see `PassTimer::begin`. The
    /// caller answers that with `PassTimer::abandon`, and this is what tells it
    /// to.
    pub blur_encoded: bool,
    /// Whether the HD/Fury bloom chain was actually encoded this frame.
    ///
    /// The same shape as [`Self::blur_encoded`] and for the same reason, with
    /// one difference worth stating: `hd_bloom::Chain::run` has no early
    /// return, so a caller that claims a slot only when a scene actually holds
    /// a `Chain` (see `race::scene::frame::Scene::hd`) never needs the
    /// abandon path in the running case - this field earns its keep on the
    /// PSP/Pure titles and any HD circuit with no `HDR and Bloom` block, where
    /// no `Chain` exists to claim a slot for at all.
    pub hd_bloom_encoded: bool,
}

impl SceneStats {
    pub fn add(&mut self, other: Self) {
        self.draws_submitted += other.draws_submitted;
        self.draws_culled += other.draws_culled;
        self.triangles += other.triangles;
        // Or-ed rather than summed: they are the two fields here that are not
        // counts, and "any part of this frame encoded the chain" is the
        // question a timestamp claim is asking.
        self.blur_encoded |= other.blur_encoded;
        self.hd_bloom_encoded |= other.hd_bloom_encoded;
    }
}
