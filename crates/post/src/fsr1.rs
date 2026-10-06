//! AMD FidelityFX Super Resolution 1: EASU, then RCAS.
//!
//! Two fullscreen passes. The first, EASU, resamples the scene target up to the
//! presentation rectangle with an edge-adaptive twelve-tap kernel; the second,
//! RCAS, sharpens the result back up without the ringing an unconditional
//! sharpen would add. Both are transliterated from AMD's MIT-licensed
//! `ffx_fsr1.h` - see [`fsr1.wesl`](../shaders/fsr1.wesl) and
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
    /// Upstream takes the drawn rectangle and the resource holding it
    /// separately - `FsrEasuCon(con0..con3, inputViewportInPixelsX/Y,
    /// inputSizeInPixelsX/Y, outputSizeInPixelsX/Y)` - and the split between
    /// them is exact: `con0` is the viewport-to-output ratio and nothing else,
    /// while `con1`..`con3` are gather offsets in texels *of the resource*.
    /// Since [ADR-0037] the scene target is allocated at the `render_scale`
    /// ceiling and a smaller rectangle of it may be drawn, so the two are no
    /// longer the same number and the port keeps them apart the way
    /// `ffx_fsr1.h` does.
    ///
    /// [ADR-0037]: ../../../../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md
    fn new(
        viewport: (u32, u32),
        size: (u32, u32),
        output: (u32, u32),
        sharpness: Sharpness,
    ) -> Self {
        let (vw, vh) = (viewport.0.max(1) as f32, viewport.1.max(1) as f32);
        let (iw, ih) = (size.0.max(1) as f32, size.1.max(1) as f32);
        let (ow, oh) = (output.0.max(1) as f32, output.1.max(1) as f32);
        Self {
            // Output integer position to a pixel position in viewport.
            con0: [vw / ow, vh / oh, 0.5 * vw / ow - 0.5, 0.5 * vh / oh - 0.5],
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
    /// The rectangle of `source` that was actually drawn this frame -
    /// upstream's `inputViewportInPixels`. Equal to [`Frame::input`] unless a
    /// controller is varying the render extent.
    pub viewport: (u32, u32),
    /// The size of the resource behind `source` - upstream's
    /// `inputSizeInPixels`. This is the scene target's **allocation**, which a
    /// gather offset is measured in texels of.
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
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/fsr1.wgsl")).into(),
            ),
        });

        let layout = super::fullscreen_layout(device, "fsr1");

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
            viewport,
            input,
            output,
            sharpness,
        } = frame;
        self.resize(device, output);

        // Two floats and a handful of ratios, moved by a menu row and by a
        // window resize. Uploaded only when one of them actually changes.
        let wanted = Constants::new(viewport, input, output, sharpness);
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
        oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
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
            },
        )
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
mod tests;
