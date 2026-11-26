//! The front end's glow mask, through the real [`Renderer`]: only what a `Draw::GlowStamp` covers
//! blooms, and by its value.

use super::*;
use oag_ui::frontend::Draw;

const W: u32 = 480;
const H: u32 = 272;

/// The frame `list` draws, as RGBA rows; `None` on a machine with no adapter.
fn frame(list: &[Draw]) -> Option<Vec<u8>> {
    frame_with(list, &oag_hud::sprite::Sheet::default())
}

/// [`frame`] over a sprite sheet of the caller's.
fn frame_with(list: &[Draw], sheet: &oag_hud::sprite::Sheet) -> Option<Vec<u8>> {
    frame_pass(list, sheet, None)
}

/// [`frame_with`], drawn as a `LoadOp::Load` pass over the target when `over_scene` is
/// `Some(flag)`, with [`Renderer::set_glow_over_scene`] set to `flag`.
fn frame_pass(
    list: &[Draw],
    sheet: &oag_hud::sprite::Sheet,
    over_scene: Option<bool>,
) -> Option<Vec<u8>> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).ok()?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = Renderer::new(&device, &queue, format, None, Atlas::build(), sheet)
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
        texture_binding_view_dimension: None,
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
    match over_scene {
        None => renderer.render(
            &device,
            &queue,
            &mut encoder,
            &view,
            list,
            (0.0, 0.0, W as f32, H as f32),
            None,
        ),
        Some(flag) => {
            renderer.set_glow_over_scene(flag);
            renderer.render_with(
                wgpu::LoadOp::Load,
                &device,
                &queue,
                &mut encoder,
                &view,
                list,
                (0.0, 0.0, W as f32, H as f32),
                None,
            );
        }
    }
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

/// A page over a parked race is a `LoadOp::Load` pass: it blooms its stamps only when the
/// renderer is told so (`menu_stage` does for a frozen race), and a `Load` list drawn without the
/// flag stays as unbloomed as the overlays and the results table.
#[test]
fn a_load_pass_blooms_its_stamps_only_when_the_renderer_is_told_to() {
    let sheet = oag_hud::sprite::Sheet::default();
    let list = white_square(Some(200));
    let (Some(on), Some(off)) = (
        frame_pass(&list, &sheet, Some(true)),
        frame_pass(&list, &sheet, Some(false)),
    ) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert_eq!(
        red(&off, 194, 120),
        0,
        "a Load pass keeps its stamps unbloomed"
    );
    assert!(
        red(&on, 194, 120) > 8,
        "told so, a Load pass blooms past the square's edge, got {}",
        red(&on, 194, 120)
    );
    assert!(red(&on, 220, 120) >= red(&off, 220, 120));
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

fn text_list(stamp: Option<u8>) -> Vec<Draw> {
    let mut list = vec![Draw::Text {
        x: 200.0,
        y: 100.0,
        scale: 6.0,
        color: [1.0, 1.0, 1.0, 1.0],
        border: None,
        align: oag_ui::frontend::Align::Left,
        text: "HHHH".to_string(),
        wrap_width: None,
    }];
    if let Some(value) = stamp {
        list.push(Draw::GlowText { value });
    }
    list
}

/// A `Draw::GlowText` stamps the glyph quads of the text draw before it: the text blooms past its
/// own edge, and far from it nothing does.
#[test]
fn a_text_stamps_its_glyph_quads_and_blooms() {
    let (Some(plain), Some(stamped)) = (frame(&text_list(None)), frame(&text_list(Some(220))))
    else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let near = |frame: &[u8]| {
        (180..200)
            .map(|x| u32::from(red(frame, x, 110)))
            .sum::<u32>()
    };
    assert_eq!(near(&plain), 0, "no stamp, no bloom");
    assert!(
        near(&stamped) > 40,
        "the stamped text blooms past its left edge, got {}",
        near(&stamped)
    );
    assert_eq!(
        red(&stamped, 20, 20),
        0,
        "far from the text nothing is added"
    );
}

/// A sheet of one opaque white texel, so a sheet-textured draw shows its colour alone.
fn white_sheet() -> oag_hud::sprite::Sheet {
    let mut sheet = oag_hud::sprite::Sheet::default();
    sheet.rgba = vec![255; 4];
    sheet
}

fn corner(at: [f32; 2], color: [f32; 3]) -> oag_ui::frontend::Corner {
    oag_ui::frontend::Corner {
        at,
        uv: [0.5, 0.5],
        color,
    }
}

/// A `Draw::Triangles` triangle covers its own area and nothing beyond it, and each corner's
/// colour is interpolated across it.
#[test]
fn a_triangle_fills_its_area_with_the_corners_colours() {
    let list = [Draw::Triangles(vec![[
        corner([200.0, 100.0], [1.0, 0.0, 0.0]),
        corner([280.0, 100.0], [0.0, 0.0, 0.0]),
        corner([200.0, 180.0], [0.0, 0.0, 0.0]),
    ]])];
    let Some(frame) = frame_with(&list, &white_sheet()) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert_eq!(
        red(&frame, 270, 170),
        0,
        "beyond the hypotenuse nothing is drawn"
    );
    assert_eq!(
        red(&frame, 190, 90),
        0,
        "outside the bounding box nothing is drawn"
    );
    let near_red = red(&frame, 206, 106);
    let far = red(&frame, 255, 112);
    assert!(near_red > 200, "next to the red corner, got {near_red}");
    assert!(
        far < near_red / 2,
        "the colour fades from the red corner, got {far}"
    );
}

/// A `Draw::SolidTriangles` triangle is a flat fill of its colour inside its three corners and
/// nothing outside them.
#[test]
fn a_solid_triangle_fills_its_area_in_its_one_colour() {
    let list = [Draw::SolidTriangles {
        triangles: vec![[[200.0, 100.0], [280.0, 100.0], [200.0, 180.0]]],
        color: [1.0, 0.0, 0.0, 1.0],
    }];
    let Some(frame) = frame_with(&list, &white_sheet()) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert!(red(&frame, 206, 106) > 240, "inside, next to the corner");
    assert!(
        red(&frame, 235, 115) > 240,
        "inside, away from every corner: one flat colour"
    );
    assert_eq!(red(&frame, 270, 170), 0, "beyond the hypotenuse");
    assert_eq!(red(&frame, 190, 90), 0, "outside the bounding box");
}

/// A sheet of two texels: opaque white, then white with no alpha.
fn half_transparent_sheet() -> oag_hud::sprite::Sheet {
    let mut sheet = oag_hud::sprite::Sheet::default();
    sheet.width = 2;
    sheet.rgba = vec![255, 255, 255, 255, 255, 255, 255, 0];
    sheet
}

/// A white square under a sprite whose left half is opaque and whose right half is transparent,
/// then `stamp`.
fn half_sprite_list(stamp: Draw) -> Vec<Draw> {
    vec![
        Draw::Fill {
            rect: RECT,
            color: [1.0, 1.0, 1.0, 1.0],
        },
        Draw::Sprite {
            rect: RECT,
            uv: [0.0, 0.0, 2.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
        },
        stamp,
    ]
}

/// A `Draw::GlowTexel` writes the sprite's texel alpha, not its rectangle: the stamped half blooms
/// and the half whose texels have no alpha does not, though a white square shows through it. The
/// same sprite stamped as a rectangle blooms past both edges.
#[test]
fn a_texel_stamp_blooms_where_the_sprite_is_opaque_and_nowhere_else() {
    let sheet = half_transparent_sheet();
    let (Some(texel), Some(rectangle)) = (
        frame_with(&half_sprite_list(Draw::GlowTexel), &sheet),
        frame_with(
            &half_sprite_list(Draw::GlowStamp {
                rect: RECT,
                value: 255,
            }),
            &sheet,
        ),
    ) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let left = (195, 120);
    let right = (250, 120);
    assert!(
        red(&texel, left.0, left.1) >= 4,
        "the opaque half blooms past its edge, got {}",
        red(&texel, left.0, left.1)
    );
    assert_eq!(
        red(&texel, right.0, right.1),
        0,
        "the transparent half stamps nothing, so nothing blooms past its edge"
    );
    assert!(
        red(&rectangle, right.0, right.1) >= 4,
        "the rectangle stamp covers both halves, got {}",
        red(&rectangle, right.0, right.1)
    );
    assert_eq!(
        red(&texel, 20, 20),
        0,
        "far from the sprite nothing is added"
    );
}

/// Where the fill of the test below sits: its glow, were it stamped, reaches a few pixels left of its
/// edge, which no stamp of the sprite's reaches.
const FILL: [f32; 4] = [260.0, 100.0, 40.0, 40.0];

/// A texel stamp after a draw that is not a sprite stamps nothing, so nothing blooms: neither the
/// earlier sprite's quads (a `last_sprite` that was not cleared) nor the fill's.
#[test]
fn a_texel_stamp_after_a_fill_blooms_nothing() {
    let list = [
        Draw::Sprite {
            rect: RECT,
            uv: [0.0, 0.0, 2.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
        },
        Draw::Fill {
            rect: FILL,
            color: [1.0, 1.0, 1.0, 1.0],
        },
        Draw::GlowTexel,
    ];
    let Some(frame) = frame_with(&list, &half_transparent_sheet()) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert_eq!(
        red(&frame, 195, 120),
        0,
        "the sprite was not the draw before the stamp"
    );
    assert_eq!(red(&frame, 256, 120), 0, "nor was the fill a sprite");
}

/// The PS2 chain's picture is the letterboxed one: on the display's own aspect it is the whole
/// viewport, and in a taller window it is the centred band the screen is fitted into, so the
/// composite's `5/640` and `5/448` are fractions of the picture and not of the bars around it.
#[test]
fn the_ps2_chain_reads_the_letterboxed_picture() {
    let wide = 16.0 / 9.0;
    assert_eq!(picture_rect((1280, 720), wide), ((0.0, 0.0), (1280, 720)));
    assert_eq!(picture_rect((1280, 960), wide), ((0.0, 120.0), (1280, 720)));
    assert_eq!(picture_rect((1600, 720), wide), ((160.0, 0.0), (1280, 720)));
}
