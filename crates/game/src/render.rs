//! Rasterises a [`Draw`] list with wgpu.
//!
//! Two pipelines and nothing else: one for colour-modulated quads, which covers
//! text, solid fills and the front end's own images, and one for a movie frame
//! out of three 8-bit planes. Everything is positioned in the grid its source's
//! XML places widgets in - the PSP's 480x272, the PS2's 640x448 - and
//! letterboxed into whatever the window is, so those coordinates can be used
//! untouched. See [`Space`], and note that a grid and the aspect it is *shown*
//! as are two different numbers on the PS2.
//!
//! The same code path serves the window and the headless screenshot. That is
//! deliberate: a screenshot that went through a different pipeline would not
//! prove anything about what the window shows.

use anyhow::{Context, Result};

use oag_display::space::{SCREEN, Space};
use oag_ui::font::{self, Atlas};
use oag_ui::frontend::{Align, Draw};

mod backdrop;
mod face;
mod hud_scale;
mod quad;
mod resources;
mod scene;
mod text;
mod video;

use face::{face_atlas_size, face_sampler, upload_face};
use resources::{
    sampler_entry, sampler_kind, texture_entry, ui_bind_group, uniform_entry, upload_rg8,
    upload_rgba,
};

use hud_scale::HudStretch;
use quad::{
    MODE_ATLAS, MODE_BUTTONS_ATLAS, MODE_FACE_ATLAS, MODE_SPRITE, MODE_SPRITE_ADDITIVE, Quad,
    Uniforms,
};
use text::GlyphSlot;
use video::Video;

pub use backdrop::FuryBackdrop;
pub use scene::SceneBackdrop;

/// What a glyph's baked outline is drawn in when nothing supplies a colour.
///
/// Fully transparent, so the outline drops out and only the body draws. See the
/// `Draw::Text` arm in [`Renderer::render_with`] for why this rather than the body
/// colour or an invented black.
const TRANSPARENT: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

/// How many quads the instance buffer holds before it is grown.
const INITIAL_QUADS: usize = 1024;

/// A movie's plane geometry, so textures can be sized once.
#[derive(Debug, Clone, Copy)]
pub struct VideoFormat {
    /// Luma width.
    pub width: u32,
    /// Luma height.
    pub height: u32,
    /// Chroma width.
    pub chroma_width: u32,
    /// Chroma height.
    pub chroma_height: u32,
}

impl VideoFormat {
    /// The plane geometry `movie`'s cached frames need, or `None` when it has
    /// no frames to show.
    ///
    /// Three callers ask the same question - the front end's intro, the menus'
    /// backdrop, and the offscreen capture of either - so it is one function.
    /// `None` covers a source that carries no such movie, `--no-video`, and a
    /// missing `ffmpeg`, and every one of those has to end in a renderer built
    /// **without** the video pipeline: building it and never filling it draws a
    /// green rectangle rather than nothing, the planes being zeroed rather than
    /// absent.
    #[must_use]
    pub fn of(movie: &crate::movie::Movie) -> Option<Self> {
        movie.frames.as_ref().map(|frames| Self {
            width: movie.width,
            height: movie.height,
            chroma_width: frames.chroma_width,
            chroma_height: frames.chroma_height,
        })
    }

    /// The same question, asked of a movie whose frames have moved onto a
    /// decode thread.
    ///
    /// [`crate::movie::Feed::spawn`] consumes the [`crate::movie::FrameStore`],
    /// so [`VideoFormat::of`] cannot answer for a movie that is being played in a
    /// window - `frames` is `None` there and the honest reading of that is "no
    /// picture", which would build a renderer with no video pipeline and draw the
    /// menus on black. The feed carries the four numbers across instead.
    #[must_use]
    pub fn of_feed(feed: &crate::movie::Feed) -> Self {
        Self {
            width: feed.width,
            height: feed.height,
            chroma_width: feed.chroma_width,
            chroma_height: feed.chroma_height,
        }
    }
}

/// The renderer.
pub struct Renderer {
    ui_pipeline: wgpu::RenderPipeline,
    ui_bind_group: wgpu::BindGroup,
    /// What [`Self::ui_bind_group`] was built against, kept so
    /// [`Self::set_sprites`] can rebuild it around a new sheet without
    /// touching the atlas half.
    ui_layout: wgpu::BindGroupLayout,
    atlas_view: wgpu::TextureView,
    atlas_sampler: wgpu::Sampler,
    /// Kept, like [`Self::atlas_view`], so [`Self::set_face_atlas`] can
    /// rebuild [`Self::ui_bind_group`] without losing the sprite sheet.
    sprite_view: wgpu::TextureView,
    sprite_sampler: wgpu::Sampler,
    sprite_format: wgpu::TextureFormat,
    /// The second glyph texture, for `Draw::FacedText` - a 1x1 placeholder
    /// until [`Self::set_face_atlas`] loads a real one; see [`face`].
    face_view: wgpu::TextureView,
    face_sampler: wgpu::Sampler,
    /// `Some` once a role has loaded; see [`Self::push_text`]'s `face`.
    face_atlas: Option<Atlas>,
    /// The role name [`Self::face_atlas`] was loaded for - `"Title"` for
    /// Wipeout HD's chrome, `"Default"` for Pulse's body face loaded beside
    /// its own `menu`-role primary. A `Draw::FacedText` whose own `role`
    /// does not match this (case-insensitively), or a `face_atlas` still
    /// `None`, falls back to [`Self::atlas`] - see [`Self::push_text`]'s
    /// `face` argument.
    face_role: Option<&'static str>,
    /// The third glyph texture, for `Draw::FacedText { role: "Buttons" }` -
    /// a 1x1 placeholder until `Renderer::set_buttons_atlas` loads
    /// `ps_buttons.fnt`/`PS_BUTTONS.fnt`. Reuses `Self::face_sampler`
    /// (identical linear descriptor) rather than a second one - see
    /// `render::resources::ui_bind_group`'s own doc.
    buttons_view: wgpu::TextureView,
    /// `Some` once `Renderer::set_buttons_atlas` has loaded a real atlas.
    /// Unlike `Self::face_atlas`, there is no role name to check and no
    /// fallback to `Self::atlas` when this is `None` - a `Draw::FacedText`
    /// asking for `"Buttons"` with nothing loaded draws **nothing**, per
    /// `CLAUDE.md`'s "never invent" rule: the plain codepoint this would
    /// otherwise fall back to is a Greek letter, not a button glyph. See
    /// `render::text::atlas_for`.
    buttons_atlas: Option<Atlas>,
    /// The role [`Self::buttons_atlas`] answers to: `Buttons`, or `Title` on
    /// a title whose body face holds the other slot. See
    /// [`Renderer::set_buttons_atlas`].
    buttons_role: &'static str,
    /// Which substitute glyph each of the title's button stand-ins draws as
    /// right now; empty draws the disc's own. See [`oag_ui::prompt`].
    prompt_substitution: oag_ui::prompt::Substitution,
    uniform_buffer: wgpu::Buffer,
    quad_buffer: wgpu::Buffer,
    quad_capacity: usize,
    atlas: Atlas,
    /// The sprite sheet's pixel size, which the shader needs to normalise UVs.
    sprites: (u32, u32),
    video: Option<Video>,
    /// The Fury menu backdrop's clouds and passes, once a boot has loaded
    /// them - see [`Self::set_fury_backdrop`]. `None` draws a
    /// `Draw::FuryBackdrop` as nothing, which is the honest absence.
    fury: Option<FuryBackdrop>,
    /// The HD-style scene backdrop - see [`Self::set_scene_backdrop`].
    scene: Option<SceneBackdrop>,
    /// What the pipelines were built for, kept so the backdrop's composite
    /// can be built later against the same target.
    target_format: wgpu::TextureFormat,
    quads: Vec<Quad>,
    /// The grid the draw lists it is given are in, and what that grid is shown
    /// as. [`Space::PSP`] until someone says otherwise, which is what every
    /// caller that draws our own layouts - the loading screen, the menus, the HUD.
    space: Space,
    /// How a raster HUD is stretched - see [`hud_scale`].
    hud: HudStretch,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer")
            .field("quad_capacity", &self.quad_capacity)
            .field("has_video", &self.video.is_some())
            .finish()
    }
}

impl Renderer {
    /// Tells the renderer which grid the draw lists it is given are in.
    ///
    /// Set from the front end's own [`Space`] whenever a stage draws a source's
    /// XML rather than one of our layouts. Nothing calls this for the loading
    /// screen or the menus, whose rects this project authors at 480x272.
    pub fn set_space(&mut self, space: Space) {
        self.space = space;
    }

    /// Builds the pipelines for a target of `format`.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        video: Option<VideoFormat>,
        atlas: Atlas,
        sprites: &oag_hud::sprite::Sheet,
    ) -> Result<Self> {
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oag-game uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let quad_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oag-game quads"),
            size: (INITIAL_QUADS * std::mem::size_of::<Quad>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // The built-in 5x7 glyphs are hard-edged pixel art and smoothing them
        // turns crisp shapes into mush; the disc's fonts are antialiased and
        // drawn at fractional scales, where nearest sampling combs them. So the
        // filter follows the atlas rather than being fixed.
        let filter = if atlas.is_real() {
            wgpu::FilterMode::Linear
        } else {
            wgpu::FilterMode::Nearest
        };
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas"),
            mag_filter: filter,
            min_filter: filter,
            ..Default::default()
        });

        let atlas_texture = upload_rg8(
            device,
            queue,
            "glyph atlas",
            atlas.width,
            atlas.height,
            &atlas.luma,
            &atlas.coverage,
        );
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // The disc's images are authored at screen resolution and drawn at it,
        // so linear filtering here only softens what should be a 1:1 blit. It is
        // linear anyway because nothing guarantees 1:1 once a screen scales an
        // image, and a magnified nearest sample is the worse failure.
        let sprite_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprites"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // **The sheet's format has to follow the target's colour space**, and
        // getting this wrong is not subtle. A `.mip` palette holds sRGB bytes.
        // Declaring the texture sRGB makes sampling return linear, which is
        // right when the target is sRGB and encodes on write - the window - and
        // wrong when it is not: the headless capture targets `Rgba8Unorm` on
        // purpose, so linear values would go into the PNG unencoded. Measured
        // rather than reasoned: the logo's dominant teal is `(36, 147, 153)` in
        // the texture and came out `(5, 74, 81)` in a capture, which is that
        // colour linearised exactly once.
        //
        // Text and fills are not affected either way - they write their colour
        // straight through with no sRGB source - so this is the only draw kind
        // that can differ between the window and a screenshot.
        let sprite_format = if format.is_srgb() {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        };

        let sprite_texture = upload_rgba(
            device,
            queue,
            "sprite sheet",
            sprite_format,
            sprites.width,
            sprites.height,
            &sprites.rgba,
        );
        let sprite_view = sprite_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // No title has loaded a role yet at construction - see
        // `Self::set_face_atlas`, which every menu-drawing caller reaches
        // once it knows whether this title named one. Always linear-filtered
        // and always bound, the same way `sprite_sampler` is, so loading a
        // real face later needs no layout change.
        let face_view = upload_face(device, queue, None);
        let face_sampler = face_sampler(device);
        // The buttons atlas, uploaded through the same helper `face_view`
        // is - it is the identical shape (a two-channel glyph texture, 1x1
        // placeholder until a title's boot loads a real one) - see
        // `Renderer::buttons_view`'s own doc for why this is a separate
        // texture rather than a second use of `face_view`.
        let buttons_view = upload_face(device, queue, None);

        let ui_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui"),
            entries: &[
                uniform_entry(0),
                texture_entry(1),
                sampler_entry(2, sampler_kind(atlas.is_real())),
                texture_entry(3),
                sampler_entry(4, wgpu::SamplerBindingType::Filtering),
                texture_entry(5),
                sampler_entry(6, wgpu::SamplerBindingType::Filtering),
                texture_entry(7),
                sampler_entry(8, wgpu::SamplerBindingType::Filtering),
            ],
        });

        let ui_bind_group = ui_bind_group(
            device,
            &ui_layout,
            &uniform_buffer,
            &atlas_view,
            &atlas_sampler,
            &sprite_view,
            &sprite_sampler,
            &face_view,
            &face_sampler,
            &buttons_view,
        );

        let ui_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/ui.wgsl")).into(),
            ),
        });

        let ui_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui"),
            bind_group_layouts: &[Some(&ui_layout)],
            immediate_size: 0,
        });

        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui"),
            layout: Some(&ui_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ui_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Quad>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x4,
                        1 => Float32x4,
                        2 => Float32x4,
                        3 => Float32x4,
                        4 => Float32,
                        5 => Float32,
                        6 => Float32x2,
                        7 => Float32x2
                    ],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ui_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // **Premultiplied, and it composites identically to the
                    // `ALPHA_BLENDING` this used to be.** The two differ in
                    // exactly one factor - the colour source, `One` here
                    // against `SrcAlpha` there - and `fs_main` multiplies the
                    // colour by its own alpha before returning it, cancelling
                    // the difference. The alpha component is
                    // `BlendComponent::OVER` in both, so `render_with(
                    // LoadOp::Load, ..)` does not move; the pause-overlay test
                    // below asserts that directly. What it buys is a
                    // **per-quad** additive blend without a second pipeline or
                    // a split pass - a quad emitting an alpha of zero leaves
                    // the destination factor at one - which is what the three
                    // lock-on sight models' own `pass_mask` asks for. See
                    // [`MODE_SPRITE_ADDITIVE`] and [`Draw::BlendedSprite`].
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let video = video
            .map(|format_info| Video::new(device, &uniform_buffer, format, format_info))
            .transpose()?;

        Ok(Self {
            ui_pipeline,
            ui_bind_group,
            ui_layout,
            atlas_view,
            atlas_sampler,
            sprite_view,
            sprite_sampler,
            sprite_format,
            face_view,
            face_sampler,
            face_atlas: None,
            face_role: None,
            buttons_view,
            buttons_atlas: None,
            buttons_role: oag_ui::language::roles::BUTTONS,
            prompt_substitution: oag_ui::prompt::Substitution::none(),
            uniform_buffer,
            quad_buffer,
            quad_capacity: INITIAL_QUADS,
            atlas,
            sprites: (sprites.width, sprites.height),
            video,
            fury: None,
            scene: None,
            target_format: format,
            quads: Vec::new(),
            space: Space::PSP,
            hud: HudStretch::default(),
        })
    }

    /// Replaces the sprite sheet every later draw samples.
    ///
    /// What the selection screens need: their stills are per circuit, read
    /// when a circuit is selected rather than at boot, and a sheet is one
    /// texture. The caller hands over a sheet that **extends** the one this
    /// was built with (`oag_hud::sprite::Sheet::extended`), so every placement
    /// the menus already hold stays where it was and nothing has to be
    /// restored when the screen closes. One texture upload and one bind
    /// group; the pipeline, the atlas and the uniforms are untouched.
    pub fn set_sprites(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sprites: &oag_hud::sprite::Sheet,
    ) {
        let sprite_texture = upload_rgba(
            device,
            queue,
            "sprite sheet",
            self.sprite_format,
            sprites.width,
            sprites.height,
            &sprites.rgba,
        );
        self.sprite_view = sprite_texture.create_view(&wgpu::TextureViewDescriptor::default());
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
        self.sprites = (sprites.width, sprites.height);
    }

    /// The plane geometry the video pipeline was built for.
    #[must_use]
    pub fn video_format(&self) -> Option<VideoFormat> {
        self.video.as_ref().map(|v| v.format)
    }

    /// Uploads one decoded frame as three consecutive planes.
    ///
    /// Only [`crate::movie::PixelFormat::I420`] is implemented today; anything
    /// else is rejected rather than silently misread.
    pub fn upload_frame(
        &self,
        queue: &wgpu::Queue,
        frame: &crate::movie::VideoFrame,
    ) -> Result<()> {
        let video = self.video.as_ref().context("no video pipeline")?;
        anyhow::ensure!(
            frame.format == crate::movie::PixelFormat::I420,
            "upload_frame only knows I420 today, got {:?}",
            frame.format
        );
        let luma = (video.format.width * video.format.height) as usize;
        let chroma = (video.format.chroma_width * video.format.chroma_height) as usize;

        let planes = [
            (0usize, 0..luma, video.format.width, video.format.height),
            (
                1,
                luma..luma + chroma,
                video.format.chroma_width,
                video.format.chroma_height,
            ),
            (
                2,
                luma + chroma..luma + 2 * chroma,
                video.format.chroma_width,
                video.format.chroma_height,
            ),
        ];

        for (index, range, width, height) in planes {
            let bytes = frame
                .bytes
                .get(range)
                .context("frame is shorter than its declared planes")?;
            queue.write_texture(
                video.planes[index].as_image_copy(),
                bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        Ok(())
    }

    /// Draws `list` into `view`, clearing it first.
    ///
    /// What a stage does: it owns the frame, so it starts from black and the
    /// bars outside `viewport` are what it leaves uncovered. `clip` is
    /// `(index, left, right)` for a marquee row - see `oag_ui_screens::marquee`.
    #[expect(clippy::too_many_arguments, reason = "clip is one more fact")]
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        list: &[Draw],
        viewport: (f32, f32, f32, f32),
        clip: Option<(usize, f32, f32)>,
    ) {
        self.render_with(
            wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            device,
            queue,
            encoder,
            view,
            list,
            viewport,
            clip,
        );
    }

    /// Draws `list` **over** whatever is already in `view`.
    ///
    /// What the performance overlay does: it is not a stage and does not own
    /// the frame, so it has to keep what the stage drew. That is the only
    /// difference - a second pass with the same pipelines, which is also why
    /// the overlay needs a renderer of its own rather than a flag on the
    /// stage's: a race draws through [`oag_raceplay::Scene`] and has no
    /// [`Renderer`] at all.
    pub fn overlay(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        list: &[Draw],
        viewport: (f32, f32, f32, f32),
    ) {
        self.render_with(
            wgpu::LoadOp::Load,
            device,
            queue,
            encoder,
            view,
            list,
            viewport,
            None, // the overlay never marquees; only a menu row does
        );
    }

    /// `pub` rather than private, and for the same reason [`Self::render`]
    /// and [`Self::overlay`] already are: the composition root's own
    /// `menu_stage` module is a different crate (the `[[bin]]` over this
    /// `[lib]`), and it draws its own rows over a parked race's
    /// already-resolved picture exactly this way - `LoadOp::Load` with the
    /// menu's own marquee `clip` still honoured, which neither
    /// [`Self::render`] nor [`Self::overlay`] alone offers.
    #[expect(clippy::too_many_arguments, reason = "one wrapper's worth of load op")]
    pub fn render_with(
        &mut self,
        load: wgpu::LoadOp<wgpu::Color>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        list: &[Draw],
        viewport: (f32, f32, f32, f32),
        clip: Option<(usize, f32, f32)>,
    ) {
        self.quads.clear();
        self.hud.pixels_per_grid = HudStretch::measure(self.space, viewport);
        // Where the movie sits in the quad order. The list is painted back to
        // front, and the `LogoFMV` screen puts a black `Image` *behind* its
        // `Movie`, so drawing the movie first unconditionally would let that
        // black backdrop paint straight over the picture.
        let mut video_at = None;
        // Where the movie sits in screen space - not always all of it, since a
        // PS2 `.PSS`'s own display aspect pillarboxes inside [`SCREEN`] rather
        // than filling it the way a `.PMF` always has. See
        // `oag_display::space::pillarbox`.
        let mut video_rect = [0.0, 0.0, self.space.size.0, self.space.size.1];
        // Where the Fury backdrop sits, the same way: its passes run before
        // this one and its composite is drawn at its place in the order.
        let mut fury_at = None;
        let mut scene_at = None;
        for (index, draw) in list.iter().enumerate() {
            match draw {
                Draw::FuryBackdrop(frame) => {
                    fury_at = Some((self.quads.len() as u32, frame.as_ref()));
                }
                Draw::SceneBackdrop(frame) => {
                    scene_at = Some((self.quads.len() as u32, frame.as_ref()));
                }
                Draw::Fill { rect, color } => self.push_solid(*rect, *color, [0.0, 0.0]),
                Draw::ChamferedFill {
                    rect,
                    chamfer,
                    color,
                } => self.push_solid(*rect, *color, *chamfer),
                Draw::GradientFill { rect, left, right } => {
                    self.push_gradient(*rect, *left, *right)
                }
                Draw::Video { rect, .. } => {
                    video_at = Some(self.quads.len() as u32);
                    video_rect = *rect;
                }
                Draw::Sprite { rect, uv, color } => self.quads.push(Quad {
                    rect: *rect,
                    uv: *uv,
                    color: *color,
                    // A sprite is sampled as RGBA and never mixed, so this is
                    // inert here. It still has to be a real value.
                    border: *color,
                    mode: MODE_SPRITE,
                    rotation: 0.0,
                    chamfer: [0.0, 0.0],
                    tile: [0.0, 0.0],
                }),
                Draw::TiledSprite {
                    rect,
                    uv,
                    repeat,
                    color,
                } => self.quads.push(Quad {
                    rect: *rect,
                    uv: *uv,
                    color: *color,
                    border: *color,
                    mode: MODE_SPRITE,
                    rotation: 0.0,
                    chamfer: [0.0, 0.0],
                    tile: *repeat,
                }),
                Draw::RotatedSprite {
                    rect,
                    uv,
                    color,
                    rotation,
                } => self.quads.push(Quad {
                    rect: *rect,
                    uv: *uv,
                    color: *color,
                    border: *color,
                    mode: MODE_SPRITE,
                    rotation: *rotation,
                    chamfer: [0.0, 0.0],
                    tile: [0.0, 0.0],
                }),
                Draw::BlendedSprite {
                    rect,
                    uv,
                    color,
                    rotation,
                    blend,
                } => self.quads.push(Quad {
                    rect: *rect,
                    uv: *uv,
                    color: *color,
                    border: *color,
                    // **The model's own class, mapped and not chosen.**
                    // `Additive` is the one this pipeline expresses as
                    // something other than an over-blend; `AlphaOver` *is* the
                    // over-blend, and the `0x400` class has no equivalent for a
                    // 2D overlay quad and is declared by nothing measured
                    // reaching here, so it draws as it did rather than being
                    // invented an equation.
                    mode: match blend {
                        Some(oag_vex::vex::BlendClass::Additive) => MODE_SPRITE_ADDITIVE,
                        _ => MODE_SPRITE,
                    },
                    rotation: *rotation,
                    chamfer: [0.0, 0.0],
                    tile: [0.0, 0.0],
                }),
                Draw::Text {
                    x,
                    y,
                    scale,
                    color,
                    border,
                    align,
                    text,
                    wrap_width,
                } => {
                    // Transparent is a no-op for the menu/built-in fonts' constant
                    // mask, and the picked default for the HUD fonts. See
                    // `docs/ui/hud.md` for the fuller reasoning and open question.
                    let border = border.unwrap_or(TRANSPARENT);
                    // By index, not `y` - a label shares its row's `y`.
                    let bounds = clip.filter(|(at, ..)| *at == index).map(|(_, l, r)| (l, r));
                    match wrap_width {
                        Some(width) => {
                            self.push_wrapped_text(
                                GlyphSlot::Primary,
                                *x,
                                *y,
                                *scale,
                                *color,
                                border,
                                *align,
                                text,
                                *width,
                                bounds,
                            );
                        }
                        None => self.push_text(
                            GlyphSlot::Primary,
                            *x,
                            *y,
                            *scale,
                            *color,
                            border,
                            *align,
                            text,
                            bounds,
                        ),
                    }
                }
                Draw::FacedText {
                    // Resolved against `Self::face_role`/`Self::buttons_atlas`
                    // below - a `"buttons"` role always reads
                    // `Self::buttons_atlas` and never falls back
                    // ([`GlyphSlot::Buttons`]'s own doc); every other role
                    // falls back to the primary atlas the way it always has,
                    // whether or not it matches `Self::face_role`. See
                    // `Self::face_role`'s own doc.
                    role,
                    x,
                    y,
                    scale,
                    color,
                    border,
                    align,
                    text,
                    wrap_width,
                } => {
                    let border = border.unwrap_or(TRANSPARENT);
                    let bounds = clip.filter(|(at, ..)| *at == index).map(|(_, l, r)| (l, r));
                    let slot = if role.eq_ignore_ascii_case(oag_ui::language::roles::BUTTONS)
                        || (self.buttons_atlas.is_some()
                            && role.eq_ignore_ascii_case(self.buttons_role))
                    {
                        GlyphSlot::Buttons
                    } else if self.face_atlas.is_some()
                        && self
                            .face_role
                            .is_some_and(|loaded| loaded.eq_ignore_ascii_case(role))
                    {
                        GlyphSlot::Face
                    } else {
                        GlyphSlot::Primary
                    };
                    match wrap_width {
                        Some(width) => {
                            self.push_wrapped_text(
                                slot, *x, *y, *scale, *color, border, *align, text, *width, bounds,
                            );
                        }
                        None => self
                            .push_text(slot, *x, *y, *scale, *color, border, *align, text, bounds),
                    }
                }
            }
        }

        if self.hud.mode.snaps_geometry() {
            hud_scale::snap_sprites(&mut self.quads, self.hud.pixels_per_grid);
        }
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&Uniforms {
                // Fitted to the *viewport rectangle*, not to the surface. The
                // two are the same thing at `Aspect::Psp` on a PSP-shaped
                // window and are not otherwise, and feeding the surface here
                // would letterbox the widgets a second time inside the bars
                // `set_viewport` already left.
                viewport: letterbox_in(
                    (viewport.2 as u32, viewport.3 as u32),
                    self.space.display_aspect,
                ),
                // The **grid**, not the display aspect: this maps a draw's rect
                // onto the viewport, and the rects are in grid coordinates.
                // `letterbox_in` above is the one that wants the other number.
                // See `oag_display::space::Space`.
                screen: [self.space.size.0, self.space.size.1],
                atlas: [self.atlas.width as f32, self.atlas.height as f32],
                sprites: [self.sprites.0 as f32, self.sprites.1 as f32],
                video_rect,
                face_atlas: face_atlas_size(self.face_atlas.as_ref()),
                buttons_atlas: face_atlas_size(self.buttons_atlas.as_ref()),
                hud: self.hud.uniform(),
            }),
        );

        if self.quads.len() > self.quad_capacity {
            self.quad_capacity = self.quads.len().next_power_of_two();
            self.quad_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("oag-game quads"),
                size: (self.quad_capacity * std::mem::size_of::<Quad>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        self.prepare_backdrops(
            (device, queue, encoder),
            (
                fury_at.map(|(_, frame)| frame),
                scene_at.map(|(_, frame)| frame),
            ),
            (viewport.2 as u32, viewport.3 as u32),
        );
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("frame"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        // A clearing pass clears the whole surface to black, so whatever the
        // rectangle does not cover stays black - which is what the bars are.
        pass.set_viewport(viewport.0, viewport.1, viewport.2, viewport.3, 0.0, 1.0);

        let total = self.quads.len() as u32;
        if !self.quads.is_empty() {
            queue.write_buffer(&self.quad_buffer, 0, bytemuck::cast_slice(&self.quads));
        }

        // Quads behind a picture, the picture, then the quads in front of it -
        // for each of the two pictures a list can carry, in the order the
        // list puts them, so the list's own order is honoured.
        let mut cuts: Vec<(u32, backdrop::Cut)> = Vec::new();
        cuts.extend(video_at.map(|at| (at, backdrop::Cut::Video)));
        cuts.extend(fury_at.map(|(at, _)| (at, backdrop::Cut::Fury)));
        cuts.extend(scene_at.map(|(at, _)| (at, backdrop::Cut::Scene)));
        cuts.sort_unstable();
        let mut drawn = 0;
        let draw_quads = |pass: &mut wgpu::RenderPass<'_>, range: std::ops::Range<u32>| {
            if range.start < range.end {
                pass.set_pipeline(&self.ui_pipeline);
                pass.set_bind_group(0, &self.ui_bind_group, &[]);
                pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
                pass.draw(0..6, range);
            }
        };
        for (at, cut) in cuts {
            draw_quads(&mut pass, drawn..at);
            drawn = at;
            match (cut, &self.video, &self.fury, &fury_at) {
                (backdrop::Cut::Video, Some(video), ..) => {
                    pass.set_pipeline(&video.pipeline);
                    pass.set_bind_group(0, &video.bind_group, &[]);
                    pass.draw(0..6, 0..1);
                }
                (backdrop::Cut::Fury, _, Some(fury), Some((_, frame))) => {
                    fury.composite(&mut pass, frame.tint);
                }
                (backdrop::Cut::Scene, ..) => {
                    if let Some(scene) = &self.scene {
                        scene.composite(&mut pass);
                    }
                }
                _ => {}
            }
        }
        // A picture last in the list - `Show Logo`'s movie with nothing over
        // it - is drawn by the loop above, not skipped by a cut at `total`.
        draw_quads(&mut pass, drawn..total);
    }
}

/// Scale that fits 480x272 inside `target` without distorting it.
#[must_use]
pub fn letterbox(target: (u32, u32)) -> [f32; 2] {
    letterbox_in(target, SCREEN.0 / SCREEN.1)
}

/// [`letterbox`], for a screen that is not the PSP's shape.
///
/// `screen_aspect` is a **display** aspect, not a grid one - the PS2's 640x448
/// grid is shown at the PSP's own 480/272, and fitting 640/448 into the window
/// instead would squeeze the whole front end. See `oag_display::space::Space`.
#[must_use]
pub fn letterbox_in(target: (u32, u32), screen_aspect: f32) -> [f32; 2] {
    let (width, height) = (target.0.max(1) as f32, target.1.max(1) as f32);
    let target_aspect = width / height;
    if target_aspect > screen_aspect {
        [screen_aspect / target_aspect, 1.0]
    } else {
        [1.0, target_aspect / screen_aspect]
    }
}

/// A window position, in the grid a stage's `Draw`s are authored in.
///
/// [`letterbox_in`] and `ui.wesl`'s `to_clip` run backwards: `viewport` is
/// the aspect rectangle every stage draws into (`oag_display::display::viewport`,
/// in physical pixels), and `space` is the grid and display aspect the
/// stage's list is fitted with. What comes back is what a `Draw` at that
/// point would have as its `rect` origin, so a screen can hit-test a click
/// against the rects it just emitted - see `oag_ui::pointer`.
///
/// **The letterbox is undone, not only the viewport**, and that is the
/// whole reason this is a function rather than two divisions. A title whose
/// display aspect is not the viewport's is fitted inside it with bars, and a
/// click in those bars maps to a grid coordinate outside `space.size` - as it
/// should, since nothing is drawn there. `MenuStage`'s `overlay_rect` is the
/// same cancellation in the other direction.
///
/// A point outside the viewport is still mapped rather than refused: a
/// pointer that has left the picture reads as a grid coordinate off the
/// grid, and every hit test already answers `None` for one of those.
#[must_use]
pub fn to_grid(space: Space, viewport: (f32, f32, f32, f32), window: (f32, f32)) -> (f32, f32) {
    let (left, top, width, height) = viewport;
    let [scale_x, scale_y] = letterbox_in((width as u32, height as u32), space.display_aspect);
    // Physical pixel to clip space, `set_viewport`'s own mapping: `-1` at the
    // viewport's left and top edges, except that clip y runs upward.
    let clip_x = (window.0 - left) / width.max(1.0) * 2.0 - 1.0;
    let clip_y = 1.0 - (window.1 - top) / height.max(1.0) * 2.0;
    // `to_clip` multiplied by the letterbox scale last; divide it out first.
    let normalised_x = (clip_x / scale_x + 1.0) * 0.5;
    let normalised_y = (1.0 - clip_y / scale_y) * 0.5;
    (normalised_x * space.size.0, normalised_y * space.size.1)
}

#[cfg(test)]
mod prompt_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod composite_tests;
