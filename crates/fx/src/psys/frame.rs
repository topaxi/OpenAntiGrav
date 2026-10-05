//! The frame scale: what a matrix with rows of length `s` does to an instance's spawns.
//! Its own file because `psys.rs` is baselined.

use super::{Playing, Stage, System};

impl System {
    /// Sets the uniform scale of the matrix this instance was spawned with.
    ///
    /// `ParticleSystem_EmitParticle` carries a root emitter's spawn offset and its
    /// velocity through the instance's node matrix (`instance + 0xf0`) unless the
    /// emitter's flag `0x2` is set, so a matrix with rows of length `s` scales both
    /// by `s` - and **neither the drawn size, the lifetime, the gravity nor a
    /// child instance's own spawn**, which takes a matrix of unit rows. Read live
    /// on the ship explosion, whose matrix rows are `0.75`: every pool's RMS radius
    /// and rise is `0.75` of what the unscaled law gives while its sizes are exact
    /// (`particle-system.md`, 2026-10-01). `1.0` unless a caller sets it.
    pub fn set_frame_scale(&mut self, scale: f32) {
        self.frame_scale = scale;
    }
}

impl Stage {
    /// Sets the uniform scale of the matrix an instance was spawned with - see
    /// [`System::set_frame_scale`]. A no-op on a stale handle.
    pub fn set_frame_scale(&mut self, playing: Playing, scale: f32) {
        if let Some(instance) = self.get_mut(playing) {
            instance.system.set_frame_scale(scale);
        }
    }
}
