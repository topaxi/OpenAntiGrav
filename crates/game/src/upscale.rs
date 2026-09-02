//! Rendering at a resolution the window is not, and blitting the result up.
//!
//! A **race** draws into an offscreen colour texture rather than straight onto
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
//! **Every other stage skips the offscreen texture entirely** and draws into
//! that presentation target itself - the launcher, the loading screen, the
//! front end and the menus are UI all the way down, so there is no scene for an
//! upscaler to carry and the render scale means nothing to them. See
//! [ADR-0038](../../../docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md).
//! `resolve_scene` is not called on those frames, and their own clear draws the
//! aspect bars that [`Framebuffer::present`] would otherwise have drawn.
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

use anyhow::Result;
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

mod blit;

use blit::{Grade, Source, grade_buffer};

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
    /// The scene texture's real dimensions, and what every size-matched
    /// attachment is built against.
    ///
    /// `target_size` off the **ceiling** - `[graphics] render_scale` - so it
    /// moves when the window, the aspect or that row moves, and not otherwise.
    /// See [ADR-0037](../../../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md).
    allocation: (u32, u32),
    /// How much of [`Framebuffer::allocation`] this frame is drawn into,
    /// anchored at the origin and never larger than it.
    ///
    /// The viewport, in other words, as against the texture. Equal to the
    /// allocation on every frame the game currently draws - nothing moves it
    /// yet - and the value a dynamic-resolution controller will write.
    extent: (u32, u32),
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
            allocation: size,
            // The whole target to begin with, which is what keeps every caller
            // that predates the split correct with no change: a capture, a
            // `--presented` capture and the window all draw the full
            // rectangle until something calls `set_extent`.
            extent: size,
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
        self.set_blit(queue, brightness, gamma, decode, Source::WHOLE);
    }

    /// The same, plus how much of the bound source to read.
    ///
    /// One buffer and one comparison for both, because they are one uniform:
    /// see [`Grade`]. Only [`Framebuffer::resolve_scene`] has anything but
    /// [`Source::WHOLE`] to pass, and only when the render extent is below the
    /// allocation.
    fn set_blit(
        &mut self,
        queue: &wgpu::Queue,
        brightness: Brightness,
        gamma: Gamma,
        decode: bool,
        source: Source,
    ) {
        let wanted = Grade::new(brightness, gamma, decode).reading(source);
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
        // **The extent, not the allocation**: every pass below is asking
        // "what was drawn", and since ADR-0037 the texture can be larger than
        // that. The two are equal on every frame the game draws today - see
        // [`Framebuffer::set_extent`] for what bounds that.
        let extent = self.extent;
        // Every scene-resolution pass reads a view of the **allocation** while
        // drawing the **extent** into it, and since Phase 5 each is told both:
        // FXAA/SMAA size their own targets off the allocation so a moving
        // extent never rebuilds them, and FSR 1 takes upstream's own
        // `inputViewportInPixels` and `inputSizeInPixels` separately again.
        let allocation = self.allocation;

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
                                size: allocation,
                                viewport: extent,
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
                                size: allocation,
                                viewport: extent,
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

        let resolved = (presentation.upscaler == Upscaler::Fsr1 && magnifies(extent, output_size))
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
                        viewport: extent,
                        input: allocation,
                        output: output_size,
                        sharpness: fsr1::Sharpness::stops(presentation.sharpness),
                    },
                );
                fsr.output()
                    .map(|view| bind(device, &self.layout, &self.sampler, &self.grade, view))
            })
            .flatten();
        // **Whether FSR 1 actually resolved**, which decides the source
        // rectangle below - not `source.is_some()`, which is also true for an
        // FXAA or SMAA frame that is scene-sized.
        let upscaled = resolved.is_some();
        let source = resolved
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
        //
        // The source rectangle rides the same write, and **the question is
        // which size the bound view is, not whether a pass ran.** FSR 1
        // resolves to the presentation rectangle and hands back a texture
        // exactly that size, so its output is whole. FXAA and SMAA draw at
        // scene resolution into an allocation-sized target with the extent in
        // its corner, so their output owes the same sub-rectangle the scene
        // target does.
        //
        // The two arms are identical while the extent is the allocation, which
        // is every frame until a controller moves it - so no capture can tell
        // a correct reading of this from a wrong one, and the test named on
        // `Framebuffer::set_extent` is the only thing that can.
        let read = resolved_source(upscaled, extent, allocation);
        self.set_blit(
            queue,
            Brightness::NEUTRAL,
            Gamma::NEUTRAL,
            source.is_some() && self.format.is_srgb(),
            read,
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

    /// Makes sure the **allocation** is `size`, rebuilding it if it is not.
    ///
    /// Returns whether it was rebuilt, which is what tells a caller with its own
    /// size-matched attachments - the race's depth buffer - to rebuild too. A
    /// depth attachment whose size does not match the colour one is a validation
    /// error rather than a bad picture.
    ///
    /// `size` is the **ceiling**: `target_size` off `[graphics] render_scale`,
    /// which moves when the window, the aspect or that row moves. A
    /// dynamic-resolution controller does not call this - it calls
    /// [`Framebuffer::set_extent`], which allocates nothing. See
    /// [ADR-0037](../../../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md).
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) -> bool {
        let size = (size.0.max(1), size.1.max(1));
        if size == self.allocation {
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
        self.allocation = size;
        // **Reset, not preserved.** A window resize that shrinks the
        // allocation while a stale larger extent survived would set a viewport
        // past the attachment, which is a validation error rather than a bad
        // picture. A controller re-applies its own value on the next frame,
        // which costs it one frame at full size and costs nothing to reason
        // about.
        self.extent = size;
        true
    }

    /// Moves the render extent - how much of the allocation this frame is
    /// drawn into - without allocating anything.
    ///
    /// Clamped into `1..=allocation` on each axis, so the invariant the
    /// viewport depends on cannot be broken from outside. This is the whole
    /// per-frame cost of dynamic resolution: a uniform write and two viewport
    /// calls, against the six texture creations [`Framebuffer::resize`] pays.
    ///
    /// # Who may call this
    ///
    /// **The frame loop, once a frame, from `drs::Controller::extent`** - and
    /// deliberately every frame rather than only when the value moves, because
    /// [`Framebuffer::resize`] resets the extent whenever it reallocates.
    ///
    /// **Not the capture paths.** `race/capture.rs` builds its own
    /// `Framebuffer` at its own `--render-scale` and must never gain a
    /// controller: this project's comparisons are byte-identical screenshot
    /// diffs, and a rectangle that follows how busy the machine is makes every
    /// one of them irreproducible. The front-end capture has no `Framebuffer`
    /// at all, which since ADR-0038 is correct rather than a gap.
    ///
    /// Every scene-resolution pass takes a resource size and a viewport
    /// separately now - FSR 1 back to `ffx_fsr1.h`'s own two arguments, FXAA
    /// and SMAA sizing their targets off the allocation so a moving extent
    /// never rebuilds them, and both blooms reading a sub-rectangle and
    /// writing into one. See
    /// [`docs/rendering/dynamic-resolution.md`](../../../docs/rendering/dynamic-resolution.md).
    pub fn set_extent(&mut self, extent: (u32, u32)) {
        self.extent = (
            extent.0.clamp(1, self.allocation.0),
            extent.1.clamp(1, self.allocation.1),
        );
    }

    /// The view every stage draws into.
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// How big the scene texture actually is.
    ///
    /// What an attachment builder wants: a depth or multisampled attachment
    /// has to match this, not [`Framebuffer::extent`]. There is no `size()`
    /// deliberately - since ADR-0037 the two questions have different answers
    /// and a caller has to say which it meant.
    #[must_use]
    pub fn allocation(&self) -> (u32, u32) {
        self.allocation
    }

    /// How much of it this frame is drawn into.
    ///
    /// What a viewport wants, and therefore what anything normalising against
    /// the frame's pixels wants - the projection, and the sub-pixel camera
    /// jitter of [ADR-0039](../../../docs/architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md),
    /// which takes the viewport it is handed rather than reading a size here.
    #[must_use]
    pub fn extent(&self) -> (u32, u32) {
        self.extent
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

/// Which rectangle of the bound source the blit reads.
///
/// **The question is which size the bound view is, not whether a pass ran.**
/// FSR 1 resolves to the presentation rectangle and hands back a texture
/// exactly that size, so its output is the whole thing. FXAA and SMAA draw at
/// scene resolution into an allocation-sized target with the extent in its
/// corner, so their output owes the same sub-rectangle the scene target does -
/// and `source.is_some()` cannot tell those two apart.
///
/// Its own function because both arms are identical while the extent is the
/// allocation, which is every frame until a controller moves it: no
/// `--presented` capture can distinguish a correct reading of this from a
/// wrong one, so a test on the decision itself is the only guard there is.
fn resolved_source(upscaled: bool, extent: (u32, u32), allocation: (u32, u32)) -> Source {
    if upscaled {
        Source::WHOLE
    } else {
        Source::of(extent, allocation)
    }
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
            .field("allocation", &self.allocation)
            .field("extent", &self.extent)
            .field("format", &self.format)
            .field("texture", &self.texture.size())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;

// A second test file rather than more of `tests`, which is already 874 lines
// against `just check-size`'s 1,000-line ratchet.
#[cfg(test)]
mod extent_tests;
