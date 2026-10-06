//! The HUD's GPU half: three [`crate::render::Renderer`]s and the pass that drives
//! them.
//!
//! Split out of `oag_hud` so the layout model and the draw list stay testable
//! without a GPU, which is the reason those two are separated from each other in
//! the first place. Nothing here decides anything - every widget, string and
//! colour is already resolved by the time [`Overlay::draw`] runs.

use oag_hud::{Assets, Context, Layout, Readout, draw_list};

/// The stretch a title's HUD is drawn with: the player's `[graphics] hud_scale`
/// for a raster HUD, plain linear for every other.
#[must_use]
pub fn stretch_for(
    art: &oag_title::HudArt,
    requested: oag_display::display::HudScale,
) -> oag_display::display::HudScale {
    if art.raster {
        requested
    } else {
        oag_display::display::HudScale::Linear
    }
}

/// The HUD's three renderers and the data they draw.
///
/// # Why two renderers
///
/// [`crate::render::Renderer`] binds exactly one font atlas into an immutable
/// bind group and picks its sampler filter once from `Atlas::is_real`. The HUD
/// uses two fonts - `HUD` for values and `HUDSmall` for captions - so it needs two
/// of them and two passes. The alternative, a second atlas binding plus a third
/// `mode` value in `ui.wesl`, touches the bind group every existing screen depends
/// on; two renderers touch nothing. See `docs/ui/hud.md`.
///
/// Both are built with `video: None`. A video pipeline that is built and never
/// filled draws a **green** rectangle rather than nothing, the planes being zeroed
/// rather than absent - see `crate::render`.
pub struct Overlay {
    layout: Layout,
    strings: oag_ui::language::StringTable,
    /// Every texture the layout names, packed into one sheet. Held rather than
    /// reduced to one origin, because a layout may name six - see
    /// [`Assets::sheet`].
    sheet: oag_hud::sprite::Sheet,
    art: &'static oag_title::HudArt,
    hud_line_height: f32,
    small_line_height: f32,
    default_line_height: f32,
    /// The grid the layout is authored in - `Assets::space`, kept so the
    /// frame loop can hand a screen filter the title's own row count on the
    /// `--race` route, which has no menu shell to read it from.
    space: oag_display::space::Space,
    /// Draws the values, in `PulseHud.fnt`.
    values: crate::render::Renderer,
    /// Draws the captions, in `small.fnt`.
    captions: crate::render::Renderer,
    /// Draws the per-craft rows, in the `Default` face.
    rows: crate::render::Renderer,
}

impl std::fmt::Debug for Overlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Overlay")
            .field("widgets", &self.layout.widget_count())
            .field("strings", &self.strings.len())
            .field("textures", &self.sheet.len())
            .finish()
    }
}

impl Overlay {
    /// Builds the two renderers, or `None` when there is no layout to draw.
    ///
    /// # Errors
    ///
    /// Propagates pipeline creation.
    ///
    /// `hud_scale` is `[graphics] hud_scale`, which only a title whose HUD is
    /// raster art ([`oag_title::HudArt::raster`]) obeys.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        assets: &Assets,
        hud_scale: oag_display::display::HudScale,
    ) -> anyhow::Result<Option<Self>> {
        let Some(layout) = assets.layout.clone() else {
            return Ok(None);
        };

        let mut values = crate::render::Renderer::new(
            device,
            queue,
            format,
            None,
            assets.font.clone(),
            &assets.sheet,
        )?;
        // The layout's own grid, on both renderers. A `Renderer` starts at
        // `Space::PSP` and every widget rect here is in the *source's* space -
        // see `Assets::space`, which is where the HD case is written down.
        values.set_space(assets.space);
        // The captions renderer never draws a sprite, but it binds the sheet
        // anyway: `Renderer::new` takes one unconditionally, and a second copy of
        // a 256x256 atlas is cheaper than making the parameter optional.
        let mut captions = crate::render::Renderer::new(
            device,
            queue,
            format,
            None,
            assets.small_font.clone(),
            &assets.sheet,
        )?;
        captions.set_space(assets.space);
        let mut rows = crate::render::Renderer::new(
            device,
            queue,
            format,
            None,
            assets.default_font.clone(),
            &assets.sheet,
        )?;
        rows.set_space(assets.space);
        // The player's stretch, for a title whose HUD is raster art; every
        // other HUD keeps the linear draw it was made for.
        let stretch = stretch_for(assets.art, hud_scale);
        for renderer in [&mut values, &mut captions, &mut rows] {
            renderer.set_hud_scale(device, stretch);
        }

        Ok(Some(Self {
            hud_line_height: assets.font.line_height,
            small_line_height: assets.small_font.line_height,
            default_line_height: assets.default_font.line_height,
            sheet: assets.sheet.clone(),
            art: assets.art,
            strings: assets.strings.clone(),
            space: assets.space,
            layout,
            values,
            captions,
            rows,
        }))
    }

    /// The grid this HUD is authored in: the title's own, per `Assets::space`.
    #[must_use]
    pub fn space(&self) -> oag_display::space::Space {
        self.space
    }

    /// The context [`draw_list`] takes.
    #[must_use]
    pub fn context(&self) -> Context<'_> {
        Context {
            layout: &self.layout,
            strings: &self.strings,
            sheet: &self.sheet,
            art: self.art,
            hud_line_height: self.hud_line_height,
            small_line_height: self.small_line_height,
            default_line_height: self.default_line_height,
            default_border: self.layout.default_border(),
        }
    }

    /// Draws the HUD **over** whatever is already in `view`.
    ///
    /// Up to three passes, all `LoadOp::Load`: sprites and values through the
    /// `HUD` renderer, captions through the `HUDSmall` one, then per-craft rows
    /// through the `Default` one. The sprites go with the
    /// values because they share a renderer and the sprite sheet is bound in both.
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        readout: &Readout,
        viewport: (f32, f32, f32, f32),
    ) {
        let frame = draw_list(&self.context(), readout);
        if frame.is_empty() {
            return;
        }

        // Sprites first so text sits over the bars, and both in one pass because
        // one renderer holds both the sheet and the value font.
        let mut first = frame.sprites;
        first.extend(frame.hud_text);
        if !first.is_empty() {
            self.values
                .overlay(device, queue, encoder, view, &first, viewport);
        }
        if !frame.small_text.is_empty() {
            self.captions
                .overlay(device, queue, encoder, view, &frame.small_text, viewport);
        }
        if !frame.default_text.is_empty() {
            self.rows
                .overlay(device, queue, encoder, view, &frame.default_text, viewport);
        }
    }
}
