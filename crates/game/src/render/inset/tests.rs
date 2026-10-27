use super::*;

const HD: Space = Space::HD;

fn text(x: f32, y: f32, scale: f32) -> Draw {
    Draw::Text {
        x,
        y,
        scale,
        color: [1.0; 4],
        border: None,
        align: oag_ui::frontend::Align::Left,
        text: "SPEED".into(),
        wrap_width: None,
    }
}

#[test]
fn full_size_places_nothing() {
    assert!(placement(HD, 1.0).is_none());
}

#[test]
fn a_widget_moves_in_about_the_centre_and_shrinks() {
    let at = placement(HD, 0.96).expect("an inset");
    let Some(Draw::Fill { rect, .. }) = inset(
        &Draw::Fill {
            rect: [160.0, 56.0, 1600.0, 954.0],
            color: [1.0; 4],
        },
        &at,
        HD,
    ) else {
        panic!("a fill stays a fill");
    };
    // The measured frame of the original at the comparison save's 96: x 192..1728.
    assert!((rect[0] - 192.0).abs() < 0.01, "{rect:?}");
    assert!((rect[0] + rect[2] - 1728.0).abs() < 0.01, "{rect:?}");
    let Some(Draw::Text { x, y, scale, .. }) = inset(&text(960.0, 540.0, 2.0), &at, HD) else {
        panic!("text stays text");
    };
    assert!(
        (x - 960.0).abs() < 0.01 && (y - 540.0).abs() < 0.01,
        "the centre stays"
    );
    assert!((scale - 1.92).abs() < 1e-4, "{scale}");
}

#[test]
fn the_picture_behind_the_widgets_is_not_moved() {
    let at = placement(HD, 0.9).expect("an inset");
    let clear = Draw::Fill {
        rect: [0.0, 0.0, 1920.0, 1080.0],
        color: [0.0, 0.0, 0.0, 1.0],
    };
    assert_eq!(inset(&clear, &at, HD), None);
    let video = Draw::Video {
        rect: [0.0, 0.0, 1920.0, 1080.0],
        frame: 0,
        position: 0,
        source: oag_ui::frontend::Video::Backdrop,
    };
    assert_eq!(inset(&video, &at, HD), None);
}

#[test]
fn a_pointer_lands_on_the_widget_drawn_under_it() {
    let at = placement(HD, 0.96).expect("an inset");
    let Some(Draw::Text { x, y, .. }) = inset(&text(300.0, 200.0, 1.0), &at, HD) else {
        panic!("text");
    };
    let (gx, gy) = uninset((x, y), 0.96, HD);
    assert!(
        (gx - 300.0).abs() < 0.01 && (gy - 200.0).abs() < 0.01,
        "{gx} {gy}"
    );
}

/// A renderer told to inset leaves the corner of a full-grid-sized widget uncovered, and one
/// that is not told covers it: the wiring `Renderer::render_with` applies, not only the maths.
///
/// **Skips with no adapter.**
#[test]
fn the_renderer_insets_its_widgets_once_told_to() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&Default::default()))
    else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let corner = |scale: f32| -> u8 {
        let mut renderer = crate::render::Renderer::new(
            &device,
            &queue,
            format,
            None,
            oag_ui::font::Atlas::build(),
            &oag_hud::sprite::Sheet::default(),
        )
        .expect("the ui pipeline");
        renderer.set_space(Space {
            size: (8.0, 8.0),
            display_aspect: 1.0,
        });
        renderer.set_inset(scale);
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("inset test"),
            size: wgpu::Extent3d {
                width: 8,
                height: 8,
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
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("inset readback"),
            size: u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * 8,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        // Not a full-grid fill (which is the page's clear and stays put): one a texel short.
        renderer.render(
            &device,
            &queue,
            &mut encoder,
            &view,
            &[Draw::Fill {
                rect: [0.0, 0.0, 7.0, 7.0],
                color: [1.0; 4],
            }],
            (0.0, 0.0, 8.0, 8.0),
            None,
        );
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                    rows_per_image: Some(8),
                },
            },
            wgpu::Extent3d {
                width: 8,
                height: 8,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the GPU");
        let pixel = slice.get_mapped_range().expect("the readback")[0];
        readback.unmap();
        pixel
    };
    assert_eq!(corner(1.0), 255, "uninset, the widget covers the corner");
    assert_eq!(corner(0.5), 0, "inset, the corner is left black");
}
