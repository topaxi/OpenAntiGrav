//! The four render flags applied to one profile - its own file so `cli.rs`
//! stays under the size ratchet, which it had a `--fury-path` of headroom to.

use super::Cli;

impl Cli {
    /// Applies `--render-scale`, `--upscaler`, `--anti-aliasing`,
    /// `--motion-blur`, `--shadows`, `--screen-filter`, `--lod` and `--texture-detail`'s preset
    /// to one render profile.
    ///
    /// **One function because there are two callers and they disagreed.**
    /// `main.rs` applies these by walking `settings.render_profiles`, which is
    /// keyed by title - and a headless `--race` capture has no title in hand,
    /// so it built a `RenderProfile::default()` instead and every one of these
    /// four flags was silently discarded. That made `just compare-upscalers`
    /// produce three byte-identical images, which is exactly the failure a
    /// comparison instrument cannot survive: it reported "these upscalers look
    /// the same" and nobody could tell that from "the flag did nothing".
    ///
    /// `render_scale` is parsed by the caller because it is the one of the four
    /// that can fail.
    pub(crate) fn apply_render_overrides(
        &self,
        profile: &mut crate::settings::RenderProfile,
        render_scale: Option<oag_display::display::Scale>,
    ) {
        if let Some(render_scale) = render_scale {
            profile.render_scale = render_scale;
        }
        if let Some(reconstruction) = self.reconstruction {
            profile.reconstruction = reconstruction;
        }
        if let Some(msaa) = self.msaa {
            profile.msaa = msaa;
        }
        if let Some(motion_blur) = self.blur.motion_blur {
            profile.motion_blur = motion_blur;
        }
        if let Some(resolution) = self.blur.motion_blur_resolution {
            profile.motion_blur_resolution = resolution;
        }
        if let Some(shadows) = self.shadows {
            profile.shadows = shadows;
        }
        if let Some(screen_filter) = &self.screen_filter {
            profile.screen_filter = screen_filter.clone();
        }
        if let Some(detail) = self.lod {
            profile.model_detail = detail;
        }
        if let Some(detail) = self.texture_detail {
            profile.texture_detail = detail;
        }
    }
}
