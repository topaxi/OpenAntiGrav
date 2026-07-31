//! Rendering at a resolution the window is not, and blitting the result up.
//!
//! Every stage draws into an offscreen colour texture rather than straight onto
//! the surface, and one pass afterwards stretches that texture into the
//! [`display::viewport`](crate::display::viewport) rectangle. Below 100 % that
//! is the usual internal-resolution knob; above it, it is supersampling.
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
//! Because this is the one pass every frame goes through. The front end, the
//! menus and a race are three renderers that share nothing else, and grading in
//! each would be three places to get it wrong and three places to forget when a
//! fourth stage lands. Here it is one shader, and a player calibrating the
//! picture sees the menu they are standing on change as they do it.
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

use oag_render::post::fsr1;

use crate::display::{Brightness, Gamma, Scale, Upscaler};

/// Everything the blit needs that a player chose, gathered so the window's
/// frame loop and a capture can be handed the same thing.
#[derive(Debug, Clone, Copy)]
pub struct Presentation {
    /// Which resampler carries the frame onto the surface.
    pub upscaler: Upscaler,
    /// FSR 1's RCAS sharpness in stops. Ignored by the bilinear path.
    pub sharpness: f32,
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
    /// Zero when the source is the offscreen target, which is bound through an
    /// sRGB view and so arrives already decoded by the sampler. One when it is
    /// an upscaler's output, which is deliberately *not* sRGB-formatted because
    /// the upscaler works in perceptual space and a decode on every internal
    /// read would be both wrong and paid twice. See `oag_render::post`.
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
    size: (u32, u32),
    format: wgpu::TextureFormat,
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

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("upscale"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

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
        // install draws. Filled at creation rather than through the queue,
        // which keeps building a framebuffer a device-only operation.
        let graded = Grade::new(Brightness::NEUTRAL, Gamma::NEUTRAL, false);
        let grade = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("upscale grade"),
            size: std::mem::size_of::<Grade>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });
        grade
            .slice(..)
            .get_mapped_range_mut()
            .context("mapping the grade buffer")?
            .copy_from_slice(bytemuck::bytes_of(&graded));
        grade.unmap();

        let (texture, view, perceptual, bind_group) =
            target(device, &layout, &sampler, &grade, format, size);
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
            size,
            format,
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
    /// bound rather than the setting that asked for it, and the upscaler has to
    /// run after every stage has finished drawing but before the blit.
    pub fn resolve(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        surface: &wgpu::TextureView,
        rect: (f32, f32, f32, f32),
        presentation: &Presentation,
    ) {
        let source = (presentation.upscaler == Upscaler::Fsr1).then(|| {
            let fsr = self
                .fsr1
                .get_or_insert_with(|| fsr1::Fsr1::new(device, self.format));
            let fsr = match fsr {
                Ok(fsr) => fsr,
                // A shader that will not compile is a build-time mistake, but
                // it must not be a crash in a player's frame loop: say so once
                // and carry on bilinear.
                Err(why) => {
                    eprintln!("the FSR 1 pipelines did not build ({why:#}); staying bilinear");
                    return None;
                }
            };
            fsr.render(
                device,
                queue,
                encoder,
                fsr1::Frame {
                    source: &self.perceptual,
                    input: self.size,
                    output: (rect.2 as u32, rect.3 as u32),
                    sharpness: fsr1::Sharpness::stops(presentation.sharpness),
                },
            );
            fsr.output()
                .map(|view| bind(device, &self.layout, &self.sampler, &self.grade, view))
        });
        let source = source.flatten();

        self.set_grade(
            queue,
            presentation.brightness,
            presentation.gamma,
            source.is_some(),
        );
        self.present(encoder, surface, rect, source.as_ref());
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

/// The blit's one bind group: a source view, the sampler and the grade.
fn bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    grade: &wgpu::Buffer,
    view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
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
    })
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
mod tests {
    use super::*;

    const LIMIT: u32 = 8192;

    /// Blits a flat colour with and without FSR 1 and returns both results.
    ///
    /// The offscreen target is **sRGB**, as it is in the game, which is what
    /// makes this worth doing at all: it is the only configuration where the
    /// perceptual view differs from the ordinary one, and so the only one that
    /// exercises the whole colour-space arrangement rather than an accidental
    /// identity. The surface is `Rgba8Unorm` so the readback is the shader's
    /// own linear output with no second curve on top.
    ///
    /// Returns `None` with no adapter, so the caller skips.
    fn both_paths(input: [f64; 3]) -> Option<([u8; 4], [u8; 4])> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("upscale fsr test"),
            ..Default::default()
        }))
        .ok()?;

        // One format for the offscreen target, the blit's pipeline and the
        // surface, exactly as the game has it: `Framebuffer::new` takes the
        // surface format and builds both against it.
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let mut fsr =
            oag_render::post::fsr1::Fsr1::new(&device, format).expect("the fsr pipelines");
        let mut out = Vec::new();
        for upscaling in [false, true] {
            // Four texels in, sixteen out. Small, and still a real upscale: at
            // one texel EASU's twelve taps would all be the same clamped edge.
            let mut framebuffer = Framebuffer::new(&device, format, (2, 2)).expect("the pipeline");

            let surface = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("fsr readback"),
                size: wgpu::Extent3d {
                    width: 4,
                    height: 4,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let surface_view = surface.create_view(&Default::default());
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("fsr readback"),
                size: (wgpu::COPY_BYTES_PER_ROW_ALIGNMENT * 4) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            let mut encoder = device.create_command_encoder(&Default::default());
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fsr input"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: framebuffer.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: input[0],
                            g: input[1],
                            b: input[2],
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            let source = upscaling.then(|| {
                fsr.render(
                    &device,
                    &queue,
                    &mut encoder,
                    oag_render::post::fsr1::Frame {
                        source: framebuffer.perceptual(),
                        input: framebuffer.size(),
                        output: (4, 4),
                        sharpness: oag_render::post::fsr1::Sharpness::DEFAULT,
                    },
                );
                framebuffer.source(&device, fsr.output().expect("an fsr output"))
            });
            framebuffer.set_grade(
                &queue,
                Brightness::NEUTRAL,
                Gamma::NEUTRAL,
                source.is_some(),
            );
            framebuffer.present(
                &mut encoder,
                &surface_view,
                (0.0, 0.0, 4.0, 4.0),
                source.as_ref(),
            );

            encoder.copy_texture_to_buffer(
                surface.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                        rows_per_image: Some(4),
                    },
                },
                wgpu::Extent3d {
                    width: 4,
                    height: 4,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(Some(encoder.finish()));

            let slice = readback.slice(..);
            slice.map_async(wgpu::MapMode::Read, |_| {});
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("the GPU");
            // The middle of the picture, away from the edge clamping.
            let at = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize + 4;
            let mapped = slice.get_mapped_range().expect("the readback");
            out.push([mapped[at], mapped[at + 1], mapped[at + 2], mapped[at + 3]]);
            drop(mapped);
            readback.unmap();
        }
        Some((out[0], out[1]))
    }

    /// Turning the upscaler on must not move a flat colour.
    ///
    /// This is the whole colour-space arrangement in one assertion, and it is
    /// the cheapest way to catch the mistake most likely to be made here.
    /// A flat picture is a fixed point of both EASU and RCAS, so the only thing
    /// that can differ between the two paths is the transfer function: the
    /// bilinear path lets the sampler decode an sRGB view, the FSR path reads a
    /// non-sRGB view and decodes in the shader. If those two disagree - if the
    /// decode were `pow(c, 2.2)` standing in for the real curve, or if the
    /// non-sRGB view were not actually reaching the shader - a flat grey would
    /// come out at two different values and this fails.
    ///
    /// **Skips with no adapter**, so a green CI run is not evidence it ran.
    #[test]
    fn turning_the_upscaler_on_does_not_shift_a_flat_colour() {
        // Three levels, because the sRGB curve's two pieces meet in the darks
        // and an approximation goes wrong there first.
        for level in [0.02, 0.25, 0.5] {
            let Some((bilinear, fsr)) = both_paths([level, level, level]) else {
                eprintln!("no GPU adapter: skipping");
                return;
            };
            for channel in 0..3 {
                let drift = i32::from(bilinear[channel]).abs_diff(i32::from(fsr[channel]));
                assert!(
                    drift <= 2,
                    "at {level}, channel {channel}: bilinear gave {bilinear:?} and fsr gave \
                     {fsr:?}, a drift of {drift}/255 - the two paths disagree about the \
                     transfer function"
                );
            }
        }
    }

    /// Runs one grade through the real pipeline and reads back the pixel.
    ///
    /// `Rgba8Unorm` and not the surface's sRGB format, so the arithmetic is
    /// exact rather than encoded on the way out: this is asserting what the
    /// shader *did*, and a gamma encode on top of it would put every expected
    /// value behind a second curve.
    ///
    /// Returns `None` on a machine with no adapter, which is what CI's runners
    /// are - so the caller skips rather than fails there.
    fn graded(input: [f32; 4], brightness: Brightness, gamma: Gamma) -> Option<[u8; 4]> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("upscale grade test"),
            ..Default::default()
        }))
        .ok()?;

        let format = wgpu::TextureFormat::Rgba8Unorm;
        let mut framebuffer = Framebuffer::new(&device, format, (1, 1)).expect("the pipeline");
        framebuffer.set_grade(&queue, brightness, gamma, false);

        let surface = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("grade readback"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let surface_view = surface.create_view(&Default::default());
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grade readback"),
            size: wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&Default::default());
        // The offscreen target is filled by clearing it, which is the cheapest
        // way to put a known colour under the blit.
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("grade input"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: framebuffer.view(),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: f64::from(input[0]),
                        g: f64::from(input[1]),
                        b: f64::from(input[2]),
                        a: f64::from(input[3]),
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        framebuffer.present(&mut encoder, &surface_view, (0.0, 0.0, 1.0, 1.0), None);
        encoder.copy_texture_to_buffer(
            surface.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the GPU");
        let mapped = slice.get_mapped_range().expect("the readback");
        Some([mapped[0], mapped[1], mapped[2], mapped[3]])
    }

    /// The one thing no other check in this repository can see.
    ///
    /// `--screenshot` deliberately bypasses this pass, so brightness and gamma
    /// have no picture anywhere to be compared against, and at their defaults
    /// both are 1.0 - a swapped pair or a misbound uniform is invisible in
    /// every other test here and would ship looking fine. So this one renders
    /// through the real pipeline and reads the pixel back.
    #[test]
    fn the_grade_moves_the_picture_in_the_direction_the_setting_names() {
        // A midtone: black and white are the two values gamma cannot move, so
        // either would pass a broken exponent.
        const INPUT: [f32; 4] = [0.25, 0.25, 0.25, 1.0];
        let neutral = Brightness::NEUTRAL;
        let plain = Gamma::NEUTRAL;

        let Some(untouched) = graded(INPUT, neutral, plain) else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        // 0.25 in, 0.25 out - the neutral grade is not a grade.
        assert!(
            untouched[0].abs_diff(64) <= 1,
            "neutral changed it: {untouched:?}"
        );
        assert_eq!(untouched[3], 255, "alpha must come through untouched");

        let up = graded(INPUT, neutral, "140".parse().expect("parse")).expect("an adapter");
        let down = graded(INPUT, neutral, "60".parse().expect("parse")).expect("an adapter");
        assert!(up[0] > untouched[0], "gamma 140 has to brighten: {up:?}");
        assert!(down[0] < untouched[0], "gamma 60 has to darken: {down:?}");

        // Brightness on its own, at the neutral gamma, is exactly the
        // multiply - which is also what proves the two are not swapped, since
        // a gamma of 1.5 applied to 0.25 is nothing like 1.5 times it.
        let brighter = graded(INPUT, "150".parse().expect("parse"), plain).expect("an adapter");
        assert!(
            brighter[0].abs_diff(96) <= 1,
            "150 % of 0.25 is 0.375: {brighter:?}"
        );
        let darker = graded(INPUT, "50".parse().expect("parse"), plain).expect("an adapter");
        assert!(darker[0].abs_diff(32) <= 1, "half of 0.25: {darker:?}");
    }

    /// The uniform has to be the size a uniform binding may be, and the
    /// neutral grade has to be the one that changes nothing - `set_grade`
    /// compares against it to decide whether to write at all, so a wrong
    /// neutral would leave a fresh install grading its own picture.
    #[test]
    fn the_neutral_grade_changes_nothing_and_fills_a_uniform_binding() {
        assert_eq!(std::mem::size_of::<Grade>(), 16);
        let neutral = Grade::new(Brightness::NEUTRAL, Gamma::NEUTRAL, false);
        assert_eq!(neutral.brightness, 1.0);
        assert_eq!(neutral.exponent, 1.0);
        assert_eq!(neutral.padding, 0.0);
        // And it is what `Default` gives, which is what an untouched settings
        // file loads as.
        assert_eq!(
            Grade::new(Brightness::default(), Gamma::default(), false),
            neutral
        );
    }

    /// The decode flag is what stops an upscaled frame being graded in the
    /// wrong space, and it is a float in the uniform because WGSL has no
    /// `bool` it can read from a buffer. Zero and one, not "anything truthy".
    #[test]
    fn the_decode_flag_is_zero_or_one_and_nothing_else() {
        let plain = Grade::new(Brightness::NEUTRAL, Gamma::NEUTRAL, false);
        let decoded = Grade::new(Brightness::NEUTRAL, Gamma::NEUTRAL, true);
        assert_eq!(plain.decode, 0.0);
        assert_eq!(decoded.decode, 1.0);
        // And it is part of what `set_grade` compares, so flipping the source
        // rewrites the uniform even when neither menu row moved.
        assert_ne!(plain, decoded);
    }

    #[test]
    fn full_scale_is_the_rectangle_itself() {
        let size = target_size((0.0, 0.0, 1440.0, 816.0), Scale::FULL, LIMIT);
        assert_eq!(size, (1440, 816));
    }

    #[test]
    fn a_half_scale_halves_both_axes() {
        let size = target_size(
            (0.0, 0.0, 1920.0, 1080.0),
            "50".parse().expect("parse"),
            LIMIT,
        );
        assert_eq!(size, (960, 540));
    }

    #[test]
    fn a_scale_above_full_supersamples() {
        let size = target_size(
            (0.0, 0.0, 1920.0, 1080.0),
            "200".parse().expect("parse"),
            LIMIT,
        );
        assert_eq!(size, (3840, 2160));
    }

    /// Measured against the rectangle, so the pillarbox a 4:3 aspect leaves
    /// costs no offscreen pixels: the same scale on the same window renders
    /// fewer of them than at `free`, which is the point.
    #[test]
    fn the_scale_follows_the_rectangle_rather_than_the_window() {
        let free = target_size(
            crate::display::viewport((1920, 1080), crate::display::Aspect::Free),
            Scale::FULL,
            LIMIT,
        );
        let ps2 = target_size(
            crate::display::viewport((1920, 1080), crate::display::Aspect::Ps2),
            Scale::FULL,
            LIMIT,
        );
        assert_eq!(free.0, 1920);
        assert!(ps2.0 < free.0, "{ps2:?} against {free:?}");
        assert_eq!(ps2.1, free.1, "the pillarbox loses width, not height");
    }

    /// 200 % of a 4K window is past what some adapters allow, and drawing
    /// slightly smaller beats refusing to draw.
    #[test]
    fn an_oversized_target_is_clamped_to_what_the_device_allows() {
        let size = target_size(
            (0.0, 0.0, 3840.0, 2160.0),
            "200".parse().expect("parse"),
            4096,
        );
        // 7680x4320 wanted, 4096 allowed on both axes.
        assert_eq!(size, (4096, 4096));
    }

    /// A minimised window times a small scale rounds to zero, and a zero-sized
    /// texture is a validation error rather than a blank frame.
    #[test]
    fn a_degenerate_rectangle_still_gives_a_creatable_texture() {
        for rect in [(0.0, 0.0, 0.0, 0.0), (0.0, 0.0, 1.0, 1.0)] {
            for percent in ["25", "50", "100", "200"] {
                let size = target_size(rect, percent.parse().expect("parse"), LIMIT);
                assert!(
                    size.0 >= 1 && size.1 >= 1,
                    "{rect:?} at {percent}: {size:?}"
                );
            }
        }
    }
}
