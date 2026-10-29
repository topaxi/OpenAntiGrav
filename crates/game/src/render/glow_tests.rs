//! The front end's glow mask, through the real [`Renderer`]: only what a `Draw::GlowStamp` covers
//! blooms, and by its value.

use super::*;
use oag_ui::frontend::Draw;

const W: u32 = 480;
const H: u32 = 272;

/// The frame `list` draws, as RGBA rows; `None` on a machine with no adapter.
fn frame(list: &[Draw]) -> Option<Vec<u8>> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).ok()?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = Renderer::new(
        &device,
        &queue,
        format,
        None,
        Atlas::build(),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("glow test"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let row = (W * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("glow test readback"),
        size: u64::from(row * H),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    renderer.render(
        &device,
        &queue,
        &mut encoder,
        &view,
        list,
        (0.0, 0.0, W as f32, H as f32),
        None,
    );
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the GPU");
    let mapped = slice.get_mapped_range().expect("the readback");
    let mut out = Vec::new();
    for y in 0..H {
        let at = (y * row) as usize;
        out.extend_from_slice(&mapped[at..at + (W * 4) as usize]);
    }
    Some(out)
}

fn red(frame: &[u8], x: u32, y: u32) -> u8 {
    frame[((y * W + x) * 4) as usize]
}

const RECT: [f32; 4] = [200.0, 100.0, 40.0, 40.0];

fn white_square(stamp: Option<u8>) -> Vec<Draw> {
    let mut list = vec![Draw::Fill {
        rect: RECT,
        color: [1.0, 1.0, 1.0, 1.0],
    }];
    if let Some(value) = stamp {
        list.push(Draw::GlowStamp { rect: RECT, value });
    }
    list
}

#[test]
fn a_stamped_rectangle_glows_past_its_edge_and_an_unstamped_one_does_not() {
    let (Some(plain), Some(stamped)) =
        (frame(&white_square(None)), frame(&white_square(Some(200))))
    else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    // Six pixels left of the square: inside the blur's reach, outside the square.
    assert_eq!(red(&plain, 194, 120), 0, "no stamp, no bloom");
    assert!(
        red(&stamped, 194, 120) > 8,
        "the stamp's rgb * 200/255 blurs past the square's edge, got {}",
        red(&stamped, 194, 120)
    );
    // Far from every stamp nothing is added, and the square's own pixels are never darkened.
    assert_eq!(red(&stamped, 20, 20), 0);
    assert!(red(&stamped, 220, 120) >= red(&plain, 220, 120));
}

#[test]
fn the_glow_follows_the_stamps_value() {
    let (Some(weak), Some(strong)) = (
        frame(&white_square(Some(70))),
        frame(&white_square(Some(230))),
    ) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert!(
        red(&strong, 194, 120) > red(&weak, 194, 120),
        "230 must glow more than 70: {} vs {}",
        red(&strong, 194, 120),
        red(&weak, 194, 120)
    );
}
