//! The particle geometry a frame gathers: the collision sparks' and the stage's.
//!
//! Split out of `effects.rs` under the 1,000-line rule; a move.

use super::*;

impl Race {
    /// The collision sparks' geometry this frame, split by blend class.
    ///
    /// The pool and the effect it plays are handed out together because
    /// neither means anything alone - a particle carries an index into the
    /// effect's emitters rather than a copy of their parameters. Empty when
    /// the disc's own effect did not load.
    pub fn extend_spark_vertices(
        &self,
        additive: &mut Vec<oag_mesh::mesh::GpuVertex>,
        alpha_over: &mut Vec<oag_mesh::mesh::GpuVertex>,
        right: Vec3,
        up: Vec3,
    ) {
        let Some(effect) = self.view.handles.get(Trigger::CollisionSpark) else {
            return;
        };
        self.view
            .sparks
            .extend_vertices(additive, alpha_over, effect, right, up, None);
    }

    /// Everything the [`psys::Stage`] is playing this frame, split by blend
    /// class the same way [`Self::spark_vertices`] is.
    ///
    /// The rocket flares and the detonations today. Uploaded through the same
    /// [`oag_fx::psys::Pipeline`] as the sparks - one pass, two buffers,
    /// no third pipeline per effect.
    pub fn extend_stage_vertices(
        &self,
        additive: &mut Vec<oag_mesh::mesh::GpuVertex>,
        alpha_over: &mut Vec<oag_mesh::mesh::GpuVertex>,
        right: Vec3,
        up: Vec3,
    ) {
        let camera = self.camera_frame();
        // The GE's guard band, on Pulse's PSP source alone - `psys::guard`.
        let guard = self
            .view
            .screen_flash
            .is_some()
            .then_some(psys::guard::GuardBand {
                eye: camera.position,
                right: camera.right,
                up: camera.up,
                forward: -camera.back,
            });
        self.view
            .stage
            .extend_vertices(additive, alpha_over, right, up, guard.as_ref());
        self.view
            .scenery_fx
            .stage()
            .extend_vertices(additive, alpha_over, right, up, None);
        self.view
            .scenery_fx
            .weather()
            .extend_vertices(additive, alpha_over, camera, right, up);
    }
}
