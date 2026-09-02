//! Rendering at a resolution the window is not, and blitting the result up.
//!
//! Every stage draws into an offscreen colour texture rather than straight onto
//! the surface, and [`Framebuffer::resolve_scene`] afterwards stretches that
//! texture into the [`display::viewport`](crate::display::viewport) rectangle.
//! Below 100 % that is the usual internal-resolution knob; above it, it is
//! supersampling.
//!
//! It stretches it into a **presentation-sized target**, not onto the surface:
//! the UI composites there, at the size a player actually has, and
//! [`Framebuffer::composite`] then writes the surface. Two passes rather than
//! one, and one more full-size texture, which
//! [ADR-0036](../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md)
//! is the reasoning for.
//!
//! # Why a scale and not a resolution
//!
//! A percentage has no invalid values. An absolute render resolution has to be
//! validated against the window every time either changes, and a settings file
//! left holding `3840x2160` on a 1080p window is a state somebody has to define
//! behaviour for. `display.window_size` is a window property and absolute; this
//! is a graphics one and relative, and keeping them different kinds is what
//! stops them being conflated - and is why they sit in different tables and on
//! different menu pages.
//!
//! # What it is measured against
//!
//! The **viewport rectangle**, not the surface. So the scale means the same
//! thing whatever [`Aspect`](crate::display::Aspect) is set to - at 50 % on a
//! pillarboxed 4:3 window, the game is drawn at half the 4:3 area rather than
//! half a window it was never using. The bars are drawn by the blit pass
//! clearing the surface, so they cost no offscreen pixels at all.
//!
//! # Why brightness and gamma are here
//!
//! Because [`Framebuffer::composite`] is the one pass every frame goes through,
//! and the last one. The front end, the menus and a race are three renderers
//! that share nothing else, and grading in each would be three places to get it
//! wrong and three places to forget when a fourth stage lands. Here it is one
//! shader, and a player calibrating the picture sees the menu they are standing
//! on change as they do it.
//!
//! **Being last is the load-bearing half of that**, and is why the grade sits
//! in `composite` rather than in `resolve_scene`: the HUD is drawn between the
//! two, so grading on the way in would grade the scene and leave the HUD
//! outside the calibration. The performance overlay, drawn onto the surface
//! after `composite`, is deliberately outside it - see `crate::perf`.
//!
//! **A screenshot is not graded, by default.** `--screenshot` and the race
//! capture write the offscreen frame straight out without going through this
//! pass, and that is the wanted answer rather than an oversight: brightness and
//! gamma are a setting about somebody's monitor, and baking them into a PNG
//! that goes into a bug report would make every capture disagree with every
//! other one.
//!
//! `--presented` opts out of that and puts the whole path in the way - the
//! render scale, the upscaler, the grade and the bars. It exists because the
//! ordinary capture never reaches this pass at all, so without it there is no
//! way to *see* what an upscaler did, and therefore no way to choose one. A
//! comparison wants a picture of a window; a bug report does not.

use anyhow::{Context, Result};
use log::warn;

use oag_render::post::{fsr1, fullscreen_layout, fxaa, smaa};

use crate::display::{AntiAliasing, Brightness, Gamma, Scale, Upscaler};

/// Everything the blit needs that a player chose, gathered so the window's
/// frame loop and a capture can be handed the same thing.
#[derive(Debug, Clone, Copy)]
pub struct Presentation {
    /// Which resampler carries the frame onto the surface.
    pub upscaler: Upscaler,
    /// FSR 1's RCAS sharpness in stops. Ignored by the bilinear path.
    pub sharpness: f32,
    /// FXAA or SMAA, run before the upscaler - see [`Framebuffer::resolve_scene`]
    /// and [ADR-0013](../../../docs/architecture/adr/0013-anti-aliasing-architecture.md).
    /// MSAA is not read here: its sample count is baked into the scene's own
    /// pipelines rather than being a blit-time choice - see `race::Scene`.
    pub anti_aliasing: AntiAliasing,
    pub brightness: Brightness,
    pub gamma: Gamma,
}

/// What the blit does to the picture on its way onto the surface, as the
/// shader's uniform expects it.
///
/// `repr(C)` and sixteen bytes: a uniform binding has a minimum size, and the
/// two floats alone are half of it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Grade {
    brightness: f32,
    exponent: f32,
    /// Whether the blit's source holds sRGB-encoded values the shader has to
    /// decode itself, as `1.0` or `0.0`.
    ///
    /// One only when `self.format.is_srgb()` - the offscreen target is bound
    /// through an sRGB view and arrives already decoded by the sampler, while
    /// an upscaler's or FXAA/SMAA's output is deliberately read through the
    /// *non*-sRGB twin because that pass works in perceptual space and a
    /// decode on every internal read would be both wrong and paid twice. See
    /// `oag_render::post`. Since [ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md)
    /// forced every real `format` here non-sRGB, `twin == format` and this is
    /// always zero in production - "a post-process ran" is not the same fact
    /// as "the source needs decoding", and treating them as one is the bug
    /// that made FXAA, SMAA and a magnifying FSR 1 darken the picture.
    decode: f32,
    padding: f32,
}

impl Grade {
    fn new(brightness: Brightness, gamma: Gamma, decode: bool) -> Self {
        Self {
            brightness: brightness.factor(),
            exponent: gamma.exponent(),
            decode: f32::from(u8::from(decode)),
            padding: 0.0,
        }
    }
}

/// The offscreen target and the pipeline that puts it on screen.
pub struct Framebuffer {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    /// The same texels as [`Framebuffer::view`], seen through a non-sRGB view.
    ///
    /// Every stage draws through `view` and so keeps encoding on write exactly
    /// as it would onto a window. An upscaler reads through this one instead,
    /// which returns those encoded bytes without the hardware's decode -
    /// perceptual space, which is what FSR 1's edge detection wants. See
    /// [`oag_render::post`].
    perceptual: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    /// The grade the shader reads, and the copy of it that says whether a write
    /// is needed. Uploaded only when it changes: this is two floats a player
    /// moves from a menu, not per-frame data.
    grade: wgpu::Buffer,
    graded: Grade,
    /// FSR 1's two pipelines, built the first frame the setting asks for them.
    ///
    /// Lazy because most runs never select it, and the pipelines are two shader
    /// compilations and two textures a bilinear blit has no use for. The
    /// `Result` is kept rather than unwrapped so a shader that will not compile
    /// reports itself once and leaves the game running on the blit.
    fsr1: Option<Result<fsr1::Fsr1>>,
    /// FXAA's one pipeline, built the first frame `[graphics] anti_aliasing`
    /// asks for it. Lazy for the same reason `fsr1` is.
    fxaa: Option<Result<fxaa::Fxaa>>,
    /// SMAA's three pipelines and its two lookup textures, built the first
    /// frame `[graphics] anti_aliasing` asks for them. Lazy for the same
    /// reason `fsr1` is.
    smaa: Option<Result<smaa::Smaa>>,
    size: (u32, u32),
    format: wgpu::TextureFormat,
    /// Where the scene lands at presentation size, and where the UI composites
    /// on top of it before one graded pass writes the surface. See [`Output`]
    /// and [ADR-0036](../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md).
    output: Output,
}

/// The presentation-sized target, between the upscale and the surface.
///
/// It exists so the UI can be drawn at presentation resolution **and** still be
/// graded. The two are in tension without it: the grade rides in the pass that
/// puts the frame on the surface, so a UI drawn onto the surface after that
/// pass is sharp and ungraded, and a UI drawn before it is graded and
/// resampled. With a target in between, the scene resolves into it ungraded,
/// the UI draws into it at its own size, and the grade goes on the way out.
///
/// It costs one more colour texture at the surface's size and one more
/// fullscreen pass a frame, which
/// [ADR-0036](../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md)
/// states rather than hides.
///
/// Its own grade buffer, separate from [`Framebuffer::grade`], because the two
/// passes want different values in the same frame: neutral on the way in - the
/// scene must not be graded twice - and the player's brightness and gamma on
/// the way out. One buffer would have to hold both at once.
struct Output {
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    grade: wgpu::Buffer,
    graded: Grade,
    size: (u32, u32),
}

impl Framebuffer {
    /// Builds the blit pipeline and a target of `size`.
    ///
    /// # Errors
    ///
    /// Propagates a shader that will not compile, which is a build-time mistake
    /// rather than anything a player can cause.
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("upscale"),
            source: wgpu::ShaderSource::Wgsl(include_str!("upscale.wgsl").into()),
        });

        let layout = fullscreen_layout(device, "upscale");

        // Linear both ways. Upscaling a low internal resolution with nearest
        // sampling gives hard pixel edges, which is a look this project has not
        // chosen; downscaling with it aliases outright and would make
        // supersampling worse than not doing it.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("upscale"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("upscale"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("upscale"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
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

        // Neutral to begin with, and moved by `set_grade` from the first frame
        // if the settings say otherwise: an untouched picture is what a fresh
        // install draws.
        let graded = Grade::new(Brightness::NEUTRAL, Gamma::NEUTRAL, false);
        let grade = grade_buffer(device, "upscale grade", graded)?;

        let (texture, view, perceptual, bind_group) =
            target(device, &layout, &sampler, &grade, format, size);
        // Built at the scene's size to begin with and corrected by
        // `resize_output` on the first frame, exactly as the scene target
        // itself is by `resize`: this constructor is handed the size the scene
        // draws at and has no way to know the surface's.
        let output = output(device, &layout, &sampler, format, size)?;
        Ok(Self {
            pipeline,
            layout,
            sampler,
            texture,
            view,
            perceptual,
            bind_group,
            grade,
            graded,
            fsr1: None,
            fxaa: None,
            smaa: None,
            size,
            format,
            output,
        })
    }

    /// Makes sure the shader is grading with `brightness` and `gamma`.
    ///
    /// Written only when it changes, which is when a player moves one of two
    /// menu rows. Cheap enough to call every frame, which is what the caller
    /// does - it has no other way to know the settings moved.
    pub fn set_grade(
        &mut self,
        queue: &wgpu::Queue,
        brightness: Brightness,
        gamma: Gamma,
        decode: bool,
    ) {
        let wanted = Grade::new(brightness, gamma, decode);
        if wanted == self.graded {
            return;
        }
        queue.write_buffer(&self.grade, 0, bytemuck::bytes_of(&wanted));
        self.graded = wanted;
    }

    /// Runs the chosen upscaler and blits the result onto `surface`.
    ///
    /// The whole back half of a frame in one call, so that the window's frame
    /// loop and a capture that wants to see what the window sees cannot drift
    /// apart - which they would, because the ordering here has two traps in it.
    /// The grade's `decode` flag has to describe the source that is *actually*
    /// bound rather than the setting that asked for it, **and** whether that
    /// source is genuinely sRGB-encoded rather than merely "produced by a
    /// pass" - see the field's own doc - and the upscaler has to run after
    /// every stage has finished drawing but before the blit.
    ///
    /// **An upscaler only runs when it is actually upscaling** - see
    /// [`magnifies`].
    ///
    /// **FXAA/SMAA run before the upscaler, not after**, reading `self.perceptual`
    /// at the scene's own size and handing their output on as what the
    /// upscaler reads instead - see [ADR-0013](../../../docs/architecture/adr/0013-anti-aliasing-architecture.md)
    /// for why the order is the reverse of a generic post-process-AA diagram.
    ///
    /// This puts the frame in [`Framebuffer::output`], **not** on the surface,
    /// and it does so *ungraded* - the caller draws the UI on top of it and
    /// then calls [`Framebuffer::composite`], which is where the grade is and
    /// where the surface is written. See
    /// [ADR-0036](../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md).
    pub fn resolve_scene(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        rect: (f32, f32, f32, f32),
        presentation: &Presentation,
    ) {
        let output_size = (rect.2 as u32, rect.3 as u32);

        let post_process: Option<&wgpu::TextureView> = match presentation.anti_aliasing {
            AntiAliasing::Fxaa => {
                let fxaa = self
                    .fxaa
                    .get_or_insert_with(|| fxaa::Fxaa::new(device, self.format));
                match fxaa {
                    Ok(fxaa) => {
                        fxaa.render(
                            device,
                            queue,
                            encoder,
                            fxaa::Frame {
                                source: &self.perceptual,
                                size: self.size,
                            },
                        );
                        fxaa.output()
                    }
                    // A shader that will not compile is a build-time mistake,
                    // but it must not be a crash in a player's frame loop: say
                    // so once and carry on unfiltered.
                    Err(why) => {
                        warn!("the FXAA pipeline did not build ({why:#}); staying unfiltered");
                        None
                    }
                }
            }
            AntiAliasing::Smaa => {
                let smaa = self
                    .smaa
                    .get_or_insert_with(|| smaa::Smaa::new(device, queue, self.format));
                match smaa {
                    Ok(smaa) => {
                        smaa.render(
                            device,
                            queue,
                            encoder,
                            smaa::Frame {
                                source: &self.perceptual,
                                size: self.size,
                            },
                        );
                        smaa.output()
                    }
                    // A shader that will not compile is a build-time mistake,
                    // but it must not be a crash in a player's frame loop: say
                    // so once and carry on unfiltered.
                    Err(why) => {
                        warn!("the SMAA pipelines did not build ({why:#}); staying unfiltered");
                        None
                    }
                }
            }
            AntiAliasing::Off | AntiAliasing::Msaa4x => None,
        };
        let upscale_source = post_process.unwrap_or(&self.perceptual);

        let source = (presentation.upscaler == Upscaler::Fsr1 && magnifies(self.size, output_size))
            .then(|| {
                let fsr = self
                    .fsr1
                    .get_or_insert_with(|| fsr1::Fsr1::new(device, self.format));
                let fsr = match fsr {
                    Ok(fsr) => fsr,
                    // A shader that will not compile is a build-time mistake, but
                    // it must not be a crash in a player's frame loop: say so once
                    // and carry on bilinear.
                    Err(why) => {
                        warn!("the FSR 1 pipelines did not build ({why:#}); staying bilinear");
                        return None;
                    }
                };
                fsr.render(
                    device,
                    queue,
                    encoder,
                    fsr1::Frame {
                        source: upscale_source,
                        input: self.size,
                        output: output_size,
                        sharpness: fsr1::Sharpness::stops(presentation.sharpness),
                    },
                );
                fsr.output()
                    .map(|view| bind(device, &self.layout, &self.sampler, &self.grade, view))
            })
            .flatten()
            // The upscaler did not run - off, not magnifying, or its own
            // pipelines failed to build - but FXAA/SMAA already produced a
            // frame in the same non-sRGB space an upscaler's output would be:
            // bind that directly rather than falling back to the untouched
            // target.
            .or_else(|| {
                post_process
                    .map(|view| bind(device, &self.layout, &self.sampler, &self.grade, view))
            });

        // `source.is_some()` alone says a post-process bound a view in the
        // *non-sRGB* twin format, not that a decode is owed for it: since
        // ADR-0020, `self.format` is never sRGB in any real call site (the
        // window surface and every capture target are forced non-sRGB), so
        // the twin and the ordinary view hold identical bytes and there is
        // nothing to decode. Gating on `self.format.is_srgb()` too is what
        // keeps this flag tracking the format's actual encoding rather than
        // "did a pass run" - the two agreed by accident before that ADR and
        // silently stopped afterwards, which is what made FXAA, SMAA and a
        // magnifying FSR 1 decode already-linear-in-name gamma values as if
        // they were sRGB-encoded, crushing the shadows and midtones.
        //
        // **Neutral**, not the player's brightness and gamma: this pass writes
        // the presentation target, and the UI has not been drawn yet. Grading
        // here would grade the scene and leave the HUD and the menus outside
        // the calibration, which is the arrangement ADR-0036 exists to end. The
        // `decode` flag still belongs here, being about what *this* pass reads.
        self.set_grade(
            queue,
            Brightness::NEUTRAL,
            Gamma::NEUTRAL,
            source.is_some() && self.format.is_srgb(),
        );
        self.present(encoder, &self.output.view, rect, source.as_ref());
    }

    /// Makes sure the presentation target is `size` - the **surface's** size,
    /// not the scene's.
    ///
    /// Separate from [`Framebuffer::resize`] because the two move
    /// independently: the render scale changes the scene target while the
    /// surface stands still, and a window resize changes both. Rebuilding on
    /// every call would reallocate a surface-sized texture whenever a player
    /// nudged the render-scale row.
    pub fn resize_output(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.output.size == size {
            return;
        }
        match output(device, &self.layout, &self.sampler, self.format, size) {
            Ok(output) => self.output = output,
            // The only failure here is a buffer that would not map, which is
            // not something a window resize should end the game over: keeping
            // the old target draws a stretched frame rather than no frame.
            Err(why) => {
                warn!("the presentation target did not resize ({why:#}); keeping the old one")
            }
        }
    }

    /// Where the UI composites: the resolved scene, at presentation size.
    #[must_use]
    pub fn output(&self) -> &wgpu::TextureView {
        &self.output.view
    }

    /// Puts the presentation target on the surface, graded.
    ///
    /// The other half of [`Framebuffer::resolve_scene`], and the pass that has
    /// to come last: everything a player sees, the UI included, is in the
    /// target by now, so this is the one place brightness and gamma can be
    /// applied to *all* of it. One to one, no resampling - the target is the
    /// surface's own size, and the aspect bars are already in it from the
    /// clear `resolve_scene` did.
    pub fn composite(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        surface: &wgpu::TextureView,
        brightness: Brightness,
        gamma: Gamma,
    ) {
        // Never a decode. The target is `self.format`, which ADR-0020 forces
        // non-sRGB at every real call site, and it is read through an ordinary
        // view rather than the perceptual twin an upscaler wants - so the bytes
        // arrive in the space they were written in. The flag belongs to
        // `resolve_scene`, which is the pass that reads a post-process output.
        let wanted = Grade::new(brightness, gamma, false);
        if wanted != self.output.graded {
            queue.write_buffer(&self.output.grade, 0, bytemuck::bytes_of(&wanted));
            self.output.graded = wanted;
        }
        let size = self.output.size;
        let rect = (0.0, 0.0, size.0 as f32, size.1 as f32);
        self.present(encoder, surface, rect, Some(&self.output.bind_group));
    }

    /// Makes sure the target is `size`, rebuilding it if it is not.
    ///
    /// Returns whether it was rebuilt, which is what tells a caller with its own
    /// size-matched attachments - the race's depth buffer - to rebuild too. A
    /// depth attachment whose size does not match the colour one is a validation
    /// error rather than a bad picture.
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) -> bool {
        let size = (size.0.max(1), size.1.max(1));
        if size == self.size {
            return false;
        }
        let (texture, view, perceptual, bind_group) = target(
            device,
            &self.layout,
            &self.sampler,
            &self.grade,
            self.format,
            size,
        );
        self.texture = texture;
        self.view = view;
        self.perceptual = perceptual;
        self.bind_group = bind_group;
        self.size = size;
        true
    }

    /// The view every stage draws into.
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// How big the target currently is.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Stretches the target into `rect` on `surface`, clearing the rest.
    ///
    /// The clear is what draws the aspect bars, so it happens here rather than
    /// in each stage: a stage now draws into a texture that *is* the game's
    /// rectangle and has no bars in it at all.
    /// A blit source that is not the offscreen target - an upscaler's output.
    ///
    /// Built per frame rather than cached: an upscaler rebuilds its own targets
    /// whenever the presentation rectangle moves, so a cached bind group would
    /// need invalidating on a condition this type cannot see, and a bind group
    /// is a cheap thing to make next to the two passes that produced the view.
    #[must_use]
    pub fn source(&self, device: &wgpu::Device, view: &wgpu::TextureView) -> wgpu::BindGroup {
        bind(device, &self.layout, &self.sampler, &self.grade, view)
    }

    /// The non-sRGB view of the target, for an upscaler to read.
    #[must_use]
    pub fn perceptual(&self) -> &wgpu::TextureView {
        &self.perceptual
    }

    pub fn present(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        surface: &wgpu::TextureView,
        rect: (f32, f32, f32, f32),
        source: Option<&wgpu::BindGroup>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("upscale"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_viewport(rect.0, rect.1, rect.2, rect.3, 0.0, 1.0);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, Some(source.unwrap_or(&self.bind_group)), &[]);
        pass.draw(0..3, 0..1);
    }
}

/// A grade uniform, filled at creation rather than through the queue.
///
/// Through the mapping because [`Framebuffer::new`] has no queue and should not
/// need one: building a framebuffer stays a device-only operation, which is
/// what lets a capture path build one without a frame loop around it.
fn grade_buffer(device: &wgpu::Device, label: &str, graded: Grade) -> Result<wgpu::Buffer> {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: std::mem::size_of::<Grade>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: true,
    });
    buffer
        .slice(..)
        .get_mapped_range_mut()
        .context("mapping the grade buffer")?
        .copy_from_slice(bytemuck::bytes_of(&graded));
    buffer.unmap();
    Ok(buffer)
}

/// Builds the presentation-sized target and the bind group that reads it.
///
/// No non-sRGB twin, unlike [`target`]: nothing samples this one in perceptual
/// space. The upscalers and the post-process passes all read the *scene*
/// target, upstream of here, and the only pass that reads this one is the
/// graded blit onto the surface.
fn output(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    format: wgpu::TextureFormat,
    size: (u32, u32),
) -> Result<Output> {
    let size = (size.0.max(1), size.1.max(1));
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("presentation target"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let graded = Grade::new(Brightness::NEUTRAL, Gamma::NEUTRAL, false);
    let grade = grade_buffer(device, "presentation grade", graded)?;
    let bind_group = bind(device, layout, sampler, &grade, &view);
    Ok(Output {
        view,
        bind_group,
        grade,
        graded,
        size,
    })
}

fn target(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    grade: &wgpu::Buffer,
    format: wgpu::TextureFormat,
    size: (u32, u32),
) -> (
    wgpu::Texture,
    wgpu::TextureView,
    wgpu::TextureView,
    wgpu::BindGroup,
) {
    // The non-sRGB twin is declared here so an upscaler can take a view in it
    // later. Declaring a view format costs nothing when nobody asks for one,
    // and on some backends it is the difference between a texture that can be
    // reinterpreted at all and one that cannot - which is not a thing that can
    // be retrofitted to an already-created texture.
    let twin = format.remove_srgb_suffix();
    let view_formats: &[wgpu::TextureFormat] = if twin == format { &[] } else { &[twin] };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("upscale target"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats,
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let perceptual = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("upscale target (perceptual)"),
        format: Some(twin),
        ..Default::default()
    });
    let bind_group = bind(device, layout, sampler, grade, &view);
    (texture, view, perceptual, bind_group)
}

/// Whether resolving `scene` to `rect` is a magnification, which is the only
/// thing a spatial upscaler is for.
///
/// FSR 1 is a **magnifier**, and upstream says so: EASU's contract is that the
/// output is larger than the input. Handed a render scale above 100 % it is
/// being asked to minify, and its twelve taps then step more than one input
/// texel apart and undersample - so it re-introduces exactly the aliasing the
/// supersampling was there to remove. Measured on a trackside fence at 200 %:
/// the bilinear blit resolves the mesh smoothly and FSR 1 turns it crunchy.
///
/// So the setting is honoured where it means something and quietly not where it
/// would only do harm. A row that silently degrades the picture at three of the
/// six render scales it sits next to would be worse than one that does nothing
/// at those three.
/// Either axis and not both: a render scale applies to both, but a clamped
/// target or an odd rectangle can leave one axis equal while the other is
/// short, and one short axis is still something to reconstruct. On the equal
/// axis EASU then steps exactly one input texel per output texel, which is the
/// one-to-one case - it reconstructs nothing there, but it also cannot produce
/// the undersampling artefact above, which needs a step *greater* than one.
pub(crate) fn magnifies(scene: (u32, u32), rect: (u32, u32)) -> bool {
    scene.0 < rect.0 || scene.1 < rect.1
}

/// The blit's one bind group: a source view, the sampler and the grade.
fn bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    grade: &wgpu::Buffer,
    view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    // Through the probe rather than `device.create_bind_group` directly, the
    // same as the five sites in `oag_render::post`. This is the sixth, and the
    // only one outside that module - which is why a sweep of `post/` alone
    // missed it. A counting passthrough: the same bind group, and nothing at
    // all without the `perf-probe` feature.
    //
    // It counts three callers, and only two of them are per-frame:
    // `resolve_scene` binds an upscaler's or a post-process's output every
    // frame it runs one, while `target` and `output` bind once per resize. A
    // resize is rare enough that the count still reads as per-frame churn.
    oag_render::perfprobe::bind_group(
        device,
        &wgpu::BindGroupDescriptor {
            label: Some("upscale"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: grade.as_entire_binding(),
                },
            ],
        },
    )
}

/// How big the offscreen target should be for a viewport rectangle and a scale.
///
/// Clamped at both ends. The floor is one pixel, because a minimised window and
/// a 25 % scale can otherwise multiply out to zero and a zero-sized texture is a
/// validation error. The ceiling is `limit`, the device's own maximum texture
/// dimension: 200 % of a 4K window is 7680 wide, which is past what some
/// adapters allow, and silently rendering slightly smaller is better than
/// refusing to draw.
#[must_use]
pub fn target_size(rect: (f32, f32, f32, f32), scale: Scale, limit: u32) -> (u32, u32) {
    let factor = scale.factor();
    let scaled = |value: f32| {
        let pixels = (value * factor).round();
        // `as u32` saturates at 0 for negatives and at u32::MAX above, so the
        // clamp below is the only bound that has to be reasoned about.
        (pixels.max(1.0) as u32).clamp(1, limit.max(1))
    };
    (scaled(rect.2), scaled(rect.3))
}

/// Marks `Framebuffer` as holding a texture whose contents are a frame.
///
/// Purely documentary: `texture` is not read outside this module today, and
/// keeping the handle alive is what the bind group needs.
impl std::fmt::Debug for Framebuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Framebuffer")
            .field("size", &self.size)
            .field("format", &self.format)
            .field("texture", &self.texture.size())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
