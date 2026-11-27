use super::*;

/// The law off `ghost.md`, at the points that pin its shape: nothing inside
/// 5 units, a linear ramp to half the far weight at 15, and the jump there.
#[test]
fn the_fade_is_invisible_close_ramps_to_half_and_jumps_at_fifteen() {
    assert_eq!(
        fade(0.0),
        Fade {
            weight: 0.0,
            glow: 0
        }
    );
    assert_eq!(
        fade(5.0),
        Fade {
            weight: 0.0,
            glow: 0
        }
    );
    let ten = fade(10.0);
    assert!((ten.weight - 5.0 * 0.55 / 20.0).abs() < 1e-6);
    assert_eq!(ten.glow, 63);
    let just_under = fade(14.999);
    assert!(just_under.weight < 0.276 && just_under.weight > 0.274);
    assert_eq!(just_under.glow, 127);
    assert_eq!(
        fade(15.0),
        Fade {
            weight: 0.55,
            glow: 255
        }
    );
    assert_eq!(
        fade(400.0),
        Fade {
            weight: 0.55,
            glow: 255
        }
    );
}

#[test]
fn the_static_offsets_are_hundredths_below_one() {
    let mut rng = Rng::new(3);
    for _ in 0..1_000 {
        for offset in static_offsets(&mut rng) {
            assert!((0.0..=0.99).contains(&offset));
            assert!((offset * 100.0 - (offset * 100.0).round()).abs() < 1e-4);
        }
    }
}

/// The three pipelines build against a race-shaped target - colour, velocity,
/// depth, 4x MSAA - on whatever adapter this machine has, and none of them
/// is rejected by validation. Skipped without an adapter.
#[test]
fn the_three_passes_build_against_a_race_target() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        return;
    };
    let Ok((device, queue)) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
    else {
        return;
    };
    let static_glow = oag_fx::exhaust::FlareTexture {
        width: 2,
        height: 2,
        rgba: vec![255; 16],
        mips: Vec::new(),
        lod_bias: 0.0,
    };
    for (format, stamps) in [
        (wgpu::TextureFormat::Rgba8Unorm, true),
        (wgpu::TextureFormat::Rgba16Float, false),
    ] {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let pipeline = Pipeline::new(
            &device,
            &queue,
            format,
            oag_gpu::formats::SceneStore::Computed,
            4,
            stamps,
            Some(&static_glow),
            -1.5,
        );
        let error = pollster::block_on(scope.pop());
        assert!(error.is_none(), "{format:?}: {error:?}");
        assert_eq!(pipeline.stamps_glow(), stamps);
        assert_eq!(pipeline.stamp.is_some(), stamps);
    }
}

/// The hull pass samples the hull's texture down its chain at the title's bias. Each level is
/// one flat colour: 64 texels over 8 pixels is level 3 (blue) on the sampler's own selection
/// and level 1.5 (half red, half green) at -1.5, either way cross-faded over black at
/// [`FAR_WEIGHT`]. Skipped without an adapter.
#[test]
fn the_hull_pass_takes_the_titles_bias() {
    let Some(biased) = hull_pixel(-1.5) else {
        return;
    };
    let plain = hull_pixel(0.0).expect("the adapter was just there");
    let [red, green, blue] = biased;
    assert!(
        (50..=90).contains(&red) && (50..=90).contains(&green) && blue < 10,
        "biased: {biased:?}"
    );
    assert!(plain[0] < 10 && plain[2] > 120, "unbiased: {plain:?}");
}

/// The centre pixel of a ghost hull covering an 8x8 target, its 64-texel texture carrying
/// a white base then red, green, blue repeating down its chain.
fn hull_pixel(lod_bias: f32) -> Option<[u8; 3]> {
    use oag_gpu::formats::VELOCITY_FORMAT;
    use oag_mesh::mesh_render::{DEPTH_FORMAT, material_bind_group_layout};
    use wgpu::util::DeviceExt as _;

    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let flat =
        |side: u32, rgb: [u8; 3]| [rgb[0], rgb[1], rgb[2], 255].repeat((side * side) as usize);
    let colours = [[255, 0, 0], [0, 255, 0], [0, 0, 255]];
    let mips: Vec<Vec<u8>> = (1..=6)
        .map(|level| flat(64 >> level, colours[(level - 1) % 3]))
        .collect();
    let texture = oag_gpu::texture::upload_rgba8_chain(
        &device,
        &queue,
        "hull",
        (64, 64),
        &flat(64, [255; 3]),
        &mips,
    )
    .create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    });
    let entries: Vec<wgpu::BindGroupEntry> = (0..7)
        .map(|binding| wgpu::BindGroupEntry {
            binding,
            resource: if binding == 1 {
                wgpu::BindingResource::Sampler(&sampler)
            } else {
                wgpu::BindingResource::TextureView(&texture)
            },
        })
        .collect();
    let material = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("hull"),
        layout: &material_bind_group_layout(&device),
        entries: &entries,
    });
    let corner = |x: f32, y: f32| GpuVertex {
        position: [x * 10.0, y * 10.0, -10.0],
        texcoord: [(x + 1.0) / 2.0, (1.0 - y) / 2.0],
        ..bytemuck::Zeroable::zeroed()
    };
    let vertices = [
        corner(-1.0, -1.0),
        corner(1.0, -1.0),
        corner(-1.0, 1.0),
        corner(1.0, -1.0),
        corner(1.0, 1.0),
        corner(-1.0, 1.0),
    ];
    let buffer = |contents: &[u8], usage| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents,
            usage,
        })
    };
    let hull = Hull {
        vertices: &buffer(bytemuck::cast_slice(&vertices), wgpu::BufferUsages::VERTEX),
        indices: &buffer(
            bytemuck::cast_slice(&[0u32, 1, 2, 3, 4, 5]),
            wgpu::BufferUsages::INDEX,
        ),
        textures: std::slice::from_ref(&material),
        draws: vec![(0..6, None)],
    };
    let store = oag_gpu::formats::SceneStore::Computed;
    let pipeline = Pipeline::new(&device, &queue, format, store, 1, false, None, lod_bias);
    let projection =
        oag_core::math::camera::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
    let fade = fade(NEAR + RAMP);
    pipeline.write(
        &queue,
        &Frame {
            view_projection: projection,
            model: oag_core::math::Mat4::IDENTITY,
            prev_mvp: projection,
            fade,
            offsets: [0.0; 2],
        },
    );

    let extent = wgpu::Extent3d {
        width: 8,
        height: 8,
        depth_or_array_layers: 1,
    };
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
            texture_binding_view_dimension: None,
        })
    };
    let attach = wgpu::TextureUsages::RENDER_ATTACHMENT;
    let colour = target(format, attach | wgpu::TextureUsages::COPY_SRC);
    let velocity = target(VELOCITY_FORMAT, attach).create_view(&Default::default());
    let depth = target(DEPTH_FORMAT, attach).create_view(&Default::default());
    let view = colour.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let cleared = |view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[cleared(&view), cleared(&velocity)],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pipeline.draw(&mut pass, &hull, fade);
    }
    let row = 256;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row * 8),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        colour.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        extent,
    );
    queue.submit([encoder.finish()]);
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the readback completes");
    let bytes = readback
        .slice(..)
        .get_mapped_range()
        .expect("range")
        .to_vec();
    let at = (4 * row + 4 * 4) as usize;
    Some([bytes[at], bytes[at + 1], bytes[at + 2]])
}
