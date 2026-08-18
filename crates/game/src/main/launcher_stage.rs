//! The disc chooser on screen: the list, and the renderer that draws it.
//!
//! The one stage that runs before anything has been opened, which is what
//! decides everything about it. There is no disc, so there is no font, no
//! sprite sheet and no title skin - it draws with [`oag_game::font::Atlas::build`]
//! and [`oag_game::sprite::Sheet::default`], the same disc-free pair the
//! performance overlay is built from, and in the 480x272 grid the renderer
//! defaults to.

use oag_game::launcher::{self, Launcher};
use oag_game::render::Renderer;
use oag_gameplay::input::Input;

use crate::gpu::Gpu;

/// What the window shows while a player is choosing which image to boot.
pub(crate) struct LauncherStage {
    pub(crate) renderer: Renderer,
    pub(crate) launcher: Launcher,
    /// The source the player settled on, waiting for the top of the next frame.
    ///
    /// Held for one frame rather than acted on where it is raised, for the
    /// reason `Session::frame`'s `Launch Game` check gives for its own
    /// ordering: the load that follows stalls the window for a second or so,
    /// and the frame that shows the row lighting up should be on screen before
    /// it does.
    pub(crate) picked: Option<String>,
}

impl LauncherStage {
    /// One tick: move the cursor, and take a confirmation if one arrives.
    pub(crate) fn update(&mut self, input: &mut Input) {
        if self.picked.is_some() {
            return;
        }
        self.picked = self.launcher.update(input);
    }

    /// Draws the list into `target`.
    pub(crate) fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
    ) {
        let list = launcher::draw_list(&self.launcher);
        self.renderer.render(
            &gpu.device,
            &gpu.queue,
            encoder,
            target,
            &list,
            viewport,
            None,
        );
    }
}
