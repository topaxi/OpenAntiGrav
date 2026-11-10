use super::*;

const SIZE: (u32, u32) = (512, 272);

/// The target after `stamp_glow` of `stamps` over the picture `base` paints (the colour at each
/// pixel): the `rgb` at `(x, y)` for each probe, or `None` with no adapter to run on.
fn glow_at(
    stamps: &[Stamp],
    base: &dyn Fn(u32, u32) -> [u8; 3],
    probes: &[(u32, u32)],
) -> Option<Vec<[u8; 3]>> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::ordered(&instance)
        .into_iter()
        .next()?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&Default::default())).expect("a device");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("stamp test target"),
        size: wgpu::Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let mut pixels = Vec::with_capacity((SIZE.0 * SIZE.1 * 4) as usize);
    for y in 0..SIZE.1 {
        for x in 0..SIZE.0 {
            let [r, g, b] = base(x, y);
            pixels.extend_from_slice(&[r, g, b, 255]);
        }
    }
    queue.write_texture(
        target.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE.0 * 4),
            rows_per_image: Some(SIZE.1),
        },
        wgpu::Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
    );
    let bloom = Bloom::new(&device, format).expect("the pipelines build");
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    bloom.stamp_glow(
        &device,
        &queue,
        &mut encoder,
        &view,
        (0.0, 0.0),
        SIZE,
        stamps,
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("stamp test readback"),
        size: u64::from(SIZE.0 * SIZE.1 * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE.0 * 4),
                rows_per_image: Some(SIZE.1),
            },
        },
        wgpu::Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let data = readback.slice(..).get_mapped_range().expect("mapped");
    Some(
        probes
            .iter()
            .map(|&(x, y)| {
                let at = ((y * SIZE.0 + x) * 4) as usize;
                [data[at], data[at + 1], data[at + 2]]
            })
            .collect(),
    )
}

/// Pixels `x0..x1` by `y0..y1` of the test target, from the fractions a [`Stamp`] is given.
fn within(rect: [f32; 4], x: u32, y: u32) -> bool {
    let (x, y) = (x as f32 / SIZE.0 as f32, y as f32 / SIZE.1 as f32);
    x >= rect[0] && x < rect[0] + rect[2] && y >= rect[1] && y < rect[1] + rect[3]
}

fn stamps(rects: &[[f32; 4]], value: u8) -> Vec<Stamp> {
    rects.iter().map(|&rect| Stamp { rect, value }).collect()
}

const RED: [u8; 3] = [255, 0, 0];

/// Red under `rect`, black everywhere else: the HUD's fill over a dark scene.
fn red_under(rect: [f32; 4]) -> impl Fn(u32, u32) -> [u8; 3] {
    move |x, y| if within(rect, x, y) { RED } else { [0; 3] }
}

/// A stamped rectangle glows inside and a little past its edge, and nowhere far from it; a mask of
/// `51` glows less than `255`, and no rectangle or a mask of nothing adds nothing.
#[test]
fn a_stamp_glows_in_its_rectangle_and_scales_with_its_mask() {
    let rect = [0.25, 0.25, 0.5, 0.25];
    let base = red_under(rect);
    let probes = [(256, 100), (125, 100), (2, 2), (256, 271)];
    let Some(full) = glow_at(&stamps(&[rect], 255), &base, &probes) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert_eq!(
        full[0][0], 255,
        "inside the stamp the fill is saturated: {full:?}"
    );
    assert!(full[1][0] > 0, "just past its left edge: {full:?}");
    assert_eq!(
        (full[2], full[3]),
        ([0; 3], [0; 3]),
        "far from it: {full:?}"
    );
    let weak = glow_at(&stamps(&[rect], 51), &base, &probes).expect("the same adapter");
    assert!(
        weak[1][0] > 0 && weak[1][0] < full[1][0] / 2,
        "{weak:?} against {full:?}"
    );
    let bare = |read: Vec<[u8; 3]>| [read[1], read[2]];
    let plain = glow_at(&[], &base, &probes).unwrap();
    assert_eq!(
        bare(glow_at(&stamps(&[rect], 0), &base, &probes).unwrap()),
        bare(plain.clone())
    );
    assert_eq!(bare(plain), [[0; 3]; 2], "no stamp, no glow");
}

/// **The bright pass reads the target under the rectangle.** Three rectangles of one mask byte over
/// a picture with red under the first, green under the second and black under the third: each halo
/// is its rectangle's own colour, and the black one adds nothing. A pass that lit the stamp's own
/// colour, or the same colour for all three, fails here.
#[test]
fn a_stamp_glows_in_the_colour_the_target_holds_under_it() {
    let [a, b, c] = [
        [0.1, 0.25, 0.2, 0.25],
        [0.4, 0.25, 0.2, 0.25],
        [0.7, 0.25, 0.2, 0.25],
    ];
    let base = move |x, y| {
        if within(a, x, y) {
            RED
        } else if within(b, x, y) {
            [0, 255, 0]
        } else {
            [0; 3]
        }
    };
    // Four pixels outside each rectangle's left edge, on the black.
    let probes = [(47, 100), (200, 100), (354, 100)];
    let Some(read) = glow_at(&stamps(&[a, b, c], 255), &base, &probes) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert!(
        read[0][0] > 0 && read[0][1] == 0,
        "red under the first: {read:?}"
    );
    assert!(
        read[1][1] > 0 && read[1][0] == 0,
        "green under the second: {read:?}"
    );
    assert_eq!(
        read[2], [0; 3],
        "black under the third adds nothing: {read:?}"
    );
}

/// Two rectangles in one call keep their own mask byte: over the same red, the stronger one glows
/// more, and a weaker mask on the second rectangle does not dim the first.
#[test]
fn each_stamp_keeps_its_own_mask() {
    let rects = [[0.1, 0.25, 0.3, 0.25], [0.6, 0.25, 0.3, 0.25]];
    let base = move |x, y| {
        if rects.iter().any(|&rect| within(rect, x, y)) {
            RED
        } else {
            [0; 3]
        }
    };
    let both = [
        Stamp {
            rect: rects[0],
            value: 255,
        },
        Stamp {
            rect: rects[1],
            value: 51,
        },
    ];
    let probes = [(47, 100), (304, 100)];
    let Some(read) = glow_at(&both, &base, &probes) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert!(
        read[0][0] > read[1][0],
        "the stronger mask glows more: {read:?}"
    );
    assert!(read[1][0] > 0, "the weaker one still glows: {read:?}");
    let alone = glow_at(&both[..1], &base, &probes).expect("the same adapter");
    assert_eq!(read[0], alone[0], "the second stamp leaves the first alone");
}
