//! Fast Approximate Anti-Aliasing: one fullscreen pass, luma edge detection
//! then a directional blend. See [`fxaa.wesl`](../shaders/fxaa.wesl) for the shader
//! and [`crate`] for the colour-space convention it draws in.
//!
//! # Why this and not a transliteration
//!
//! [ADR-0012](../../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)
//! established transliterating a specific MIT-licensed upstream, attributed
//! and reproduced under `licences/`, as this project's route for a shader it
//! did not write - `fsr1.wesl` and `smaa.wesl` both are one. FXAA has no
//! single canonical upstream with licensing this project can clear the same
//! way: the widely distributed "FXAA 3.11" carries NVIDIA's own terms, and a
//! transliteration of it would need those cleared and reproduced exactly like
//! AMD's are. Rather than copy text this project cannot attribute correctly,
//! this is written from the technique's public description - luma-based edge
//! detection, then a blend across the detected edge - as its own
//! implementation.
//!
//! # What that costs
//!
//! A single quality tier: one 5-tap cross sample for edge detection, one
//! directional blend, no iterative search along the edge the way NVIDIA's own
//! shader performs to find where an edge ends. That is a real difference in
//! sharpness, not a bug - see [ADR-0013](../../../../docs/architecture/adr/0013-anti-aliasing-architecture.md)
//! for where this sits next to SMAA, which *is* a transliteration and keeps
//! more detail for roughly three times the cost.

use anyhow::Result;

/// The shader's uniform: the inverse render-target size the taps are spaced
/// by, the two edge thresholds, and the sub-rectangle of the source that was
/// actually drawn. `repr(C)`, and eight floats rather than four since the
/// rectangle joined it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    inv_size: [f32; 2],
    threshold: f32,
    relative_threshold: f32,
    /// What to multiply the fullscreen triangle's `0..1` coordinate by to land
    /// inside the drawn rectangle - `viewport / size`.
    uv_scale: [f32; 2],
    /// The furthest a tap may reach, half a texel inside the drawn rectangle's
    /// outer edge. Without it a tap at the edge reads the region outside the
    /// viewport, which holds whatever the clear left there.
    uv_max: [f32; 2],
}

impl Constants {
    /// The minimum local contrast that counts as an edge at all. Below this,
    /// on a flat plain, no amount of relative contrast changes the answer.
    const THRESHOLD: f32 = 0.0625;
    /// The same test again as a fraction of the local brightness, so a
    /// shadow does not need the same absolute contrast a highlight does to
    /// register as an edge worth blending.
    const RELATIVE_THRESHOLD: f32 = 0.125;

    fn new(viewport: (u32, u32), size: (u32, u32)) -> Self {
        let (w, h) = (size.0.max(1) as f32, size.1.max(1) as f32);
        let (scale, max) = super::sub_rectangle(viewport, size);
        Self {
            inv_size: [1.0 / w, 1.0 / h],
            threshold: Self::THRESHOLD,
            relative_threshold: Self::RELATIVE_THRESHOLD,
            uv_scale: scale,
            uv_max: max,
        }
    }
}

/// One frame's worth of input to [`Fxaa::render`].
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    /// A **non-sRGB** view of the scene, for the same reason `fsr1::Frame::source`
    /// is: the luma test reasons about encoded values, the way an eye weighs
    /// them, and a linear view would misjudge which edges matter in shadow.
    pub source: &'a wgpu::TextureView,
    /// The size of the resource behind `source`, and of the output this
    /// resizes to. **The allocation, not the drawn rectangle** - keying the
    /// target off this is what stops a moving render extent rebuilding it
    /// several times a second.
    pub size: (u32, u32),
    /// The rectangle of `source` that was drawn, `<= size` on both axes. The
    /// pass draws into the same rectangle of its own output.
    pub viewport: (u32, u32),
}

/// The one pipeline, its uniform, and the target it resizes to match `Frame::size`.
#[derive(Debug)]
pub struct Fxaa {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    constants: wgpu::Buffer,
    written: Option<Constants>,
    output: Option<Target>,
    /// Non-sRGB, matching `Frame::source`: the blend must not decode or
    /// re-encode a value it is only passing through untouched.
    format: wgpu::TextureFormat,
    size: (u32, u32),
}

#[derive(Debug)]
struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Fxaa {
    /// Builds the pipeline against a surface `format`.
    ///
    /// The output target is built at the first [`render`](Self::render),
    /// which is the first time a size is known.
    ///
    /// # Errors
    ///
    /// Propagates a shader that will not compile, which is a build-time
    /// mistake rather than anything a player can cause.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fxaa"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/fxaa.wgsl")).into(),
            ),
        });

        let layout = super::fullscreen_layout(device, "fxaa");

        // Linear, unlike FSR 1's nearest-and-gather sampler: the blend tap
        // reads one texel in a cross pattern and wants ordinary bilinear
        // filtering there, not EASU's own weighting scheme.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fxaa"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fxaa"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let target_format = format.remove_srgb_suffix();
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fxaa"),
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
                    format: target_format,
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

        let constants = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fxaa constants"),
            size: size_of::<Constants>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            pipeline,
            layout,
            sampler,
            constants,
            written: None,
            output: None,
            format: target_format,
            size: (0, 0),
        })
    }

    /// The blended frame, in **perceptual space** - the caller must decode
    /// it before grading. `None` until the first [`render`](Self::render).
    #[must_use]
    pub fn output(&self) -> Option<&wgpu::TextureView> {
        self.output.as_ref().map(|target| &target.view)
    }

    /// The texture behind [`output`](Self::output), for a readback.
    #[must_use]
    pub fn output_texture(&self) -> Option<&wgpu::Texture> {
        self.output.as_ref().map(|target| &target.texture)
    }

    /// Runs the one pass, resizing the target if `frame.size` moved.
    ///
    /// The output is `frame.size` with `frame.viewport` drawn into its
    /// top-left corner, so a caller reading it back owes the same sub-rectangle
    /// treatment the scene target does.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: Frame<'_>,
    ) {
        self.resize(device, frame.size);

        let wanted = Constants::new(frame.viewport, frame.size);
        if self.written != Some(wanted) {
            queue.write_buffer(&self.constants, 0, bytemuck::bytes_of(&wanted));
            self.written = Some(wanted);
        }

        let Some(output) = &self.output else { return };

        let bind_group = self.bind_group(device, frame.source);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fxaa"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output.view,
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
        pass.set_pipeline(&self.pipeline);
        pass.set_viewport(
            0.0,
            0.0,
            frame.viewport.0.max(1) as f32,
            frame.viewport.1.max(1) as f32,
            0.0,
            1.0,
        );
        pass.set_bind_group(0, Some(&bind_group), &[]);
        pass.draw(0..3, 0..1);
    }

    fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.size == size && self.output.is_some() {
            return;
        }
        self.size = size;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fxaa output"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            // `COPY_SRC` so a test can read the result back, the same reason
            // `fsr1::Fsr1`'s intermediates carry it.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.output = Some(Target { texture, view });
    }

    fn bind_group(&self, device: &wgpu::Device, source: &wgpu::TextureView) -> wgpu::BindGroup {
        oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("fxaa"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
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

#[cfg(test)]
mod tests;
