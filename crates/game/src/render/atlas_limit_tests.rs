//! A glyph atlas larger than the device's textures is drawn reduced, never a validation panic:
//! a texture pack's face and its button glyphs are read before any device exists.
//!
//! **Skips with no adapter.**

use super::*;

/// A real face 128 texels wide whose picture is `factor` times that, so the atlas is
/// `128 * factor` wide, with one glyph.
fn big_face(factor: u32) -> Atlas {
    let font = oag_texture::fnt::Font {
        line_height: 12,
        unknown: 0,
        width: 128,
        height: 8,
        palette: vec![[0, 0, 0, 0], [200, 200, 200, 255]],
        indices: vec![0; 128 * 8],
        glyphs: vec![oag_texture::fnt::Glyph {
            codepoint: u16::from(b'A'),
            width: 4,
            height: 4,
            u0: 2,
            u1: 6,
            v0: 1,
            v1: 5,
            advance: 5,
        }],
    };
    let (width, height) = (128 * factor, 8 * factor);
    let picture = oag_texture::replacer::Image {
        width,
        height,
        rgba: [255, 255, 255, 255].repeat((width * height) as usize),
    };
    Atlas::from_replaced_font(&font, &picture).expect("a whole multiple")
}

/// A device whose `max_texture_dimension_2d` is exactly `limit`, or `None` with no adapter.
fn small_device(limit: u32) -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: wgpu::Limits {
            max_texture_dimension_2d: limit,
            ..wgpu::Limits::default()
        },
        ..Default::default()
    }))
    .ok()
}

#[test]
fn an_atlas_over_the_devices_limit_is_drawn_reduced_in_all_three_slots() {
    let limit = 1024;
    let Some((device, queue)) = small_device(limit) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let big = big_face(16);
    assert!(big.width > limit, "the fixture is over the limit");
    let mut renderer = Renderer::new(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        big.clone(),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");
    renderer.set_face_atlas(&device, &queue, Some(big.clone()), Some("Title"));
    renderer.set_buttons_atlas(&device, &queue, Some(big.clone()), "Buttons");
    for (slot, atlas) in [
        ("glyph atlas", Some(&renderer.atlas)),
        ("face atlas", renderer.face_atlas.as_ref()),
        ("buttons atlas", renderer.buttons_atlas.as_ref()),
    ] {
        let atlas = atlas.expect("loaded");
        assert!(
            atlas.width.max(atlas.height) <= limit,
            "{slot} is {}x{}",
            atlas.width,
            atlas.height
        );
        assert_eq!(
            oag_ui::font::measure(atlas, "A"),
            oag_ui::font::measure(&big, "A"),
            "{slot}: reducing a face moves no line"
        );
    }
}

#[test]
fn an_atlas_that_fits_is_uploaded_as_it_is() {
    let Some((device, _queue)) = small_device(1024) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let small = big_face(4);
    let kept = fit_to_device(&device, "test", small.clone());
    assert_eq!((kept.width, kept.height), (small.width, small.height));
}
