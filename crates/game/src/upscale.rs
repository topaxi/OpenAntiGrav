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
//! behaviour for. `graphics.window_size` is a window property and absolute;
//! this is a graphics one and relative, and keeping them different kinds is what
//! stops them being conflated.
//!
//! # What it is measured against
//!
//! The **viewport rectangle**, not the surface. So the scale means the same
//! thing whatever [`Aspect`](crate::display::Aspect) is set to - at 50 % on a
//! pillarboxed 4:3 window, the game is drawn at half the 4:3 area rather than
//! half a window it was never using. The bars are drawn by the blit pass
//! clearing the surface, so they cost no offscreen pixels at all.

use anyhow::Result;

use crate::display::Scale;

/// The offscreen target and the pipeline that puts it on screen.
pub struct Framebuffer {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
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

        let (texture, view, bind_group) = target(device, &layout, &sampler, format, size);
        Ok(Self {
            pipeline,
            layout,
            sampler,
            texture,
            view,
            bind_group,
            size,
            format,
        })
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
        let (texture, view, bind_group) =
            target(device, &self.layout, &self.sampler, self.format, size);
        self.texture = texture;
        self.view = view;
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
    pub fn present(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        surface: &wgpu::TextureView,
        rect: (f32, f32, f32, f32),
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
        pass.set_bind_group(0, Some(&self.bind_group), &[]);
        pass.draw(0..3, 0..1);
    }
}

fn target(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    format: wgpu::TextureFormat,
    size: (u32, u32),
) -> (wgpu::Texture, wgpu::TextureView, wgpu::BindGroup) {
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
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("upscale"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    (texture, view, bind_group)
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
