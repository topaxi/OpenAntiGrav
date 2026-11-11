use super::*;

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = crate::test_adapter::request(&instance)?;
    pollster::block_on(adapter.request_device(&Default::default())).ok()
}

/// The texels of an `Rgba8Unorm` `texture` back, row-major, by loading each in a compute
/// shader: the texture has no `COPY_SRC`, and a test must not add one to read it.
fn read(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    const SHADER: &str = "
        @group(0) @binding(0) var image: texture_2d<f32>;
        @group(0) @binding(1) var<storage, read_write> out: array<u32>;
        @compute @workgroup_size(1)
        fn main(@builtin(global_invocation_id) id: vec3<u32>) {
            let size = textureDimensions(image);
            if (id.x < size.x && id.y < size.y) {
                out[id.y * size.x + id.x] = pack4x8unorm(textureLoad(image, vec2<i32>(id.xy), 0));
            }
        }";
    let (width, height) = (texture.width(), texture.height());
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("texel readback"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("texel readback"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let bytes = u64::from(width * height * 4);
    let storage = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("texel storage"),
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let out = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("texel readback"),
        size: bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("texel readback"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: storage.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(width, height, 1);
    }
    encoder.copy_buffer_to_buffer(&storage, 0, &out, 0, bytes);
    queue.submit([encoder.finish()]);
    out.slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.expect("maps"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("polls");
    let mapped = out.slice(..).get_mapped_range().expect("range");
    mapped.to_vec()
}

/// An odd-sized image survives the write row for row, and the texture is what the pipelines
/// that took this over each created by hand.
#[test]
fn an_odd_sized_image_is_a_one_mip_sampled_texture_holding_its_rows() {
    let Some((device, queue)) = device() else {
        eprintln!("no adapter: skipped");
        return;
    };
    let rgba: Vec<u8> = (0..3 * 5 * 4).map(|i| (i * 7) as u8).collect();
    let texture = upload_rgba8(&device, &queue, "odd", (3, 5), &rgba);
    assert_eq!((texture.width(), texture.height()), (3, 5));
    assert_eq!(texture.depth_or_array_layers(), 1);
    assert_eq!(texture.format(), wgpu::TextureFormat::Rgba8Unorm);
    assert_eq!(texture.mip_level_count(), 1);
    assert_eq!(texture.sample_count(), 1);
    assert_eq!(texture.dimension(), wgpu::TextureDimension::D2);
    assert_eq!(
        texture.usage(),
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST
    );
    assert_eq!(read(&device, &queue, &texture), rgba);
}

/// A decoder's empty image is a 1x1 texture, not a validation error, and a one-channel
/// format strides by its own texel size.
#[test]
fn a_zero_extent_is_one_texel_and_the_row_stride_follows_the_format() {
    let Some((device, queue)) = device() else {
        eprintln!("no adapter: skipped");
        return;
    };
    let empty = upload_rgba8(&device, &queue, "empty", (0, 0), &[9, 8, 7, 6]);
    assert_eq!((empty.width(), empty.height()), (1, 1));

    let grey = upload_2d(
        &device,
        &queue,
        "grey",
        wgpu::TextureFormat::R8Unorm,
        (5, 2),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    );
    assert_eq!((grey.width(), grey.height()), (5, 2));
    assert_eq!(grey.format(), wgpu::TextureFormat::R8Unorm);
}

#[test]
fn the_sampler_builds_on_a_device() {
    let Some((device, _queue)) = device() else {
        eprintln!("no adapter: skipped");
        return;
    };
    let _ = clamp_linear_sampler(&device, "clamp linear");
}
