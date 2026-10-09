//! What the web build starts from, and the one ceiling it keeps.
//!
//! **All of it is chosen, not measured.** A browser tab shares its GPU and its
//! main thread with the page, runs under `Fifo` presentation, and is reached by
//! players on machines nobody has profiled, so a first visit starts light. These
//! apply only when no `settings.toml` exists yet (no `localStorage` item); a
//! player who changed a row keeps theirs, and so does one whose profile was
//! written before this existed. The functions are plain data so a native test
//! can hold them; only the call sites are `cfg(target_arch = "wasm32")`.

use oag_mesh::mesh_render::Anisotropy;

use super::Settings;

/// The tallest render target the web build draws, in pixels. Render scale is a
/// percentage of the canvas, so on a large or high-density screen it alone
/// would still ask for a 4K target; this clamps the height and scales the width
/// by the same factor, and the upscale pass fills the canvas. A first-visit
/// default on the web: see [`capped`].
pub const RENDER_HEIGHT_CAP: u32 = 720;

/// `size` with its height held to `cap`, the width scaled by the same factor
/// (rounded, at least 1) so the shape is kept.
#[must_use]
pub fn cap_height(size: (u32, u32), cap: u32) -> (u32, u32) {
    let (width, height) = size;
    if height <= cap || height == 0 {
        return size;
    }
    let factor = cap as f32 / height as f32;
    (((width as f32 * factor).round() as u32).max(1), cap)
}

/// Whether a render target at `scale` is held to [`RENDER_HEIGHT_CAP`]: on the
/// web, only at the default scale. A player who picks another render scale
/// gets it as on a desktop (so 100 itself is the capped default; 125 and up
/// go past the cap, and 50 and 75 go below it).
#[must_use]
pub fn capped(scale: oag_display::display::Scale) -> bool {
    cfg!(target_arch = "wasm32") && scale == oag_display::display::Scale::default()
}

/// The render target for `rect` at `scale`: [`oag_present::upscale::target_size`],
/// held to [`RENDER_HEIGHT_CAP`] where [`capped`] says so.
#[must_use]
pub fn render_target(
    rect: (f32, f32, f32, f32),
    scale: oag_display::display::Scale,
    limit: u32,
) -> (u32, u32) {
    let size = oag_present::upscale::target_size(rect, scale, limit);
    if capped(scale) {
        cap_height(size, RENDER_HEIGHT_CAP)
    } else {
        size
    }
}

/// The rows the web build drops from the options pages: the browser surface
/// only offers `Fifo`, so VSYNC has nothing to choose, and a page has no
/// monitor to pick.
pub const HIDDEN_ROWS: [&str; 2] = ["display.vsync", "display.monitor"];

/// The rows only the web build has: the canvas size stands in for the window
/// size, which a page cannot have.
pub const WEB_ONLY_ROWS: [&str; 1] = ["display.canvas_size"];

/// serde `skip_serializing_if` for `display.canvas_size`: a desktop file does
/// not gain a key that means nothing there (it is still read if present).
#[must_use]
pub fn not_on_the_web<T>(_: &T) -> bool {
    !cfg!(target_arch = "wasm32")
}

/// What a canvas is sized to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasSize {
    /// The whole page: the stylesheet's own size.
    Fit,
    /// This many CSS pixels.
    Fixed(oag_display::display::Size),
}

impl CanvasSize {
    /// `fit` or `1280x720`; anything else is `Fit`, so a hand-edited file
    /// cannot leave a page with no canvas.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        text.parse().map_or(Self::Fit, Self::Fixed)
    }
}

/// The first-run values: frame limit 60, the lowest anisotropy, shadows off in
/// every render profile. Called on a settings value freshly made from defaults,
/// after its profiles exist.
pub fn first_run(settings: &mut Settings) {
    settings.display.frame_limit = "60".parse().expect("60 is a frame limit");
    settings.graphics.anisotropy = Anisotropy::Off;
    for profile in settings.render_profiles.values_mut() {
        profile.shadows = oag_display::display::Shadows::Off;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tall_target_is_held_to_the_cap_and_keeps_its_shape() {
        assert_eq!(cap_height((3840, 2160), 720), (1280, 720));
        assert_eq!(cap_height((1920, 1080), 720), (1280, 720));
    }

    #[test]
    fn a_target_under_the_cap_is_untouched() {
        assert_eq!(cap_height((960, 544), 720), (960, 544));
        assert_eq!(cap_height((1280, 720), 720), (1280, 720));
    }

    #[test]
    fn a_canvas_size_is_fit_unless_it_is_a_size() {
        assert_eq!(CanvasSize::parse("fit"), CanvasSize::Fit);
        assert_eq!(CanvasSize::parse("nonsense"), CanvasSize::Fit);
        assert_eq!(
            CanvasSize::parse("1280x720"),
            CanvasSize::Fixed(oag_display::display::Size::new(1280, 720))
        );
    }

    #[test]
    fn a_desktop_file_does_not_gain_a_canvas_size() {
        let text = toml::to_string_pretty(&Settings::default()).expect("serialises");
        assert!(!text.contains("canvas_size"), "{text}");
        let read: Settings =
            toml::from_str("[display]\ncanvas_size = \"960x544\"\n").expect("reads");
        assert_eq!(read.display.canvas_size, "960x544");
    }

    #[test]
    fn a_first_run_is_light() {
        let mut settings = Settings::default();
        crate::settings::render_profile::ensure_known_titles(&mut settings);
        first_run(&mut settings);
        assert_eq!(settings.display.frame_limit.hz(), Some(60));
        assert_eq!(settings.graphics.anisotropy, Anisotropy::Off);
        assert!(!settings.render_profiles.is_empty());
        assert!(
            settings
                .render_profiles
                .values()
                .all(|p| p.shadows == oag_display::display::Shadows::Off)
        );
    }
}
