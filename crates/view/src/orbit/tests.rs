use super::*;

const SIZE: u32 = 64;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// The colour the pass clears to, as the 8-bit pixel it lands as.
const CLEAR: [u8; 4] = [15, 18, 23, 255];

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    pollster::block_on(
        adapter.request_device(&mesh_render::device_descriptor("orbit test", &adapter)),
    )
    .ok()
}

/// Draws `model` through [`Scene`] into an offscreen target and reads it back.
fn render(model: Model) -> Option<Vec<u8>> {
    let (device, queue) = device()?;
    let scene = Scene::new(&device, &queue, model, FORMAT, Anisotropy::default())
        .expect("building the scene");
    scene.update(&queue, 1.0, Orbit::default(), 0.0);

    let size = wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    };
    let colour = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("orbit test colour"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
        texture_binding_view_dimension: None,
    });
    let view = colour.create_view(&wgpu::TextureViewDescriptor::default());
    let depth_view = make_depth_view(&device, SIZE, SIZE);

    let row = SIZE * 4;
    assert_eq!(row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT, 0);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("orbit test readback"),
        size: u64::from(row * SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("orbit test"),
    });
    scene.draw(&mut encoder, &view, &depth_view);
    encoder.copy_texture_to_buffer(
        colour.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(SIZE),
            },
        },
        size,
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("waiting for the GPU");
    let pixels = readback.slice(..).get_mapped_range().unwrap().to_vec();
    Some(pixels)
}

/// A model with no vertices and no indices must skip every `slice(..)` in the
/// pass: `Buffer::slice` on a zero-length buffer panics, and the window would
/// die on the first frame of an empty asset. The frame is the clear colour.
#[test]
fn a_model_with_no_geometry_draws_the_clear_colour_and_does_not_panic() {
    let Some(pixels) = render(Model::none("empty")) else {
        eprintln!("no adapter: skipped");
        return;
    };
    assert!(
        pixels.as_chunks::<4>().0.iter().all(|p| *p == CLEAR),
        "every pixel is the clear colour, first pixel {:?}",
        &pixels[..4]
    );
}
