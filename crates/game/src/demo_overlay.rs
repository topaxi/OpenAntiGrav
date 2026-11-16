//! The screen an attract demo draws over its race: [`Overlay`].
//!
//! What it draws is `oag_raceplay::demo_overlay`'s, read off the disc at race load. The drawing is
//! the front end's own (`oag_ui::frontend::Frontend::draw_screen_over`), so the logo's fade, the
//! rules' slide and the text's pulse are the ones the disc's boot screens use, run on the demo's
//! clock.

use oag_raceplay::demo_overlay::Assets;
use oag_ui::frontend::{Draw, Frontend};

/// Draws the screen over a demo.
pub struct Overlay {
    frontend: Frontend,
    state: &'static str,
    renderer: crate::render::Renderer,
}

impl std::fmt::Debug for Overlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Overlay")
            .field("state", &self.state)
            .finish()
    }
}

impl Overlay {
    /// Builds the renderer for `assets`.
    ///
    /// # Errors
    ///
    /// Propagates pipeline creation.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        assets: &Assets,
    ) -> anyhow::Result<Self> {
        let mut renderer = crate::render::Renderer::new(
            device,
            queue,
            format,
            None,
            assets.font.clone(),
            &assets.sheet,
        )?;
        renderer.set_space(assets.space);
        Ok(Self {
            frontend: assets.frontend(),
            state: assets.state(),
            renderer,
        })
    }

    /// The screen `seconds` after the demo began.
    #[must_use]
    pub fn draw_list(&self, seconds: f32) -> Vec<Draw> {
        self.frontend
            .draw_screen_over(self.state, f64::from(seconds))
    }

    /// Draws the screen over what is in `view`.
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        seconds: f32,
        viewport: (f32, f32, f32, f32),
    ) {
        let list = self.draw_list(seconds);
        self.renderer.render_with(
            wgpu::LoadOp::Load,
            device,
            queue,
            encoder,
            view,
            &list,
            viewport,
            None,
        );
    }
}

/// Builds an overlay for `assets` and draws it once - what a headless capture does, with no
/// stage to keep one in. A screen that will not build is skipped with a warning.
pub fn draw_once(
    (device, queue, format): (&wgpu::Device, &wgpu::Queue, wgpu::TextureFormat),
    assets: Option<&Assets>,
    (encoder, view, viewport): (
        &mut wgpu::CommandEncoder,
        &wgpu::TextureView,
        (f32, f32, f32, f32),
    ),
    seconds: f32,
) {
    let Some(assets) = assets else {
        return;
    };
    match Overlay::new(device, queue, format, assets) {
        Ok(mut screen) => screen.draw(device, queue, encoder, view, seconds, viewport),
        Err(why) => log::warn!("the demo screen did not build ({why}); capturing without it"),
    }
}
