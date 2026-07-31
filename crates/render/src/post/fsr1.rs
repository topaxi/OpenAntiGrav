//! AMD FidelityFX Super Resolution 1: EASU, then RCAS.
//!
//! Two fullscreen passes. The first, EASU, resamples the scene target up to the
//! presentation rectangle with an edge-adaptive twelve-tap kernel; the second,
//! RCAS, sharpens the result back up without the ringing an unconditional
//! sharpen would add. Both are transliterated from AMD's MIT-licensed
//! `ffx_fsr1.h` - see [`fsr1.wgsl`](./fsr1.wgsl) and
//! `licences/AMD-FidelityFX-MIT.txt`.
//!
//! # Why this and not the temporal upscaler
//!
//! FSR 1 is spatial: it reads one frame and needs nothing else. It therefore
//! wants none of the things a temporal upscaler wants - no motion vectors, no
//! readable depth, no camera jitter, no history - which is what makes it
//! useful *now*, and what caps its quality below FSR 3.1's. It is also the
//! middle rung of the fallback chain, for adapters that cannot give the
//! temporal path the storage-texture features it needs.
//!
//! # Colour space
//!
//! Both passes work on **sRGB-encoded** values. See the [module
//! docs](super) for the table of who wants what; the short version is that the
//! caller must hand [`Fsr1::render`] a *non-sRGB* view of the scene target, and
//! must decode [`Fsr1::output`] before grading it.

use anyhow::Result;

/// The two passes' constants, as the shader's uniform expects them.
///
/// Upstream bit-packs these into `uint4`s so that its 16-bit path can unpack two
/// halves from one lane; nothing here does, so they stay floats.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    con0: [f32; 4],
    con1: [f32; 4],
    con2: [f32; 4],
    con3: [f32; 4],
    sharpness: f32,
    padding: [f32; 3],
}

impl Constants {
    /// `FsrEasuCon` and `FsrRcasCon` in one.
    ///
    /// The input viewport and the input resource are the same rectangle here -
    /// the scene target has nothing else in it - so upstream's separate
    /// viewport and size arguments collapse into one.
    fn new(input: (u32, u32), output: (u32, u32), sharpness: Sharpness) -> Self {
        let (iw, ih) = (input.0.max(1) as f32, input.1.max(1) as f32);
        let (ow, oh) = (output.0.max(1) as f32, output.1.max(1) as f32);
        Self {
            // Output integer position to a pixel position in viewport.
            con0: [iw / ow, ih / oh, 0.5 * iw / ow - 0.5, 0.5 * ih / oh - 0.5],
            // Viewport pixel position to normalized image space, then the
            // centres of the first gather4 relative to the upper-left of 'F'.
            con1: [1.0 / iw, 1.0 / ih, 1.0 / iw, -1.0 / ih],
            // The remaining three gather centres, relative to the first.
            con2: [-1.0 / iw, 2.0 / ih, 1.0 / iw, 2.0 / ih],
            con3: [0.0 / iw, 4.0 / ih, 0.0, 0.0],
            sharpness: sharpness.factor(),
            padding: [0.0; 3],
        }
    }
}

/// How hard RCAS sharpens, in upstream's units: **stops**, where 0 is maximum
/// and each whole step halves it.
///
/// Kept as a newtype rather than a bare `f32` because the scale is neither
/// linear nor in the direction a reader expects - a larger number sharpens
/// *less* - and because the shader wants `exp2(-stops)` rather than the value
/// itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sharpness(f32);

impl Sharpness {
    /// Upstream's own default, and what the sample ships with.
    pub const DEFAULT: Self = Self(0.2);
    /// A zero lobe, which is as close to no sharpening as this pass gets.
    ///
    /// **Not quite a pass-through**, and the difference is upstream's rather
    /// than a mistake here: the resolve divides by `prx_med_rcp(4*lobe + 1)`,
    /// and that approximation returns 0.99685 at 1.0 rather than exactly 1.0.
    /// So a zero lobe still darkens by about a third of a percent - 0.4/255 at
    /// mid grey. Nothing in the game selects this ([`Sharpness::stops`] clamps
    /// to upstream's documented range and never reaches it); a caller that
    /// genuinely wants an untouched frame should skip the pass, not ask for
    /// zero.
    pub const OFF: Self = Self(f32::INFINITY);

    /// Clamps `stops` into the range upstream documents, `0.0` to `2.0`.
    #[must_use]
    pub fn stops(stops: f32) -> Self {
        Self(stops.clamp(0.0, 2.0))
    }

    /// `exp2(-stops)`, which is what the shader multiplies the lobe by.
    #[must_use]
    pub fn factor(self) -> f32 {
        // `OFF` is an infinity, and `exp2(-inf)` is exactly zero, so the lobe
        // is exactly zero - which is not the same as the pass being an
        // identity. See [`Sharpness::OFF`].
        (-self.0).exp2()
    }
}

impl Default for Sharpness {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// One frame's worth of input to [`Fsr1::render`].
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    /// A **non-sRGB** view of the scene target. See the [module docs](super):
    /// EASU reasons about luma the way an eye does, so it must be handed the
    /// encoded values rather than the linear light an sRGB view would decode to.
    pub source: &'a wgpu::TextureView,
    /// The size of `source`, which is the scene target's whole extent.
    pub input: (u32, u32),
    /// The size to resolve to: the presentation rectangle, not the surface.
    pub output: (u32, u32),
    /// How hard RCAS sharpens.
    pub sharpness: Sharpness,
}

/// The EASU and RCAS pipelines, their uniform, and the two intermediate targets.
#[derive(Debug)]
pub struct Fsr1 {
    easu: wgpu::RenderPipeline,
    rcas: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    constants: wgpu::Buffer,
    written: Option<Constants>,
    /// EASU's output and RCAS's input, at the presentation size.
    upscaled: Option<Target>,
    /// RCAS's output, and what [`Fsr1::output`] hands back.
    sharpened: Option<Target>,
    /// Non-sRGB, because both intermediates hold values that are already
    /// encoded and must not be decoded on the way in or out.
    format: wgpu::TextureFormat,
    size: (u32, u32),
}

#[derive(Debug)]
struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

impl Fsr1 {
    /// Builds both pipelines against a surface `format`.
    ///
    /// The intermediates are built at the first [`render`](Self::render), which
    /// is the first time an output size is known.
    ///
    /// # Errors
    ///
    /// Propagates a shader that will not compile, which is a build-time mistake
    /// rather than anything a player can cause.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fsr1"),
            source: wgpu::ShaderSource::Wgsl(include_str!("fsr1.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fsr1"),
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

        // **Nearest, and clamped.** EASU gathers four texels at a time and does
        // its own weighting; a linear sampler would pre-blend them and the
        // twelve-tap kernel would be filtering an already-filtered image.
        // Clamping is what upstream leans on at the frame edge, where taps
        // reach past the last texel.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fsr1"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fsr1"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let target = format.remove_srgb_suffix();
        let pipeline = |label: &str, entry: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target,
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
            })
        };

        let constants = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fsr1 constants"),
            size: size_of::<Constants>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            easu: pipeline("fsr1 easu", "fs_easu"),
            rcas: pipeline("fsr1 rcas", "fs_rcas"),
            layout,
            sampler,
            constants,
            written: None,
            upscaled: None,
            sharpened: None,
            format: target,
            size: (0, 0),
        })
    }

    /// The sharpened frame, in **perceptual space** - the caller must decode it
    /// before grading. `None` until the first [`render`](Self::render).
    #[must_use]
    pub fn output(&self) -> Option<&wgpu::TextureView> {
        self.sharpened.as_ref().map(|target| &target.view)
    }

    /// The texture behind [`output`](Self::output), for a readback.
    #[must_use]
    pub fn output_texture(&self) -> Option<&wgpu::Texture> {
        self.sharpened.as_ref().map(|target| &target.texture)
    }

    /// Runs EASU and then RCAS, resizing the intermediates if the output moved.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: Frame<'_>,
    ) {
        let Frame {
            source,
            input,
            output,
            sharpness,
        } = frame;
        self.resize(device, output);

        // Two floats and a handful of ratios, moved by a menu row and by a
        // window resize. Uploaded only when one of them actually changes.
        let wanted = Constants::new(input, output, sharpness);
        if self.written != Some(wanted) {
            queue.write_buffer(&self.constants, 0, bytemuck::bytes_of(&wanted));
            self.written = Some(wanted);
        }

        let (Some(upscaled), Some(sharpened)) = (&self.upscaled, &self.sharpened) else {
            return;
        };

        // EASU reads the scene, RCAS reads EASU. Neither loads its target, so
        // both clear - a `Load` would be a read of memory every texel is about
        // to be written over.
        let source_bind_group = self.bind_group(device, source);
        pass(encoder, "fsr1 easu", &upscaled.view).run(&self.easu, &source_bind_group);
        pass(encoder, "fsr1 rcas", &sharpened.view).run(&self.rcas, &upscaled.bind_group);
    }

    /// Rebuilds both intermediates when the presentation size moves.
    fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.size == size && self.upscaled.is_some() {
            return;
        }
        self.size = size;
        self.upscaled = Some(self.target(device, size, "fsr1 upscaled"));
        self.sharpened = Some(self.target(device, size, "fsr1 sharpened"));
    }

    fn target(&self, device: &wgpu::Device, size: (u32, u32), label: &str) -> Target {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            // `COPY_SRC` so a test - and, later, a capture - can read the
            // result back. Neither pass ever copies, so this costs nothing but
            // the flag.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.bind_group(device, &view);
        Target {
            texture,
            view,
            bind_group,
        }
    }

    fn bind_group(&self, device: &wgpu::Device, view: &wgpu::TextureView) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fsr1"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.constants.as_entire_binding(),
                },
            ],
        })
    }
}

/// One fullscreen pass over `view`, ready to be given a pipeline and its input.
fn pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    label: &str,
    view: &wgpu::TextureView,
) -> Pass<'a> {
    Pass(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
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
    }))
}

struct Pass<'a>(wgpu::RenderPass<'a>);

impl Pass<'_> {
    fn run(mut self, pipeline: &wgpu::RenderPipeline, bind_group: &wgpu::BindGroup) {
        self.0.set_pipeline(pipeline);
        self.0.set_bind_group(0, Some(bind_group), &[]);
        self.0.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_easu_constants_are_upstream_s_for_a_two_times_upscale() {
        let c = Constants::new((960, 540), (1920, 1080), Sharpness::DEFAULT);
        // Output pixel to input pixel is exactly one half in both axes, and the
        // half-texel offset upstream subtracts is -0.25 at this ratio.
        assert_eq!(c.con0, [0.5, 0.5, -0.25, -0.25]);
        // Reciprocal input size, and the first gather centre one texel right
        // and one texel *up* - the negative is upstream's, not a slip.
        let (rx, ry) = (1.0 / 960.0, 1.0 / 540.0);
        assert_eq!(c.con1, [rx, ry, rx, -ry]);
        assert_eq!(c.con2, [-rx, 2.0 * ry, rx, 2.0 * ry]);
        assert_eq!(c.con3, [0.0, 4.0 * ry, 0.0, 0.0]);
    }

    #[test]
    fn a_one_to_one_scale_maps_output_pixels_onto_input_pixels() {
        let c = Constants::new((1280, 720), (1280, 720), Sharpness::DEFAULT);
        assert_eq!(c.con0[0], 1.0);
        assert_eq!(c.con0[1], 1.0);
        // Still the half-texel shift: EASU resolves at pixel centres.
        assert_eq!(c.con0[2], 0.0);
        assert_eq!(c.con0[3], 0.0);
    }

    #[test]
    fn sharpness_counts_stops_downward_and_off_is_a_zero_lobe() {
        // Zero stops is maximum sharpening, and each whole stop halves it.
        assert_eq!(Sharpness::stops(0.0).factor(), 1.0);
        assert_eq!(Sharpness::stops(1.0).factor(), 0.5);
        assert_eq!(Sharpness::stops(2.0).factor(), 0.25);
        // Out of range in either direction lands on the documented limits
        // rather than on an lobe that produces unnatural results.
        assert_eq!(Sharpness::stops(-3.0).factor(), 1.0);
        assert_eq!(Sharpness::stops(9.0).factor(), 0.25);
        // `OFF` multiplies the lobe by exactly zero. That is *not* the same as
        // RCAS becoming a pass-through - see the constant's docs - but the
        // factor itself is exact.
        assert_eq!(Sharpness::OFF.factor(), 0.0);
    }

    /// Compiles both shaders and runs both passes on a real device.
    ///
    /// The arithmetic above is CPU-side and says nothing about whether the WGSL
    /// parses, whether `textureGather`'s component argument is accepted, or
    /// whether the two pipelines agree with their bind group layout - all of
    /// which are runtime failures in wgpu, not build ones.
    ///
    /// **Skips when there is no adapter**, so a green CI run is not evidence
    /// that it ran. Run it locally on real hardware.
    #[test]
    fn both_passes_build_and_draw_on_a_real_device() {
        let instance = wgpu::Instance::default();
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
            eprintln!("no GPU adapter: skipping");
            return;
        };
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("requesting the device");

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let scene = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene"),
            size: wgpu::Extent3d {
                width: 160,
                height: 90,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            // The non-sRGB twin FSR 1 reads through - see the module docs.
            view_formats: &[format.remove_srgb_suffix()],
        });
        let source = scene.create_view(&wgpu::TextureViewDescriptor {
            format: Some(format.remove_srgb_suffix()),
            ..Default::default()
        });

        let mut fsr = Fsr1::new(&device, format).expect("building the pipelines");
        assert!(fsr.output().is_none(), "no output before the first render");

        let mut encoder = device.create_command_encoder(&Default::default());
        fsr.render(
            &device,
            &queue,
            &mut encoder,
            Frame {
                source: &source,
                input: (160, 90),
                output: (320, 180),
                sharpness: Sharpness::DEFAULT,
            },
        );
        queue.submit(Some(encoder.finish()));
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("draining the queue");

        assert!(fsr.output().is_some(), "the sharpened target must exist");
    }

    /// Upscales a known picture and reads the result back.
    ///
    /// A pipeline that builds and draws proves only that nothing was rejected;
    /// it would pass just as happily on a shader that wrote black everywhere,
    /// which is the failure a botched gather ordering or a wrong constant
    /// actually produces. This asserts on the picture.
    ///
    /// The input is a hard vertical edge - black left half, white right half -
    /// which is the case an edge-adaptive resampler exists for, and the one
    /// where it must visibly differ from a bilinear stretch.
    ///
    /// **What this does not prove.** A vertical edge is symmetric under a
    /// swapped `textureGather` component ordering, so a gather mix-up can
    /// survive it. This asserts that the port produces a correct picture rather
    /// than garbage, not that it is EASU specifically - a bilinear stretch of
    /// this input would also pass. A diagonal edge is the case that
    /// discriminates gather ordering, and is the test to add next.
    ///
    /// **Skips when there is no adapter.** Run it locally on real hardware.
    #[test]
    fn upscaling_a_hard_edge_keeps_the_edge_and_the_two_flat_sides() {
        const IN: (u32, u32) = (64, 64);
        const OUT: (u32, u32) = (128, 128);

        let instance = wgpu::Instance::default();
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
            eprintln!("no GPU adapter: skipping");
            return;
        };
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("requesting the device");

        // `Rgba8Unorm` throughout: this is a test about resampling arithmetic,
        // and an sRGB round trip would put a transfer function between what is
        // written and what is asserted for no gain.
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let scene = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("edge"),
            size: wgpu::Extent3d {
                width: IN.0,
                height: IN.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mut pixels = vec![0u8; (IN.0 * IN.1 * 4) as usize];
        for y in 0..IN.1 {
            for x in 0..IN.0 {
                let value = if x < IN.0 / 2 { 0 } else { 255 };
                let at = ((y * IN.0 + x) * 4) as usize;
                pixels[at..at + 4].copy_from_slice(&[value, value, value, 255]);
            }
        }
        queue.write_texture(
            scene.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(IN.0 * 4),
                rows_per_image: Some(IN.1),
            },
            scene.size(),
        );
        let source = scene.create_view(&wgpu::TextureViewDescriptor::default());

        let mut fsr = Fsr1::new(&device, format).expect("building the pipelines");
        let mut encoder = device.create_command_encoder(&Default::default());
        fsr.render(
            &device,
            &queue,
            &mut encoder,
            Frame {
                source: &source,
                input: IN,
                output: OUT,
                sharpness: Sharpness::DEFAULT,
            },
        );

        let unpadded = (OUT.0 * 4) as usize;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded = unpadded.div_ceil(align) * align;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (padded * OUT.1 as usize) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            fsr.output_texture().expect("an output").as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded as u32),
                    rows_per_image: Some(OUT.1),
                },
            },
            wgpu::Extent3d {
                width: OUT.0,
                height: OUT.1,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("draining the queue");
        let mapped = readback.slice(..).get_mapped_range().expect("mapping");

        let row = OUT.1 / 2;
        let at = |x: u32| mapped[padded * row as usize + (x * 4) as usize];

        // The flat sides survive. An upscaler that got its gather ordering or
        // its constants wrong smears or shifts the edge, and either shows up
        // here as a mid-grey well away from the seam.
        assert_eq!(at(4), 0, "the black side is not black");
        assert_eq!(at(OUT.0 - 5), 255, "the white side is not white");

        // The edge lands where the geometry says it should - halfway - and is
        // genuinely sharp: at twice the scale a bilinear stretch spreads a hard
        // edge over about two output texels, and EASU plus RCAS must not do
        // worse than that. Counting the texels that are neither side tests the
        // thing the algorithm is for.
        let middle = (0..OUT.0).filter(|&x| at(x) > 8 && at(x) < 247).count();
        assert!(
            middle <= 2,
            "the edge spread over {middle} texels, which is not an upscale of a hard edge"
        );
        let first_white = (0..OUT.0).find(|&x| at(x) > 247).expect("a white side");
        assert!(
            first_white.abs_diff(OUT.0 / 2) <= 2,
            "the edge landed at {first_white}, not near {}",
            OUT.0 / 2
        );

        drop(mapped);
        readback.unmap();
    }

    /// RCAS must not overshoot, which is the artifact that would hurt text.
    ///
    /// A sharpener's characteristic failure is ringing: a bright halo just
    /// outside a dark edge and a dark one just inside it. On 480x272-era
    /// paletted sprite art and on glyphs lifted from a coverage atlas - which
    /// is what the menus and the HUD are made of - that reads as fringing, and
    /// it is the specific reason to doubt a global upscaler setting.
    ///
    /// The hard black-and-white edge in the test above cannot show it, because
    /// overshoot past 0 and 255 is clamped away by the format. A **mid-range**
    /// edge can: anything darker than the dark side or brighter than the bright
    /// side is ringing, with nowhere to hide.
    ///
    /// **Some overshoot is what sharpening *is*, and RCAS does not promise
    /// otherwise**: its limiters bound the result to `[0, 1]`, not to the local
    /// neighbourhood. So this pins the magnitude rather than asserting zero.
    /// Measured across a 128-level edge: 16/255 at maximum sharpening and
    /// 10/255 at the shipped default - 12.5 % and 7.8 % of the edge contrast.
    /// That is a well-behaved sharpener. This test exists to catch the day it
    /// stops being one - a botched limiter or lobe would blow well past this.
    ///
    /// **Skips when there is no adapter.**
    #[test]
    fn sharpening_a_mid_range_edge_rings_only_as_much_as_sharpening_must() {
        // A fifth of the edge contrast. Comfortably above what RCAS does and
        // far below what a broken limiter would.
        const BUDGET: u8 = (BRIGHT - DARK) / 5;
        let Some(max) = ringing(0.0) else {
            eprintln!("no GPU adapter: skipping");
            return;
        };
        let default = ringing(0.2).expect("an adapter, having just had one");
        for (what, (under, over)) in [("maximum", max), ("default", default)] {
            assert!(
                under <= BUDGET && over <= BUDGET,
                "at {what} sharpening: undershoot {under} below {DARK}, \
                 overshoot {over} above {BRIGHT}, budget {BUDGET}"
            );
        }
        // And sharpening less must ring less, or the scale is upside down.
        assert!(default.0 <= max.0 && default.1 <= max.1);
    }

    const DARK: u8 = 64;
    const BRIGHT: u8 = 192;

    /// The worst undershoot and overshoot across a mid-range edge, or `None`
    /// with no adapter.
    fn ringing(stops: f32) -> Option<(u8, u8)> {
        const IN: (u32, u32) = (64, 64);
        const OUT: (u32, u32) = (128, 128);

        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;

        let format = wgpu::TextureFormat::Rgba8Unorm;
        let scene = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mid edge"),
            size: wgpu::Extent3d {
                width: IN.0,
                height: IN.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mut pixels = vec![0u8; (IN.0 * IN.1 * 4) as usize];
        for y in 0..IN.1 {
            for x in 0..IN.0 {
                let value = if x < IN.0 / 2 { DARK } else { BRIGHT };
                let at = ((y * IN.0 + x) * 4) as usize;
                pixels[at..at + 4].copy_from_slice(&[value, value, value, 255]);
            }
        }
        queue.write_texture(
            scene.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(IN.0 * 4),
                rows_per_image: Some(IN.1),
            },
            scene.size(),
        );
        let source = scene.create_view(&wgpu::TextureViewDescriptor::default());

        let mut fsr = Fsr1::new(&device, format).expect("building the pipelines");
        let mut encoder = device.create_command_encoder(&Default::default());
        fsr.render(
            &device,
            &queue,
            &mut encoder,
            Frame {
                source: &source,
                input: IN,
                output: OUT,
                // Maximum sharpening: if anything rings, it rings here.
                sharpness: Sharpness::stops(stops),
            },
        );

        let unpadded = (OUT.0 * 4) as usize;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded = unpadded.div_ceil(align) * align;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (padded * OUT.1 as usize) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            fsr.output_texture().expect("an output").as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded as u32),
                    rows_per_image: Some(OUT.1),
                },
            },
            wgpu::Extent3d {
                width: OUT.0,
                height: OUT.1,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("draining the queue");
        let mapped = readback.slice(..).get_mapped_range().expect("mapping");

        let row = OUT.1 / 2;
        let mut worst = (0u8, 0u8);
        for x in 0..OUT.0 {
            let v = mapped[padded * row as usize + (x * 4) as usize];
            worst.0 = worst.0.max(DARK.saturating_sub(v));
            worst.1 = worst.1.max(v.saturating_sub(BRIGHT));
        }
        drop(mapped);
        readback.unmap();
        Some(worst)
    }

    #[test]
    fn a_degenerate_size_cannot_divide_by_zero() {
        let c = Constants::new((0, 0), (0, 0), Sharpness::DEFAULT);
        assert!(c.con0.iter().all(|v| v.is_finite()), "{:?}", c.con0);
        assert!(c.con1.iter().all(|v| v.is_finite()), "{:?}", c.con1);
    }
}
