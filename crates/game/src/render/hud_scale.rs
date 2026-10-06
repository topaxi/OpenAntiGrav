//! How [`Renderer`] stretches a raster HUD: the sampler a mode wants, the
//! whole-texel sizes `integer` draws at, and the pixels-per-grid-unit both
//! need.
//!
//! The decision of *which* titles get a mode is `oag_title::HudArt::raster`,
//! made by the overlay that owns the renderer; this module only carries it
//! out. A renderer nobody calls [`Renderer::set_hud_scale`] on stays
//! [`HudScale::Linear`], which is every menu and every non-raster HUD.

use oag_display::display::{HudScale, integer_factor};

use super::*;

/// A renderer's HUD stretch: the mode, and how many output pixels one grid
/// unit covers this frame.
#[derive(Debug, Clone, Copy)]
pub(super) struct HudStretch {
    pub(super) mode: HudScale,
    /// Output pixels per grid unit of the fitted viewport, measured by
    /// [`Self::measure`] at the top of every frame. Zero until the first.
    pub(super) pixels_per_grid: f32,
}

impl Default for HudStretch {
    fn default() -> Self {
        Self {
            mode: HudScale::Linear,
            pixels_per_grid: 0.0,
        }
    }
}

impl HudStretch {
    /// Pixels per grid unit once `space` is letterboxed into `viewport`.
    pub(super) fn measure(space: Space, viewport: (f32, f32, f32, f32)) -> f32 {
        let [scale_x, _] =
            letterbox_in((viewport.2 as u32, viewport.3 as u32), space.display_aspect);
        viewport.2 * scale_x / space.size.0
    }

    /// The `text scale` an `integer` HUD draws a texel run at, so that each
    /// atlas texel covers a whole number of pixels. `scale` unchanged for any
    /// other mode and for a texel shown under one pixel.
    pub(super) fn text_scale(&self, texel: f32, scale: f32) -> f32 {
        if !self.mode.snaps_geometry() || self.pixels_per_grid <= 0.0 {
            return scale;
        }
        match integer_factor(scale * texel * self.pixels_per_grid) {
            Some(factor) => factor / (self.pixels_per_grid * texel),
            None => scale,
        }
    }

    /// `value` (grid units) moved onto a whole output pixel, for `integer`.
    pub(super) fn snap(&self, value: f32) -> f32 {
        if self.mode.snaps_geometry() && self.pixels_per_grid > 0.0 {
            (value * self.pixels_per_grid).round() / self.pixels_per_grid
        } else {
            value
        }
    }

    /// The `hud` uniform `ui.wesl` reads.
    pub(super) fn uniform(&self) -> [f32; 4] {
        let sharp = f32::from(u8::from(self.mode.is_sharp_bilinear()));
        [self.pixels_per_grid, sharp, 0.0, 0.0]
    }
}

/// Resizes every plain sprite quad to a whole number of pixels per texel on
/// each axis, anchored at its top left on a whole pixel.
///
/// Only quads that are a straight crop of the sheet: not rotated, tiled or
/// chamfered, and not a text or fill quad (text is sized as it is laid out,
/// see [`HudStretch::text_scale`]). A quad whose axis shows a texel under one
/// pixel keeps that axis.
pub(super) fn snap_sprites(quads: &mut [Quad], pixels_per_grid: f32) {
    if pixels_per_grid <= 0.0 {
        return;
    }
    let snapped = |value: f32| (value * pixels_per_grid).round() / pixels_per_grid;
    for quad in quads {
        let sprite = quad.mode == MODE_SPRITE || quad.mode == MODE_SPRITE_ADDITIVE;
        let plain = quad.rotation == 0.0 && quad.tile[0] <= 0.0 && quad.chamfer == [0.0, 0.0];
        if !(sprite && plain && quad.uv[2] > 0.0 && quad.uv[3] > 0.0) {
            continue;
        }
        let fit = |size: f32, texels: f32| {
            integer_factor(size * pixels_per_grid / texels)
                .map_or(size, |factor| texels * factor / pixels_per_grid)
        };
        quad.rect = [
            snapped(quad.rect[0]),
            snapped(quad.rect[1]),
            fit(quad.rect[2], quad.uv[2]),
            fit(quad.rect[3], quad.uv[3]),
        ];
    }
}

impl Renderer {
    /// Draws this renderer's atlases and sprites the way `mode` asks. Only
    /// the HUD's own renderers call it, and only for a title whose HUD is
    /// raster art.
    ///
    /// `integer` and `nearest` swap the glyph atlas's and the sprite sheet's
    /// samplers to nearest; `sharp-bilinear` keeps them linear and the shader
    /// blends only the last pixel of each texel. The built-in 5x7 set is
    /// nearest whatever this says. Rebuilds the bind group, so call it at
    /// setup rather than per frame.
    pub fn set_hud_scale(&mut self, device: &wgpu::Device, mode: HudScale) {
        self.hud.mode = mode;
        let filter = if mode.samples_nearest() {
            wgpu::FilterMode::Nearest
        } else {
            wgpu::FilterMode::Linear
        };
        let make = |label| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        if self.atlas.is_real() {
            self.atlas_sampler = make("atlas");
        }
        self.sprite_sampler = make("sprites");
        self.ui_bind_group = ui_bind_group(
            device,
            &self.ui_layout,
            &self.uniform_buffer,
            &self.atlas_view,
            &self.atlas_sampler,
            &self.sprite_view,
            &self.sprite_sampler,
            &self.face_view,
            &self.face_sampler,
            &self.buttons_view,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn integer_at(pixels_per_grid: f32) -> HudStretch {
        HudStretch {
            mode: HudScale::Integer,
            pixels_per_grid,
        }
    }

    fn sprite(rect: [f32; 4], uv: [f32; 4]) -> Quad {
        Quad {
            rect,
            uv,
            color: [1.0; 4],
            border: [1.0; 4],
            mode: MODE_SPRITE,
            rotation: 0.0,
            chamfer: [0.0, 0.0],
            tile: [0.0, 0.0],
        }
    }

    #[test]
    fn a_1080p_screen_covers_one_grid_unit_with_just_under_four_pixels() {
        let space = Space::PSP;
        let s = HudStretch::measure(space, (0.0, 0.0, 1920.0, 1080.0));
        assert!(
            (s - 1080.0 / 272.0).abs() < 1e-4,
            "the grid's 272 rows fill the height: {s}"
        );
        // A 16:10 Steam Deck panel letterboxes the 16:9 grid: width-limited.
        let deck = HudStretch::measure(space, (0.0, 0.0, 1280.0, 800.0));
        assert!((deck - 1280.0 / 480.0).abs() < 1e-3, "{deck}");
    }

    #[test]
    fn integer_text_covers_a_whole_number_of_pixels_per_texel() {
        let stretch = integer_at(1080.0 / 272.0);
        let scale = stretch.text_scale(1.0, 1.0);
        let pixels = scale * stretch.pixels_per_grid;
        assert!(
            (pixels - 4.0).abs() < 1e-3,
            "3.97 px per texel is 4 within the tolerance: {pixels}"
        );
        let small = stretch.text_scale(1.0, 0.6);
        assert!(
            (small * stretch.pixels_per_grid - 2.0).abs() < 1e-3,
            "0.6 x 3.97 = 2.4 floors to 2"
        );
    }

    #[test]
    fn every_other_mode_leaves_text_and_positions_alone() {
        for mode in [HudScale::Linear, HudScale::Nearest, HudScale::SharpBilinear] {
            let stretch = HudStretch {
                mode,
                pixels_per_grid: 3.97,
            };
            assert_eq!(stretch.text_scale(1.0, 0.8), 0.8);
            assert_eq!(stretch.snap(12.34), 12.34);
        }
    }

    #[test]
    fn a_sprite_drawn_at_native_size_snaps_to_whole_texels_on_a_whole_pixel() {
        let pixels_per_grid = 3.5;
        let mut quads = [sprite([10.3, 5.2, 40.0, 20.0], [8.0, 8.0, 40.0, 20.0])];
        snap_sprites(&mut quads, pixels_per_grid);
        let [x, y, w, h] = quads[0].rect;
        assert!(
            ((w * pixels_per_grid) - 120.0).abs() < 1e-2,
            "40 texels x 3 px"
        );
        assert!(
            ((h * pixels_per_grid) - 60.0).abs() < 1e-2,
            "20 texels x 3 px"
        );
        assert!((x * pixels_per_grid - (x * pixels_per_grid).round()).abs() < 1e-3);
        assert!((y * pixels_per_grid - (y * pixels_per_grid).round()).abs() < 1e-3);
    }

    #[test]
    fn rotated_tiled_and_text_quads_are_left_as_laid_out() {
        let mut rotated = sprite([1.0, 1.0, 40.0, 20.0], [0.0, 0.0, 40.0, 20.0]);
        rotated.rotation = 0.5;
        let mut tiled = sprite([1.0, 1.0, 40.0, 20.0], [0.0, 0.0, 40.0, 20.0]);
        tiled.tile = [2.0, 2.0];
        let mut glyph = sprite([1.3, 1.3, 40.0, 20.0], [0.0, 0.0, 40.0, 20.0]);
        glyph.mode = MODE_ATLAS;
        let mut quads = [rotated, tiled, glyph];
        let before: Vec<[f32; 4]> = quads.iter().map(|q| q.rect).collect();
        snap_sprites(&mut quads, 3.97);
        let after: Vec<[f32; 4]> = quads.iter().map(|q| q.rect).collect();
        assert_eq!(before, after);
    }
}
