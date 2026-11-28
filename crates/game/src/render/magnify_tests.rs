//! A page part-way through a change: its text keeps the resting page's layout.

use super::*;
use oag_texture::fnt::{Font, Glyph};
use oag_ui::menu::Layers;

/// With Pulse's pens, which round every ink edge to a whole unit, a zoomed page's glyphs are the
/// resting page's scaled about the zoom's origin, at every step of the change.
///
/// **Skips with no adapter.**
#[test]
fn a_zoomed_pages_glyphs_are_the_resting_ones_scaled_as_a_block() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&Default::default()))
    else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let font = Font {
        line_height: 8,
        unknown: 0,
        width: 8,
        height: 8,
        palette: vec![[255, 255, 255, 255]; 16],
        indices: vec![1; 64],
        glyphs: vec![Glyph {
            codepoint: u16::from(b'm'),
            width: 4,
            height: 4,
            u0: 0,
            u1: 4,
            v0: 0,
            v1: 4,
            advance: 12,
            bearing: -1,
        }],
    };
    let mut renderer = Renderer::new(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        Atlas::from_font(&font),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");
    renderer.set_bearing_pens(true);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 480,
            height: 272,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
        texture_binding_view_dimension: None,
    });
    let view = texture.create_view(&Default::default());
    let mut drawn = |page: &Layers| -> Vec<[f32; 4]> {
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.render(
            &device,
            &queue,
            &mut encoder,
            &view,
            &page.body,
            (0.0, 0.0, 480.0, 272.0),
            None,
        );
        renderer.quads.iter().map(|quad| quad.rect).collect()
    };
    let page = Layers {
        body: vec![Draw::Text {
            x: 66.0,
            y: 40.0,
            scale: 0.8,
            color: [1.0; 4],
            border: None,
            align: Align::Left,
            text: "mmmmmmmmmm".to_string(),
            wrap_width: Some(300.0),
        }],
        ..Layers::default()
    };
    let rest = drawn(&page);
    assert_eq!(rest.len(), 10);

    let origin = (150.0, 127.0);
    for factor in [0.7, 0.83, 0.96, 1.07, 1.19, 1.5] {
        let zoomed = drawn(&page.clone().zoomed(origin, factor, 1.0));
        assert_eq!(zoomed.len(), rest.len(), "at {factor}");
        for (was, now) in rest.iter().zip(&zoomed) {
            let expected = [
                origin.0 + (was[0] - origin.0) * factor,
                origin.1 + (was[1] - origin.1) * factor,
                was[2] * factor,
                was[3] * factor,
            ];
            assert!(
                now.iter().zip(expected).all(|(a, b)| (a - b).abs() < 0.001),
                "at {factor}: {now:?}, the resting glyph scaled is {expected:?}"
            );
        }
    }
}
