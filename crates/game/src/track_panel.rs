//! The track-description panel over the pre-race flyby: read off the disc at race load, drawn by
//! [`Overlay`] over the picture while the flyby plays and fades out behind the chase view.
//!
//! The layout is `InGameTrackDescriptionScreen` in `InGame_Definition.xml`, the two strings are
//! the circuit's name and `MSC_TRACK_<nn>` paragraph in the language's string table, and the
//! timing is [`oag_ui_screens::track_panel`]'s. What reads and what does not:
//!
//! - **Pulse off a PSP disc only**, the one executable the screen was read off.
//! - A string, the screen or a texture that will not read leaves the panel out, and the load
//!   report says which. Nothing stands in for it.
//!
//! See `docs/gameplay/race-intro.md`.

use oag_raceplay::track_panel::Assets;
use oag_ui_screens::track_panel::Progress;

/// Draws the panel: two renderers, because a renderer binds one face and the name is `Menu`,
/// the paragraph `Default`.
pub struct Overlay {
    assets: Assets,
    frame: crate::render::Renderer,
    body: crate::render::Renderer,
}

impl std::fmt::Debug for Overlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Overlay")
            .field("name", &self.assets.name())
            .finish()
    }
}

impl Overlay {
    /// Builds the two renderers.
    ///
    /// # Errors
    ///
    /// Propagates pipeline creation.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        assets: Assets,
    ) -> anyhow::Result<Self> {
        let frame = crate::render::Renderer::new(
            device,
            queue,
            format,
            None,
            assets.menu_font.clone(),
            &assets.sheet,
        )?;
        let body = crate::render::Renderer::new(
            device,
            queue,
            format,
            None,
            assets.default_font.clone(),
            &assets.sheet,
        )?;
        Ok(Self {
            assets,
            frame,
            body,
        })
    }

    /// Draws the panel over what is in `view`, or nothing once it has faded out.
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        progress: Progress,
        viewport: (f32, f32, f32, f32),
    ) {
        if !progress.visible() {
            return;
        }
        let lists = self.assets.draw_lists(progress);
        let load = wgpu::LoadOp::Load;
        // The name is the frame list's last draw; both texts are clipped to the wipe.
        let name = lists.frame.len().saturating_sub(1);
        self.frame.render_with(
            load,
            device,
            queue,
            encoder,
            view,
            &lists.frame,
            viewport,
            Some((name, 0.0, lists.right)),
        );
        self.body.render_with(
            wgpu::LoadOp::Load,
            device,
            queue,
            encoder,
            view,
            &lists.body,
            viewport,
            Some((0, 0.0, lists.right)),
        );
    }
}

/// Builds an overlay for `assets` and draws it once - what a headless capture does, with no
/// stage to keep one in. A panel that will not build is skipped with a warning.
#[expect(
    clippy::too_many_arguments,
    reason = "a draw call's own device, target and pose, each a separate fact"
)]
pub fn draw_once(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    assets: Assets,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    progress: Progress,
    viewport: (f32, f32, f32, f32),
) {
    match Overlay::new(device, queue, format, assets) {
        Ok(mut panel) => panel.draw(device, queue, encoder, view, progress, viewport),
        Err(why) => log::warn!("the track panel did not build ({why}); capturing without it"),
    }
}
