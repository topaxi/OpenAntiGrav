//! The loading screen: shown while the boot's movies finish decoding, and
//! again between the menus and a race while the circuit is read off the disc.
//!
//! One stage for both because it is one screen, and which wait it is covering
//! is the difference between two of its fields being `Some`. See
//! [`oag_game::loading::Phase`], which is the same distinction on the drawing
//! side.

use oag_game::render::Renderer;
use oag_game::{boot, capture, loading, prefetch, race};

use crate::gpu::Gpu;

/// The loading screen: two passes, and the front end waiting behind it.
///
/// It holds no [`prefetch::Prefetch`], deliberately. The worker handle lives on
/// [`Session`], because this stage is *replaced* when the wait is over and a
/// handle stored here would be dropped by that replacement - which sets the
/// stop flag and silently abandons whatever conversion was still to do. What
/// arrives here instead is a [`prefetch::Progress`] snapshot, per frame, which
/// also leaves `loading::Screen` drivable from a test with no worker at all.
pub(crate) struct LoadingStage {
    pub(crate) renderer: Renderer,
    /// The wave's own pipeline: additive, screen-space, no depth.
    pub(crate) wave: oag_render::loading::Pipeline,
    pub(crate) screen: loading::Screen,
    /// The atlas the layout measures its wrapping and eliding with. [`Renderer`]
    /// owns a copy and does not lend it out.
    pub(crate) atlas: oag_game::font::Atlas,
    /// The cheap half of the boot, waiting for the other one. `None` once
    /// taken, which is also what stops the hand-off happening twice.
    pub(crate) shell: Option<boot::Shell>,
    /// The movies, still decoding. **What this screen is actually waiting for**
    /// on an ordinary boot - `--prefetch`, when it is on, is waited for as well.
    pub(crate) media: Option<boot::MediaWorker>,
    /// The circuit, still being read. `Some` on the race path and `None` on the
    /// boot one, which is the whole of what tells the two apart.
    ///
    /// Exclusive with [`Self::shell`] in practice and not by type: the boot path
    /// carries a shell and no worker, the race path a worker and no shell. An
    /// enum would say so, and would also mean every field above it appearing
    /// twice - see [`Session::finish_loading`], which branches on exactly this
    /// pair and is the only reader that has to care.
    pub(crate) race: Option<race::LoadWorker>,
    pub(crate) trace: bool,
}

impl LoadingStage {
    /// Whether the movies have landed, so the fade may start.
    pub(crate) fn media_ready(&self) -> bool {
        self.media
            .as_ref()
            .is_none_or(boot::MediaWorker::is_finished)
    }

    /// Whether the circuit has landed, on a screen that is waiting for one.
    ///
    /// `true` on the boot path, which waits for no circuit - so a caller can
    /// require both this and [`Self::media_ready`] without asking which path it
    /// is on.
    pub(crate) fn race_ready(&self) -> bool {
        self.race.as_ref().is_none_or(race::LoadWorker::is_finished)
    }
}

impl LoadingStage {
    /// Draws the text, then the wave over it.
    ///
    /// In that order and in two passes because they are two different blends:
    /// the UI list is alpha-over and clears the frame, the wave is additive over
    /// what is already there. See [`capture::draw_wave`], which is the same
    /// second pass and is shared so the window and a capture cannot drift.
    pub(crate) fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        phase: loading::Phase,
        progress: &prefetch::Progress,
    ) {
        let quads = self.screen.quads();
        let vertices = oag_render::loading::vertices(&quads);
        self.wave.upload(
            &gpu.queue,
            [1.0, 1.0, 1.0, self.screen.opacity()],
            &vertices,
        );
        let list = self.screen.draw_list(phase, progress, &self.atlas);
        self.renderer.render(
            &gpu.device,
            &gpu.queue,
            encoder,
            view,
            &list,
            viewport,
            None,
        );
        capture::draw_wave(encoder, view, &self.wave, viewport);
    }
}
