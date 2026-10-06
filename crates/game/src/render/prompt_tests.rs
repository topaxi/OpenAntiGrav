//! The renderer's half of the button-prompt swap: a string's stand-in draws
//! as the substitute glyph once a substitution is set, and as the disc's own
//! glyph before.
//!
//! **Skips with no adapter**, like the blend test beside it.

use super::*;
use oag_title::prompts::{Prompt, Prompts, Scope};
use oag_ui::prompt::{Substitution, art_index};

const TABLE: &Prompts = &Prompts {
    scope: Scope::AnyFace,
    glyphs: &[('A', Prompt::Cross)],
};

/// 24x24 pixels of `draws` over black, or `None` with no adapter.
fn render_text(substitution: Substitution) -> Option<Vec<u8>> {
    const SIDE: u32 = 24;
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).ok()?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = Renderer::new(
        &device,
        &queue,
        format,
        None,
        Atlas::build().with_prompts(TABLE),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");
    renderer.set_space(Space {
        size: (SIDE as f32, SIDE as f32),
        display_aspect: 1.0,
    });
    renderer.set_prompt_substitution(substitution);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("prompt test"),
        size: wgpu::Extent3d {
            width: SIDE,
            height: SIDE,
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
    let row = (SIDE * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("prompt test readback"),
        size: u64::from(row * SIDE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    renderer.render_with(
        wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        &device,
        &queue,
        &mut encoder,
        &view,
        &[Draw::Text {
            x: 4.0,
            y: 4.0,
            scale: 2.0,
            color: [1.0; 4],
            border: None,
            align: Align::Left,
            text: "A".to_string(),
            wrap_width: None,
        }],
        (0.0, 0.0, SIDE as f32, SIDE as f32),
        None,
    );
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(SIDE),
            },
        },
        wgpu::Extent3d {
            width: SIDE,
            height: SIDE,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
    let mapped = slice.get_mapped_range().ok()?;
    let mut out = Vec::new();
    for y in 0..SIDE as usize {
        let start = y * row as usize;
        out.extend_from_slice(&mapped[start..start + SIDE as usize * 4]);
    }
    Some(out)
}

#[test]
fn a_stand_in_draws_as_the_substitute_glyph_once_one_is_set() {
    let Some(disc) = render_text(Substitution::none()) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let swapped = render_text(Substitution::none().with(TABLE, 0, art_index("xbox-a")))
        .expect("the same adapter");
    let ink = |pixels: &[u8]| {
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] > 64)
            .count()
    };
    assert!(ink(&disc) > 0, "the disc's own glyph draws");
    assert!(ink(&swapped) > 0, "the substitute draws");
    assert_ne!(disc, swapped, "the substitution reached the pixels");
}
