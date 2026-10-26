//! The HUD's GPU half: three [`crate::render::Renderer`]s and the pass that drives
//! them.
//!
//! Split out of `oag_hud` so the layout model and the draw list stay testable
//! without a GPU, which is the reason those two are separated from each other in
//! the first place. Nothing here decides anything - every widget, string and
//! colour is already resolved by the time [`Overlay::draw`] runs.

use oag_hud::{Assets, Context, Frame, Layout, Placement, Readout, Span, draw_list};

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

/// `frame` drawn at `scale` about the centre of a `size` grid: the whole HUD moves in toward the
/// middle of the screen, as a title's `Safe Area Setting` does ([`oag_title::hud::SafeArea`]).
/// A scale of `1.0` returns the frame as it was.
#[must_use]
pub fn safe_area_frame(frame: Frame, scale: f32, size: (f32, f32)) -> Frame {
    if (scale - 1.0).abs() < f32::EPSILON || frame.is_empty() {
        return frame;
    }
    let placement = Placement {
        scale,
        offset: [(1.0 - scale) * size.0 * 0.5, (1.0 - scale) * size.1 * 0.5],
        clip: [-1.0e9, -1.0e9, 2.0e9, 2.0e9],
    };
    frame.placed(&placement)
}

/// One view of a split screen: the layout it draws from, the slot's readout and
/// where the HUD goes.
#[derive(Debug)]
pub struct ViewDraw<'a> {
    /// The view's own context: the whole-screen layout, or the title's authored
    /// layout for this player.
    pub cx: Context<'a>,
    /// What the view's slot reads.
    pub readout: Readout,
    /// Where the layout goes in the screen's grid.
    pub placement: Placement,
    /// Where the reticle goes, when it is not where the layout goes. An authored
    /// split layout is whole-screen and the reticle is view-relative, so it moves
    /// by the view's own offset. `None` draws it with the rest.
    pub sight: Option<Placement>,
    /// Draw only the reticle, with [`Self::sight`]'s placement. A split-HUD file
    /// places each group of widgets on its own, so the reticle, which is not a
    /// widget of any group, is one more draw of the view's own.
    pub sight_only: bool,
}

/// The draw list of a split screen: each view's HUD from its own `Readout`, moved
/// to its [`Placement`], in one [`Frame`]. GPU-free, so a test can ask what every
/// view shows.
#[must_use]
pub fn views_frame(views: &[ViewDraw<'_>]) -> Frame {
    let mut frame = Frame::default();
    for view in views {
        let sight_draws =
            |sight: &oag_race::sight::Sight, at: &Placement| -> Vec<oag_ui::frontend::Draw> {
                oag_hud::sight_draws(&view.cx, sight)
                    .iter()
                    .filter_map(|draw| oag_hud::place(draw, at))
                    .collect()
            };
        if view.sight_only {
            let mut mine = Frame::default();
            if let (Some(sight), Some(at)) = (&view.readout.sight, &view.sight) {
                mine.sprites = sight_draws(sight, at);
            }
            mine.spans.push(Span {
                clip: view.placement.clip,
                start: [0; 5],
            });
            frame.extend(mine);
            continue;
        }
        let Some(sight_at) = &view.sight else {
            frame.extend(draw_list(&view.cx, &view.readout).placed(&view.placement));
            continue;
        };
        let mut readout = view.readout.clone();
        let sight = readout.sight.take();
        let mut mine = draw_list(&view.cx, &readout).placed(&view.placement);
        if let Some(sight) = sight {
            mine.sprites.extend(sight_draws(&sight, sight_at));
        }
        frame.extend(mine);
    }
    frame
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
    skin_line_height: f32,
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
    /// Draws the retro skins' own face (`2097HUD`, `wo3HUD`).
    skin: crate::render::Renderer,
    /// The authored split layouts, [`Assets::split`].
    split: Option<oag_hud::SplitAssets>,
    split_columns: Option<oag_hud::SplitAssets>,
    split_hud: Option<oag_hud::split_hud::SplitHudAssets>,
    /// The whole HUD's scale about the screen's centre, [`oag_title::HudArt::safe_area`] at the
    /// player's value; `1.0` for a title with no such option.
    safe_area_scale: f32,
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
        let mut skin = crate::render::Renderer::new(
            device,
            queue,
            format,
            None,
            assets.skin_font.clone(),
            &assets.sheet,
        )?;
        skin.set_space(assets.space);
        // The player's stretch, for a title whose HUD is raster art; every
        // other HUD keeps the linear draw it was made for.
        let stretch = stretch_for(assets.art, hud_scale);
        for renderer in [&mut values, &mut captions, &mut rows, &mut skin] {
            renderer.set_hud_scale(device, stretch);
            renderer.set_whole_pixel_advances(assets.art.raster);
            renderer.set_ink_aligned_text(assets.art.ink_aligned_text);
            renderer.set_half_unit_glyph_quads(assets.art.half_unit_glyph_quads);
        }

        Ok(Some(Self {
            hud_line_height: assets.font.line_height,
            small_line_height: assets.small_font.line_height,
            default_line_height: assets.default_font.line_height,
            skin_line_height: assets.skin_font.line_height,
            sheet: assets.sheet.clone(),
            art: assets.art,
            strings: assets.strings.clone(),
            space: assets.space,
            layout,
            values,
            captions,
            rows,
            skin,
            split: assets.split.clone(),
            split_columns: assets.split_columns.clone(),
            split_hud: assets.split_hud.clone(),
            safe_area_scale: assets
                .art
                .safe_area
                .map_or(1.0, |area| area.value.scale(None)),
        }))
    }

    /// The player's `[graphics] hud_safe_area` (`None`: the title's own default). A title with
    /// no `Safe Area Setting` keeps its HUD at full size whatever this says.
    #[must_use]
    pub fn with_safe_area(mut self, percent: Option<u8>) -> Self {
        self.safe_area_scale = self
            .art
            .safe_area
            .map_or(1.0, |area| area.value.scale(percent));
        self
    }

    /// The placement [`Self::safe_area`] applies, or `None` at full size.
    fn safe_area_placement(&self) -> Option<Placement> {
        let scale = self.safe_area_scale;
        let size = self.space.size;
        ((scale - 1.0).abs() >= f32::EPSILON).then(|| Placement {
            scale,
            offset: [(1.0 - scale) * size.0 * 0.5, (1.0 - scale) * size.1 * 0.5],
            clip: [-1.0e9, -1.0e9, 2.0e9, 2.0e9],
        })
    }

    /// `frame` drawn at [`Self::safe_area_scale`] about the centre of the layout's grid.
    fn safe_area(&self, frame: Frame) -> Frame {
        safe_area_frame(frame, self.safe_area_scale, self.space.size)
    }

    /// The grid this HUD is authored in: the title's own, per `Assets::space`.
    #[must_use]
    pub fn space(&self) -> oag_display::space::Space {
        self.space
    }

    /// The title's authored split-screen layouts, when this race has them.
    #[must_use]
    pub fn split(&self) -> Option<&oag_hud::SplitAssets> {
        self.split.as_ref()
    }

    /// How this title draws a resume countdown. See
    /// [`oag_title::hud::ResumeCountdown`].
    #[must_use]
    pub fn resume_countdown(&self) -> oag_title::hud::ResumeCountdown {
        self.art.resume_countdown
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
            skin_line_height: self.skin_line_height,
            default_border: self.layout.default_border(),
        }
    }

    /// Draws the HUD **over** whatever is already in `view`.
    ///
    /// Up to three passes, all `LoadOp::Load`: sprites and values through the
    /// `HUD` renderer, captions through the `HUDSmall` one, then per-craft rows
    /// through the `Default` one. The sprites go with the
    /// values because they share a renderer and the sprite sheet is bound in both.
    ///
    /// `countdown` is the word of a resume countdown drawn as text
    /// ([`crate::hud_views::countdown_frame`]), put in the same frame as the HUD:
    /// the renderers write their buffers with `queue.write_buffer`, so a second
    /// pass in this encoder would overwrite the HUD's own text.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        readout: &Readout,
        countdown: Option<&str>,
        viewport: (f32, f32, f32, f32),
    ) -> Vec<oag_hud::boost_smear::GlowStamp> {
        let mut frame = self.safe_area(draw_list(&self.context(), readout));
        if let Some(text) = countdown {
            frame.extend(crate::hud_views::countdown_frame(
                &self.context(),
                text,
                1,
                oag_render::viewport::Cut::default(),
                viewport,
                self.space,
            ));
        }
        if let Some(marquee) = &readout.now_playing {
            let at = self.safe_area_placement();
            let marquee = at.map_or_else(|| marquee.clone(), |at| marquee.placed(&at));
            oag_hud::now_playing::append(
                &mut frame,
                &marquee,
                self.layout.default_border(),
                &self.sheet,
            );
        }
        let glow = frame.glow.clone();
        self.paint(device, queue, encoder, view, frame, viewport);
        glow
    }

    /// Draws one HUD per split-screen view, `readouts[i]` into view `i` - see
    /// [`crate::hud_views::view_draws`] for where each goes - **in the passes one
    /// HUD takes**.
    ///
    /// One list for the whole screen rather than a draw per view: the renderers
    /// write their uniform and quad buffers with `queue.write_buffer`, which
    /// lands before the submission's commands, so two draws recorded into one
    /// encoder would both show the last one's numbers. See
    /// `oag_raceplay::Scene::render_viewports` for the same trap in the scene.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_views(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        readouts: Vec<Readout>,
        countdown: Option<&str>,
        cut: oag_render::viewport::Cut,
        viewport: (f32, f32, f32, f32),
    ) -> Vec<oag_hud::boost_smear::GlowStamp> {
        let count = readouts.len();
        let mut frame = views_frame(&crate::hud_views::view_draws(
            self.context(),
            self.split.as_ref(),
            self.split_columns.as_ref(),
            self.split_hud.as_ref(),
            readouts,
            cut,
            viewport,
            self.space,
        ));
        if let Some(text) = countdown.filter(|_| count > 0) {
            frame.extend(crate::hud_views::countdown_frame(
                &self.context(),
                text,
                count,
                cut,
                viewport,
                self.space,
            ));
        }
        let glow = frame.glow.clone();
        self.paint(device, queue, encoder, view, frame, viewport);
        glow
    }

    fn paint(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        frame: Frame,
        viewport: (f32, f32, f32, f32),
    ) {
        if frame.is_empty() {
            return;
        }

        // One scissor per view of a split screen, from where its draws start.
        let clip = |at: usize, grid: [f32; 4]| {
            (
                at,
                crate::hud_views::scissor_rect(grid, viewport, self.space),
            )
        };
        let sprite_count = frame.sprites.len();
        let scissors = |pick: &dyn Fn(&Span) -> usize, base: usize| -> Vec<(usize, [u32; 4])> {
            frame
                .spans
                .iter()
                .map(|span| clip(base + pick(span), span.clip))
                .collect()
        };
        let values = scissors(&|span| span.start[1], sprite_count);
        let mut starts = scissors(&|span| span.start[0], 0);
        starts.extend(values);
        let captions = scissors(&|span| span.start[2], 0);
        let rows = scissors(&|span| span.start[3], 0);
        let skin = scissors(&|span| span.start[4], 0);

        // Sprites first so text sits over the bars, and both in one pass because
        // one renderer holds both the sheet and the value font.
        let mut first = frame.sprites;
        first.extend(frame.hud_text);
        if !first.is_empty() {
            self.values.set_scissors(starts);
            self.values
                .overlay(device, queue, encoder, view, &first, viewport);
        }
        if !frame.small_text.is_empty() {
            self.captions.set_scissors(captions);
            self.captions
                .overlay(device, queue, encoder, view, &frame.small_text, viewport);
        }
        if !frame.default_text.is_empty() {
            self.rows.set_scissors(rows);
            self.rows
                .overlay(device, queue, encoder, view, &frame.default_text, viewport);
        }
        if !frame.skin_text.is_empty() {
            self.skin.set_scissors(skin);
            self.skin
                .overlay(device, queue, encoder, view, &frame.skin_text, viewport);
        }
    }
}
