//! [`Scene::render_viewports`]: split screen, one scene draw per local player.
//!
//! One target, `N` rectangles of it ([`oag_render::viewport::layout`]), each drawn
//! by an ordinary [`Scene::render`] from its own camera
//! ([`Race::set_viewpoint`]). Four things make that more than a loop, and each
//! is a way to get a plausible picture that is wrong:
//!
//! - **One encoder and one submit per view.** `Scene::render` writes every
//!   drawable's uniforms with `queue.write_buffer`, and a queue write lands
//!   before the *submission's* commands, not at its place in the encoder. Two
//!   views recorded into one encoder would both draw with the last view's
//!   matrices: every rectangle the same camera.
//! - **Only the first view clears.** The scene pass clears the whole attachment
//!   whatever the viewport is, so the later views load what is there
//!   ([`Scene::keep_target`]). Depth is cleared each time; the views do not
//!   overlap and nothing reads one view's depth in another. Under MSAA the
//!   resolve rewrites the whole target from the multisampled attachment, which
//!   is why the colour attachment's load is switched, not only the resolve's.
//! - **The Pulse bloom is composited per view, onto the scene target**, instead of
//!   once after the HUD: its intermediates are one set, rebuilt by every
//!   [`Scene::render`], and the split draws no HUD to sit under it.
//! - **No motion blur and no jitter.** [`MotionState`] and the temporal
//!   upscaler's frame record hold one camera's history, and `N` cameras a tick
//!   would corrupt it. **Chosen, not measured.**
//!
//! Wipeout HD's and Omega's post chains encode into `view` per call and are not
//! verified under a split; Pulse and Pure are what was checked.

use super::*;

impl Scene {
    /// Draws `race` into the rectangles `race.viewport_count()` cuts `area` into,
    /// and returns the sum of what each drew. Submits its own command buffers,
    /// so everything the caller has already encoded must be submitted before
    /// this, and the caller's own encoder after it.
    ///
    /// With one view this is not the call to make: use [`Scene::render`], which
    /// is what keeps a race without a split exactly as it was.
    #[allow(clippy::too_many_arguments)]
    pub fn render_viewports(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        race: &Race,
        area: (f32, f32, f32, f32),
        fov: oag_display::display::Fov,
        cull: bool,
        pvs_cull: bool,
        anim_seconds: Option<f32>,
        shadows: oag_display::display::Shadows,
        zone_spectrum: &[f32],
    ) -> SceneStats {
        let rects = oag_render::viewport::layout(race.viewport_count(), area);
        let mut stats = SceneStats::default();
        for (slot, rect) in rects.into_iter().enumerate() {
            race.set_viewpoint(slot);
            self.keep_target.set(slot > 0);
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("race viewport"),
            });
            stats.add(self.render(
                device,
                queue,
                &mut encoder,
                view,
                race,
                rect,
                fov,
                cull,
                pvs_cull,
                anim_seconds,
                oag_display::display::MotionBlur::Off,
                shadows,
                None,
                zone_spectrum,
                None,
                None,
                None,
            ));
            self.composite_bloom(&mut encoder, view, rect);
            queue.submit(Some(encoder.finish()));
        }
        self.keep_target.set(false);
        race.reset_viewpoint();
        stats
    }
}
