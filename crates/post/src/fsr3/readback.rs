//! Running the ported passes over a hand-built depth buffer and reading an
//! intermediate back, which is the only instrument this port has until
//! [`super::Pass::Accumulate`] lands and there is an actual output to look at.
//!
//! Its own file rather than more of `tests.rs`, under the 1,000-line rule in
//! `scripts/check-file-size.py` and because the harness below is a fixture the
//! next pass's tests will reuse rather than a test in itself.

use super::*;

/// A depth buffer, and everything the passes need alongside it.
///
/// The device and queue are kept because a readback needs them after the
/// dispatch has already happened - the pass ran in [`Scene::run`], and every
/// `read_r32` after it is a second submission against the same device.
pub(super) struct Scene {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub fsr3: Fsr3,
}

/// The camera the readbacks project through - the race's own near and far, so
/// that `get_view_space_depth` is exercised over the range it really sees.
pub(super) const CAMERA: Camera = Camera {
    near: 0.1,
    far: 1000.0,
    fov_y: std::f32::consts::FRAC_PI_4,
};

/// One frame of the fixture: a depth buffer and a colour buffer, both
/// row-major at the render extent.
///
/// The scene is otherwise still - the velocity attachment is left at zero -
/// so a two-frame run differs only in what these hold, which is what makes a
/// temporal pass's output attributable to one thing.
#[derive(Debug, Clone, Copy)]
pub(super) struct Input<'a> {
    pub depth: &'a [f32],
    /// Linear RGB per texel, three floats each.
    pub colour: &'a [[f32; 3]],
}

impl Scene {
    /// Runs every ported pass once per frame in `frames`, in order, on one
    /// device. `None` when there is no adapter.
    ///
    /// **More than one frame, because half of what this port does is temporal.**
    /// The first frame has no history, so a pass that compares against the
    /// previous frame reads whatever the ping-pong's unwritten half holds and
    /// nothing about its output is attributable. Two frames of a *still* scene
    /// is the sharpest fixture available: whatever the shading-change pyramid
    /// says then, it should say nothing changed.
    ///
    /// **The depth texture is a real `Depth32Float`,** not a colour format
    /// standing in for one: binding a depth view as an unfilterable float is
    /// exactly the arrangement that would fail, and a colour texture would not
    /// exercise it. It is filled by rendering rather than by `write_texture`,
    /// because WebGPU does not allow a buffer-to-texture copy into a depth
    /// format - the fill is a fullscreen pass writing `@builtin(frag_depth)`
    /// from a storage buffer, which is the same picture by a longer road.
    ///
    /// The colour texture is `Rgba32Float` where the game's scene target is an
    /// sRGB one. The binding is identical - every pass here `textureLoad`s it -
    /// and it means the fixture can write exact `f32`s rather than encode
    /// halves, so an expected value is a number rather than a rounding.
    pub(super) fn run(render: (u32, u32), frames: &[Input<'_>]) -> Option<Self> {
        Self::run_with(render, frames, false)
    }

    /// [`Scene::run`], with the real Halton offsets fed through
    /// `Dispatch::jitter` when `jittered`.
    ///
    /// **Only the resolve can use this.** Every other pass's fixture wants a
    /// scene that is genuinely still, and a moving sub-pixel offset makes
    /// consecutive frames differ for a reason that has nothing to do with what
    /// is being tested. `accumulate` is the opposite case: without jitter there
    /// is one sample per pixel per frame and there is nothing to reconstruct
    /// *from*, so a still fixture would report a temporal upscaler and a blit
    /// as identical.
    pub(super) fn run_with(
        render: (u32, u32),
        frames: &[Input<'_>],
        jittered: bool,
    ) -> Option<Self> {
        Self::run_resetting(render, frames, jittered, |index| index == 0)
    }

    /// [`Scene::run_with`], with `reset` deciding which frames throw the
    /// history away rather than only the first.
    ///
    /// **One `Fsr3` and one set of targets across every frame**, which is the
    /// point: a reset into targets that still hold a previous sequence is the
    /// race-restart case, and a fresh allocation - zero-initialised by wgpu -
    /// cannot show what a reset fails to wipe.
    pub(super) fn run_resetting(
        render: (u32, u32),
        frames: &[Input<'_>],
        jittered: bool,
        reset: impl Fn(usize) -> bool,
    ) -> Option<Self> {
        let texels = (render.0 * render.1) as usize;
        let upscale = (render.0 * 2, render.1 * 2);

        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).ok()?;

        let make = |label, format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: render.0,
                    height: render.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let depth_texture = make(
            "fsr3 test depth",
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let depth_view = depth_texture.create_view(&Default::default());
        let colour_texture = make(
            "fsr3 test colour",
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        let colour = colour_texture.create_view(&Default::default());
        let velocity = make(
            "fsr3 test velocity",
            oag_gpu::formats::VELOCITY_FORMAT,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .create_view(&Default::default());

        let mut fsr3 = Fsr3::new(&device).expect("the FSR 3.1 shaders must compile");

        for (index, input) in frames.iter().enumerate() {
            assert_eq!(input.depth.len(), texels);
            assert_eq!(input.colour.len(), texels);

            fill_depth(&device, &queue, &depth_view, render, input.depth);
            let rgba: Vec<f32> = input
                .colour
                .iter()
                .flat_map(|[r, g, b]| [*r, *g, *b, 1.0])
                .collect();
            queue.write_texture(
                colour_texture.as_image_copy(),
                bytemuck::cast_slice(&rgba),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(render.0 * 16),
                    rows_per_image: Some(render.1),
                },
                colour_texture.size(),
            );

            let mut encoder = device.create_command_encoder(&Default::default());
            fsr3.render(
                &device,
                &queue,
                &mut encoder,
                Frame {
                    colour: &colour,
                    depth: &depth_view,
                    velocity: &velocity,
                    dispatch: Dispatch {
                        render,
                        max_render: render,
                        upscale,
                        // Zero unless asked for, so that nothing in a fixture
                        // depends on which phase of the sequence it happened to
                        // land on - and so that a still scene really is still,
                        // where a moving offset would make consecutive frames
                        // differ.
                        jitter: if jittered {
                            crate::jitter::offset_pixels(
                                index as u32,
                                crate::jitter::phases(render.0, upscale.0),
                            )
                        } else {
                            (0.0, 0.0)
                        },
                        phase_count: crate::jitter::phases(render.0, upscale.0),
                        camera: CAMERA,
                        delta_time: 1.0 / 60.0,
                        reset: reset(index),
                        sample_count: 1,
                        sharpness: crate::fsr1::Sharpness::DEFAULT,
                    },
                },
                None,
            );
            queue.submit([encoder.finish()]);
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("draining the queue");
        }

        Some(Self {
            device,
            queue,
            fsr3,
        })
    }

    /// A mip level of `texture`, copied back and unpadded to `bytes` per
    /// texel, row-major.
    ///
    /// The one place the row-alignment dance lives, because getting it wrong
    /// reads a texel from the padding and produces a plausible wrong number
    /// rather than an error.
    fn read_bytes(&self, texture: &wgpu::Texture, mip: u32, bytes: usize) -> Vec<u8> {
        let size = texture
            .size()
            .mip_level_size(mip, wgpu::TextureDimension::D2);
        let unpadded = size.width as usize * bytes;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded = unpadded.div_ceil(align) * align;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fsr3 readback"),
            size: (padded * size.height as usize) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: mip,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded as u32),
                    rows_per_image: Some(size.height),
                },
            },
            size,
        );
        self.queue.submit([encoder.finish()]);
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("draining the queue");
        let mapped = readback.slice(..).get_mapped_range().expect("mapping");
        let mut out = Vec::with_capacity(size.height as usize * unpadded);
        for y in 0..size.height as usize {
            out.extend_from_slice(&mapped[y * padded..y * padded + unpadded]);
        }
        drop(mapped);
        readback.unmap();
        out
    }

    /// One `R32Float` intermediate, read back as `f32`s row-major.
    pub(super) fn read_r32(&self, texture: &wgpu::Texture) -> Vec<f32> {
        self.read_bytes(texture, 0, 4)
            .as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| f32::from_le_bytes(*bytes))
            .collect()
    }

    /// All four channels of an `Rgba8Unorm` intermediate - which for
    /// `dilated_reactive_masks` is four independent answers, not a colour.
    pub(super) fn read_rgba_unorm(&self, texture: &wgpu::Texture) -> Vec<[f32; 4]> {
        self.read_bytes(texture, 0, 4)
            .as_chunks::<4>()
            .0
            .iter()
            .map(|texel| texel.map(|byte| f32::from(byte) / 255.0))
            .collect()
    }

    /// One `Rgba8Unorm` intermediate's red channel, back as `0..1` floats.
    ///
    /// The three targets that live in this format - `accumulation`,
    /// `shading_change`, `new_locks` - are the ones upstream holds at
    /// `R8_UNORM`, so a readback here is quantised to the same 256 levels
    /// upstream's is, and an expectation has to be given a tolerance
    /// accordingly.
    pub(super) fn read_unorm(&self, texture: &wgpu::Texture) -> Vec<f32> {
        self.read_bytes(texture, 0, 4)
            .into_iter()
            .step_by(4)
            .map(|byte| f32::from(byte) / 255.0)
            .collect()
    }

    /// All four channels of an `Rgba16Float` intermediate - which for
    /// `luma_history` is four *frames*, N-1 through N-4, not a colour.
    pub(super) fn read_rgba16(&self, texture: &wgpu::Texture) -> Vec<[f32; 4]> {
        self.read_bytes(texture, 0, 8)
            .as_chunks::<8>()
            .0
            .iter()
            .map(|texel| {
                std::array::from_fn(|channel| {
                    let at = channel * 2;
                    f16_to_f32(u16::from_le_bytes(texel[at..at + 2].try_into().unwrap()))
                })
            })
            .collect()
    }

    /// One `Rgba16Float` intermediate's first two channels at `mip`, read back
    /// as pairs row-major - which is what the pyramid holds.
    pub(super) fn read_rg16(&self, texture: &wgpu::Texture, mip: u32) -> Vec<(f32, f32)> {
        self.read_bytes(texture, mip, 8)
            .as_chunks::<8>()
            .0
            .iter()
            .map(|texel| {
                let half = |at: usize| {
                    f16_to_f32(u16::from_le_bytes(texel[at..at + 2].try_into().unwrap()))
                };
                (half(0), half(2))
            })
            .collect()
    }

    /// `GetViewSpaceDepthInMeters` on the CPU, from the same two constants the
    /// shader reads - so the check is against the transform this frame was
    /// dispatched with rather than against a second derivation of it.
    pub(super) fn view_space_metres(&self, device_depth: f32) -> f32 {
        let c = self.fsr3.constants().expect("a dispatched frame");
        let metres = c.device_to_view_depth[1] / (device_depth - c.device_to_view_depth[0]);
        metres.min(65504.0)
    }
}

/// An IEEE binary16 back to an `f32`.
///
/// Written out rather than pulled in as a dependency: `half` is already in the
/// lock file under wgpu, but adding it to this crate's manifest for eleven
/// lines of test-only decoding is a dependency the build would carry forever.
///
/// Every case is handled because the pyramid legitimately holds all of them - a
/// zero where nothing changed, a subnormal where the change is tiny, and an
/// infinity if a later pass ever writes one.
fn f16_to_f32(bits: u16) -> f32 {
    let sign = u32::from(bits & 0x8000) << 16;
    let exponent = u32::from((bits >> 10) & 0x1f);
    let mantissa = u32::from(bits & 0x3ff);
    match exponent {
        0 if mantissa == 0 => f32::from_bits(sign),
        // Subnormal: renormalise by shifting the mantissa up until its leading
        // one falls out, decrementing the exponent as it goes.
        0 => {
            let shift = mantissa.leading_zeros() - 21;
            let exponent = 127 - 15 - shift;
            let mantissa = (mantissa << (shift + 1)) & 0x3ff;
            f32::from_bits(sign | (exponent << 23) | (mantissa << 13))
        }
        0x1f => f32::from_bits(sign | 0x7f80_0000 | (mantissa << 13)),
        _ => f32::from_bits(sign | ((exponent + 127 - 15) << 23) | (mantissa << 13)),
    }
}

/// Fills a depth attachment from a storage buffer of per-texel values.
///
/// A fullscreen triangle writing `@builtin(frag_depth)`, because WebGPU has no
/// buffer-to-texture copy into a depth format. `DepthCompare::Always` and a
/// write mask, so the value written is exactly the one in the buffer.
fn fill_depth(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    size: (u32, u32),
    depth: &[f32],
) {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("fsr3 test depth fill"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!(concat!(env!("OUT_DIR"), "/fill_depth.wgsl")).into(),
        ),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("fsr3 test depth fill"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
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
    let values = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fsr3 test depth values"),
        size: (depth.len() * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&values, 0, bytemuck::cast_slice(depth));
    let extent = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fsr3 test depth extent"),
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&extent, 0, bytemuck::cast_slice(&[size.0, size.1, 0, 0]));
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("fsr3 test depth fill"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: values.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: extent.as_entire_binding(),
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("fsr3 test depth fill"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("fsr3 test depth fill"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            targets: &[],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fsr3 test depth fill"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, Some(&group), &[]);
        pass.draw(0..3, 0..1);
    }
    queue.submit([encoder.finish()]);
}
