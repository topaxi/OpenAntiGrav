//! Rendering at a resolution the window is not, and blitting the result up.
//!
//! A **race** draws into an offscreen colour texture rather than straight onto
//! the surface, and [`Framebuffer::resolve_scene`] afterwards stretches that
//! texture into the [`display::viewport`](oag_display::display::viewport) rectangle.
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
//! thing whatever [`Aspect`](oag_display::display::Aspect) is set to - at 50 % on a
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
use log::{debug, warn};

use oag_post::{fsr1, fsr3, fullscreen_layout, fxaa, smaa};

use oag_display::display::{Brightness, Gamma, Reconstruction};

/// Everything the blit needs that a player chose, gathered so the window's
/// frame loop and a capture can be handed the same thing.
#[derive(Debug, Clone, Copy)]
pub struct Presentation {
    /// What resolves the frame onto the surface.
    ///
    /// **One field where there were two.** Anti-aliasing and the upscaler used
    /// to arrive here separately and the ladder below had to reconcile them;
    /// since [ADR-0041](../../../docs/architecture/adr/0041-one-row-for-what-resolves-the-frame.md)
    /// they are one axis, so a pairing like FXAA-under-FSR-3.1 is
    /// unrepresentable rather than something this function has to skip.
    ///
    /// MSAA is not read here: its sample count is baked into the scene's own
    /// pipelines rather than being a blit-time choice - see `race::Scene`.
    pub reconstruction: Reconstruction,
    /// FSR 1's RCAS sharpness in stops. Ignored by the bilinear path.
    pub sharpness: f32,
    pub brightness: Brightness,
    pub gamma: Gamma,
}

mod blit;
mod screen;
mod targets;
mod temporal;

pub use screen::{Composite, ScreenFrame};
pub use temporal::{Temporal, jitter_phases};

use blit::{Grade, Source, bind, grade_buffer, resolved_source};
// `pub` rather than `pub(crate)`, and re-exported rather than merely used:
// `tests/menu_warnings_settings.rs` asks the same question the ladder does,
// and asking it of `upscale` from there is what keeps the two from drifting.
// That test lives in this crate's own `tests/` because it checks `oag-ui`'s
// menu definition against this crate's warning conditions - a claim about
// both sides at once, which only a crate that can see both can make.
pub use blit::magnifies;
pub use targets::target_size;
use targets::{output, target};

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
    /// [`oag_post`].
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
    /// FSR 3.1's eight pipelines and its intermediates, built the first frame
    /// the setting asks for them *and* the adapter can run them. Lazy for the
    /// same reason `fsr1` is, and more so: this one is eight shader
    /// compilations and a dozen textures.
    fsr3: Option<Result<fsr3::Fsr3>>,
    /// What FSR 3.1's intermediates cost, last time it was reported.
    ///
    /// **Held only so the line is printed once per allocation** rather than
    /// once a frame. The widening to baseline-storable formats is that port's
    /// one unmeasured cost - `docs/rendering/fsr3.md` argues it and
    /// `fsr3::Sizes` counts it - and a number nothing prints is an argument
    /// rather than a measurement. It is the reading
    /// [goals.md](../../../docs/overview/goals.md)'s first-tier Steam Deck
    /// question needs, and it moves only when a render scale or a window
    /// resize moves an allocation.
    fsr3_sizes: Option<fsr3::Sizes>,
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
    /// The viewport, in other words, as against the texture. Written every
    /// frame by `Session::render` from `drs::Controller::extent`, which
    /// returns the ceiling itself while `[graphics] dynamic_resolution` is
    /// off - so this is equal to the allocation on a fixed-scale run and
    /// strictly inside it on a controlled one.
    extent: (u32, u32),
    format: wgpu::TextureFormat,
    /// Where the scene lands at presentation size, and where the UI composites
    /// on top of it before one graded pass writes the surface. See [`Output`]
    /// and [ADR-0036](../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md).
    output: Output,
    /// The screen filter between [`Framebuffer::output`] and the grade, when
    /// a profile names one. See [`screen`].
    screen: Option<screen::ScreenFilter>,
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
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/upscale.wgsl")).into(),
            ),
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
        let grade = grade_buffer(device, "upscale grade", graded);

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
            fsr3: None,
            fsr3_sizes: None,
            allocation: size,
            // The whole target to begin with, which is what keeps every caller
            // that predates the split correct with no change: a capture, a
            // `--presented` capture and the window all draw the full
            // rectangle until something calls `set_extent`.
            extent: size,
            format,
            output,
            screen: None,
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
    /// at the scene's own size and handing their output on as what a *spatial*
    /// upscaler reads instead - see [ADR-0013](../../../docs/architecture/adr/0013-anti-aliasing-architecture.md)
    /// for why the order is the reverse of a generic post-process-AA diagram.
    ///
    /// **FSR 3.1 is the exception, and it does not run them at all.** A
    /// temporal reconstruction reasons about the edges a spatial filter has
    /// already blurred, so its input is `self.perceptual` whatever the
    /// anti-aliasing row says - which is the pairing that row's own warning is
    /// about. The pass is therefore *skipped* rather than run and discarded,
    /// which is why the FSR 3.1 resolve is encoded above the anti-aliasing
    /// match rather than below it: only `temporally_resolved` answers "did the
    /// temporal upscaler actually run", where the setting alone does not.
    ///
    /// This puts the frame in [`Framebuffer::output`], **not** on the surface,
    /// and it does so *ungraded* - the caller draws the UI on top of it and
    /// then calls [`Framebuffer::composite`], which is where the grade is and
    /// where the surface is written. See
    /// [ADR-0036](../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md).
    ///
    /// **Returns whether the FSR 3.1 pass actually wrote both halves of
    /// `upscale_timestamps`.** A caller claims its slots before knowing
    /// whether the pipelines will build, the same as
    /// [`oag_post::motion_blur::MotionBlur::render`] and
    /// [`oag_post::hd_bloom::Chain::run`] both do for their own
    /// chains - and, like both of those, needs a way to give an unwritten
    /// claim back rather than let it resolve to an unspecified value. `false`
    /// on every path that returns before `fsr.render` runs: no temporal
    /// history yet, an adapter that cannot run it, or - the case this exists
    /// for - a shader that failed to build. **One answer for both halves**,
    /// because `Fsr3::render` encodes both compute passes or neither, and two
    /// rings that disagree by a frame is exactly what
    /// `Session::feed_drs`'s frame matching cannot survive.
    #[expect(
        clippy::too_many_arguments,
        reason = "two timestamp pairs, which cannot ride in `Temporal`: \
                  `ComputePassTimestampWrites` is `Clone` and not `Copy`"
    )]
    #[must_use = "a claimed slot this returns `false` for must be abandoned, \
                  or it never resolves and costs a `PassTimer` slot forever"]
    pub fn resolve_scene(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        rect: (f32, f32, f32, f32),
        presentation: &Presentation,
        temporal: Option<Temporal<'_>>,
        upscale_timestamps: Option<fsr3::ChainTimestamps<'_>>,
    ) -> bool {
        let output_size = (rect.2 as u32, rect.3 as u32);
        // **The extent, not the allocation**: every pass below is asking
        // "what was drawn", and since ADR-0037 the texture can be larger than
        // that - which it is on every frame a resolution controller has
        // stepped down. See [`Framebuffer::set_extent`].
        let extent = self.extent;
        // Every scene-resolution pass reads a view of the **allocation** while
        // drawing the **extent** into it, and since Phase 5 each is told both:
        // FXAA/SMAA size their own targets off the allocation so a moving
        // extent never rebuilds them, and FSR 1 takes upstream's own
        // `inputViewportInPixels` and `inputSizeInPixels` separately again.
        let allocation = self.allocation;

        // **The fallback ladder, as one expression.** FSR 3.1 when the setting
        // asks for it, the adapter can run it, and this frame is a race with a
        // history to reconstruct from; FSR 1 when the setting asks for *that*
        // and it is actually magnifying; the blit otherwise. A `fsr3` chosen on
        // a machine without compute shaders, or on a menu frame, arrives here as
        // `Upscaler::Fsr1` and takes the middle rung - which is what
        // [ADR-0012] means by degrading rather than failing to boot, and why
        // FSR 1 is load-bearing rather than a stepping stone.
        //
        // [ADR-0012]: ../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md
        let temporal = temporal.filter(|_| presentation.reconstruction == Reconstruction::Fsr3);
        let effective = match (presentation.reconstruction, temporal.is_some()) {
            (Reconstruction::Fsr3, true) => Reconstruction::Fsr3,
            // Every other reading of `fsr3` falls one rung: a menu frame with
            // no scene, an adapter with no compute shaders, and a race whose
            // stage could not answer all arrive here as a `None` bundle.
            (Reconstruction::Fsr3, false) => Reconstruction::Fsr1,
            (chosen, _) => chosen,
        };

        // **Whether both halves of `upscale_timestamps` were actually written
        // into**, for the caller to give two claimed-but-unwritten slots back
        // - see the return value's own doc. Taken from `fsr.render`'s own
        // answer rather than set beside the call: the chain has an early
        // return of its own for a missing allocation, and a flag set on the
        // way in would have claimed that frame as measured.
        let mut upscale_encoded = false;
        let temporally_resolved = temporal.and_then(|temporal| {
            let fsr = self.fsr3.get_or_insert_with(|| fsr3::Fsr3::new(device));
            let fsr = match fsr {
                Ok(fsr) => fsr,
                // A shader that will not compile is a build-time mistake, but
                // it must not be a crash in a player's frame loop: say so once
                // and carry on down the ladder. `upscale_encoded` stays
                // `false`: `fsr.render` below is what would have written
                // `upscale_timestamps`, and this return skips it.
                Err(why) => {
                    warn!("the FSR 3.1 pipelines did not build ({why:#}); staying bilinear");
                    return None;
                }
            };
            upscale_encoded = fsr.render(
                device,
                queue,
                encoder,
                fsr3::Frame {
                    // **The same view FSR 1 reads, not the sRGB one**, and
                    // upstream would want the opposite. ADR-0020 makes gamma
                    // this renderer's authoritative colour space and nothing
                    // linearises, so there is no linear light here to hand it -
                    // see `docs/rendering/fsr3.md`.
                    colour: &self.perceptual,
                    depth: temporal.depth,
                    velocity: temporal.velocity,
                    dispatch: fsr3::Dispatch {
                        render: extent,
                        max_render: allocation,
                        upscale: output_size,
                        jitter: temporal.jitter,
                        phase_count: temporal.phase_count,
                        camera: temporal.camera,
                        // Inert: the only upstream reader of it is the
                        // auto-exposure smoothing this port does not have.
                        delta_time: 1.0 / 60.0,
                        reset: temporal.reset,
                        sample_count: temporal.sample_count,
                        sharpness: fsr1::Sharpness::stops(presentation.sharpness),
                    },
                },
                upscale_timestamps,
            );
            let sizes = fsr.sizes();
            let view = fsr
                .output()
                .map(|view| bind(device, &self.layout, &self.sampler, &self.grade, view));
            // After the render, which is what allocates them - so the first
            // race frame reports rather than the second.
            if let Some(sizes) = sizes
                && self.fsr3_sizes != Some(sizes)
            {
                let mib = |bytes: u64| bytes as f64 / (1024.0 * 1024.0);
                debug!(
                    "FSR 3.1 intermediates: {:.1} MiB ({:.1} render, {:.1} half, {:.1} presentation)",
                    mib(sizes.total()),
                    mib(sizes.render),
                    mib(sizes.half_render),
                    mib(sizes.upscale)
                );
                self.fsr3_sizes = Some(sizes);
            }
            view
        });

        // **A spatial post-process cannot coexist with a reconstruction any
        // more**, so this is a plain match where it used to be a match behind a
        // `temporally_resolved.is_some()` guard. A spatial pass blurs the edges
        // a temporal reconstruction reasons about, and the old shape had to run
        // the resolve first just to know whether to skip it; since ADR-0041 the
        // two are values on one axis and the pairing is unrepresentable. The
        // resolve still happens above, because `temporally_resolved` is what
        // decides the source rectangle further down.
        let post_process: Option<&wgpu::TextureView> = match presentation.reconstruction {
            Reconstruction::Fxaa => {
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
            Reconstruction::Smaa => {
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
            // Every other value on this axis is doing the resolving itself,
            // which is what makes the old `temporally_resolved.is_some()` guard
            // here unnecessary: since ADR-0041 a spatial pass and a
            // reconstruction cannot both be selected, so there is nothing to
            // run-and-discard.
            Reconstruction::Off | Reconstruction::Fsr1 | Reconstruction::Fsr3 => None,
        };
        let upscale_source = post_process.unwrap_or(&self.perceptual);

        let resolved = temporally_resolved.or_else(|| {
            (effective == Reconstruction::Fsr1 && magnifies(extent, output_size))
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
                .flatten()
        });
        // **Whether an upscaler actually resolved**, which decides the source
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
        upscale_encoded
    }

    /// Whether FSR 3.1's pipelines are known *not* to build.
    ///
    /// **For the caller's timestamp claim, not for the ladder.** The ladder
    /// falls a rung inside [`Framebuffer::resolve_scene`] and needs no help;
    /// what a caller cannot see from outside is that a build failure is
    /// permanent - the `Err` is kept rather than retried - so a frame loop
    /// claiming a timestamp pair for a pass that will never be recorded would
    /// go on claiming one every frame. `true` before the first attempt, which
    /// is right: nothing has failed yet.
    #[must_use]
    pub fn temporal_upscaler_viable(&self) -> bool {
        !matches!(self.fsr3, Some(Err(_)))
    }

    /// Whether a spatial anti-aliasing pass has ever been *built*, for a test.
    ///
    /// Both are lazy - constructed the first frame the row asks for one - so
    /// "was it built" is exactly "did a frame ever run it", which is what makes
    /// this the observable for FSR 3.1 skipping them. A pass that ran and had
    /// its output discarded and a pass that never ran produce the same picture;
    /// only this tells them apart.
    ///
    /// **On a fresh `Framebuffer`**, which is the precondition and not a
    /// detail: this says "ever", so a framebuffer reused across two settings
    /// answers about the pair of them. A test comparing two upscalers builds
    /// one framebuffer per reading.
    #[must_use]
    pub fn built_spatial_anti_aliasing(&self) -> bool {
        self.fxaa.is_some() || self.smaa.is_some()
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

    /// How big [`Framebuffer::output`] actually is.
    ///
    /// **Not always `Gpu::size`, which is what a caller wanting a depth or
    /// other size-matched attachment for it must not assume instead.**
    /// [`Framebuffer::resize_output`] keeps the previous, smaller target in
    /// place on an allocation failure rather than reporting the size it was
    /// asked for, so the two can disagree for the life of that target - the
    /// same size-matched-attachment mistake `RaceStage::draw_hud`'s countdown
    /// pass made against the *scene* target before it read
    /// [`Framebuffer::allocation`] instead of `Gpu::size`.
    #[must_use]
    pub fn output_size(&self) -> (u32, u32) {
        self.output.size
    }

    /// Puts the presentation target on the surface, graded - through the
    /// screen filter first, if [`Framebuffer::set_screen_filter`] named one.
    ///
    /// The other half of [`Framebuffer::resolve_scene`], and the pass that has
    /// to come last: everything a player sees, the UI included, is in the
    /// target by now, so this is the one place brightness and gamma can be
    /// applied to *all* of it - and, for the same reason, the one place a
    /// display simulation can be drawn over all of it. The filter runs
    /// before the grade, not after: it is part of the picture, and the grade
    /// is the monitor. See [`screen`]. One to one, no resampling - the target
    /// is the surface's own size, and the aspect bars are already in it from
    /// the clear `resolve_scene` did.
    pub fn composite(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        surface: &wgpu::TextureView,
        frame: Composite,
    ) {
        // Never a decode. The target is `self.format`, which ADR-0020 forces
        // non-sRGB at every real call site, and it is read through an ordinary
        // view rather than the perceptual twin an upscaler wants - so the bytes
        // arrive in the space they were written in. The flag belongs to
        // `resolve_scene`, which is the pass that reads a post-process output.
        // A filter's output is the same format and the same space, so the
        // flag is the same either way.
        let wanted = Grade::new(frame.brightness, frame.gamma, false);
        if wanted != self.output.graded {
            queue.write_buffer(&self.output.grade, 0, bytemuck::bytes_of(&wanted));
            self.output.graded = wanted;
        }
        let filtered = self.filter_output(device, queue, encoder, frame.screen);
        let size = self.output.size;
        let rect = (0.0, 0.0, size.0 as f32, size.1 as f32);
        self.present(
            encoder,
            surface,
            rect,
            Some(filtered.as_ref().unwrap_or(&self.output.bind_group)),
        );
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

// The screen filter's own tests, beside `extent_tests` for the same reason.
#[cfg(test)]
mod screen_tests;
