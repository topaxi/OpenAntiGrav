//! The particle geometry a frame gathers: the collision sparks' and the stage's.
//!
//! Split out of `effects.rs` under the 1,000-line rule; a move.

use super::*;

impl Race {
    /// Forgets every particle pool's last tick of motion
    /// (`oag_fx::psys::System::hold_still`), at the start of every tick path:
    /// a pool the path then advances takes this tick's move, and one it does
    /// not draws still between the ticks rather than replaying a stale move.
    /// The weather's fields need none: they draw with no move at all.
    pub(crate) fn hold_particles_still(&mut self) {
        self.view.stage.hold_still();
        self.view.contact.sparks.hold_still();
        for fx in self.view.viewpoints.contact.iter_mut().flatten() {
            fx.sparks.hold_still();
        }
        self.view.scenery_fx.hold_still();
    }

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
        // The player's pool and every other local seat's: world-space particles, drawn in
        // every view.
        let seats = self
            .view
            .viewpoints
            .contact
            .iter()
            .flatten()
            .map(|fx| &**fx);
        for fx in std::iter::once(&self.view.contact).chain(seats) {
            fx.sparks
                .extend_vertices(additive, alpha_over, effect, right, up, None);
        }
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
        // That source is the one whose flash answers every kind; Pure's
        // Quake-only wash is not a sign of it.
        let guard = self
            .view
            .screen_flash
            .as_ref()
            .is_some_and(oag_fx::flash::ScreenFlash::answers_every_kind)
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
        if self.view.weather.is_enhanced() {
            self.view
                .scenery_fx
                .weather()
                .extend_splash_vertices(additive, alpha_over, right, up);
        }
    }

    /// The blend class 8 quads (heat haze, shock rings) everything the stage
    /// plays has this frame: the geometry of the offset pass, not of either of
    /// [`Self::extend_stage_vertices`]'s lists. See `oag_fx::psys::Blend::Distort`.
    pub fn extend_distort_vertices(
        &self,
        out: &mut Vec<oag_mesh::mesh::GpuVertex>,
        right: Vec3,
        up: Vec3,
    ) {
        self.view.stage.extend_distort_vertices(out, right, up);
        self.view
            .scenery_fx
            .stage()
            .extend_distort_vertices(out, right, up);
    }
}
