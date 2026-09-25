//! `--presented` on the front-end leg: the grade and the screen filter, the
//! way a windowed front-end frame gets them.
//!
//! The front end has no 3D scene, so it has no render scale and no upscaler
//! to put in the way ([ADR-0038](../../../../docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md)),
//! and until the screen filter landed a `--presented` front-end capture
//! missed nothing a player would notice but the grade. A filter is different:
//! its whole claim is that it covers the menus and the front end as much as
//! the track ([ADR-0053](../../../../docs/architecture/adr/0053-screen-filters-are-loadable-wgsl-after-the-composite.md)),
//! and a capture that could only show it over a race could not show that.
//!
//! So this is the window's own arrangement, at the window's own size: the
//! stage draws into a [`Framebuffer`]'s presentation target and
//! [`Framebuffer::composite`] writes the capture texture through the filter
//! and the grade. No `resolve_scene`, exactly as `Session::frame` skips it
//! for a stage with no scene.

use anyhow::{Context, Result};

use crate::settings::Settings;
use crate::upscale::{Composite, Framebuffer, ScreenFrame};

/// The target a `--presented` front-end capture draws into, and what puts it
/// on the capture texture afterwards.
pub(super) struct Presented {
    framebuffer: Framebuffer,
    frame: Composite,
}

impl Presented {
    /// `None` unless `--presented` was given: the ordinary capture draws
    /// straight into its own texture, ungraded, and keeps doing so.
    ///
    /// # Errors
    ///
    /// A blit pipeline that will not build, which is a build-time mistake.
    #[allow(
        clippy::too_many_arguments,
        reason = "title and platform are two halves of one settings key \
                  (`crate::settings::profile_key`) rather than a single \
                  parameter a struct would clarify; every other parameter \
                  here is already a separate thing this needs"
    )]
    pub(super) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        settings: &Settings,
        title: &oag_title::Title,
        platform: oag_disc::Platform,
        space: oag_display::space::Space,
        presented: bool,
    ) -> Result<Option<Self>> {
        if !presented {
            return Ok(None);
        }
        let profile = settings
            .render_profiles
            .get(&crate::settings::profile_key(title, platform))
            .cloned()
            .unwrap_or_default();
        let mut framebuffer =
            Framebuffer::new(device, format, size).context("building the blit pipeline")?;
        framebuffer.resize_output(device, size);
        // The same catalogue the window reads, so a capture shows the preset
        // a player would see - their own `shaders/` directory included.
        let catalogue = crate::screen::Catalogue::load(crate::screen::Catalogue::directory());
        framebuffer.set_screen_filter(device, catalogue.get(&profile.screen_filter));
        Ok(Some(Self {
            framebuffer,
            frame: Composite {
                brightness: settings.display.brightness,
                gamma: settings.display.gamma,
                screen: ScreenFrame {
                    native: space.size,
                    strength: profile.screen_filter_strength,
                },
            },
        }))
    }

    /// Where the stage draws: the presentation target.
    pub(super) fn view(&self) -> &wgpu::TextureView {
        self.framebuffer.output()
    }

    /// Filters and grades the presentation target onto `surface`.
    pub(super) fn composite(
        mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        surface: &wgpu::TextureView,
    ) {
        self.framebuffer
            .composite(device, queue, encoder, surface, self.frame);
    }
}
