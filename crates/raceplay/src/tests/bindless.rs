//! Group 1 as binding arrays draws every material slot's own texture, from its own
//! drawable's arrays: the same pixels as the per-slot groups, slot for slot
//! (`oag_mesh::mesh_render::bindless`).

use super::scene_build::{test_gpu, vertex};
use super::*;
use oag_mesh::mesh::{Bounds, DrawCall};

const SIZE: u32 = 64;
/// Strips per drawable; two drawables share the target, one a half each.
const STRIPS: usize = 8;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SHARED: usize = 5;
const SHARED_WITH: usize = 2;
const UNDECODED: usize = 6;

/// `STRIPS` vertical strips across half `half` (0 left, 1 right) of the screen in clip
/// space, strip `i` drawn as its own draw call from texture slot `i`, a 1x1 of a colour no
/// other strip of either half has, except that slot [`SHARED`] names slot [`SHARED_WITH`]'s
/// texture (the arrays hold it once) and slot [`UNDECODED`] has none (the white fallback
/// every such slot shares).
fn strips(half: usize) -> Model {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut draws = Vec::new();
    let width = 1.0 / STRIPS as f32;
    for strip in 0..STRIPS {
        let left = -1.0 + half as f32 + width * strip as f32;
        let base = vertices.len() as u32;
        for (x, y) in [
            (left, -1.0),
            (left + width, -1.0),
            (left + width, 1.0),
            (left, 1.0),
        ] {
            vertices.push(vertex([x, y, 0.5]));
        }
        let start = indices.len() as u32;
        indices.extend([0, 1, 2, 0, 2, 3].map(|i| base + i));
        draws.push(DrawCall {
            writes_depth: false,
            moving: false,
            blend: None,
            blend_state: None,
            depth_point: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            culled: false,
            range: start..start + 6,
            texture: Some(strip),
            bounds: Bounds {
                centre: [0.0; 3],
                radius: 2.0,
            },
            node: None,
            chunk: None,
            alpha_test_ref: None,
        });
    }
    let mut textures: Vec<_> = (0..STRIPS)
        .map(|strip| {
            let strip = strip + half * STRIPS;
            let rgba = vec![
                (strip * 31 % 256) as u8,
                (255 - strip * 13) as u8,
                (strip * 67 % 256) as u8,
                255,
            ];
            Some(std::sync::Arc::new(mesh::ModelTexture::rgba8(
                format!("strip {strip}"),
                1,
                1,
                rgba,
                None,
            )))
        })
        .collect();
    textures[SHARED] = textures[SHARED_WITH].clone();
    textures[UNDECODED] = None;
    Model {
        vertices,
        indices,
        draws,
        textures,
        ..model(0, 0)
    }
}

fn drawable(device: &wgpu::Device, queue: &wgpu::Queue, half: usize, bindless: bool) -> Drawable {
    Drawable::new_with(
        device,
        queue,
        strips(half),
        FORMAT,
        oag_gpu::formats::SceneStore::Computed,
        Anisotropy::Off,
        1,
        mesh_render::Depth::Scene,
        mesh_render::TRANSPARENT_BLEND,
        mesh_render::GlowMask::Protected,
        &mesh_render::zone::StageArt::NONE,
        mesh_render::ShadowMaps::NONE,
        mesh_render::ShadowReceiver::Never,
        mesh_render::Texcoords::Interleaved,
        false,
        false,
        mesh_render::Velocity::None,
        bindless,
    )
    .expect("the strips build")
}

/// `drawables` drawn one after another in one pass with identity matrices, read back row
/// by row.
fn pixels(device: &wgpu::Device, queue: &wgpu::Queue, drawables: &[Drawable]) -> Vec<u8> {
    pixels_with(device, queue, drawables, |drawable, pass| {
        drawable.draw(pass, ViewBlock::NONE, None, None, None, None);
    })
}

/// [`pixels`] with each drawable drawn by `draw`.
fn pixels_with(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    drawables: &[Drawable],
    draw: impl Fn(&Drawable, &mut wgpu::RenderPass<'_>),
) -> Vec<u8> {
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bindless test"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let colour = target(
        FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        mesh_render::DEPTH_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    for drawable in drawables {
        drawable.write(
            &drawable::Uploads::immediate(queue),
            Mat4::IDENTITY,
            Mat4::IDENTITY,
            Mat4::IDENTITY,
        );
    }
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let colour_view = colour.create_view(&Default::default());
        let depth_view = depth.create_view(&Default::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("bindless test"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &colour_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        for drawable in drawables {
            draw(drawable, &mut pass);
        }
    }
    let row = SIZE * 4;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bindless test readback"),
        size: u64::from(row * SIZE),
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
                rows_per_image: Some(SIZE),
            },
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the readback maps");
    slice
        .get_mapped_range()
        .expect("the readback is mapped")
        .to_vec()
}

/// Each strip's centre pixel, left to right across both halves.
fn centres(pixels: &[u8]) -> Vec<[u8; 4]> {
    let y = SIZE as usize / 2;
    (0..2 * STRIPS)
        .map(|strip| {
            let x = (2 * strip + 1) * SIZE as usize / (4 * STRIPS);
            let at = (y * SIZE as usize + x) * 4;
            [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
        })
        .collect()
}

/// **A draw on the arrays reads its own slot of its own drawable.** Two drawables of eight
/// slots each, drawn in one pass once through the arrays and once through the per-slot
/// groups: every pixel equal, and the strips distinct but for the pairs that share a
/// texture, so a draw reading another slot (a row pushed for the wrong slot or not pushed at
/// a change, the white fallback at 0 forgotten, a role in the row shifted) or another
/// drawable's arrays (a group left bound from the drawable before) changes a strip. Fails,
/// too, if a device that has the features stops taking the arrays path.
#[test]
fn each_draw_on_the_binding_arrays_reads_its_own_material_slot() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let arrays = [0, 1].map(|half| drawable(&device, &queue, half, true));
    if !mesh_render::bindless::supported(&device) {
        assert!(!arrays[0].binds_arrays(), "no features, no arrays");
        eprintln!("no binding arrays on this adapter: the per-slot path only");
        return;
    }
    assert!(
        arrays.iter().all(Drawable::binds_arrays),
        "the device has the features"
    );
    let per_slot = [0, 1].map(|half| drawable(&device, &queue, half, false));
    assert!(!per_slot.iter().any(Drawable::binds_arrays));
    let expected = pixels(&device, &queue, &per_slot);
    let strips = centres(&expected);
    for half in [0, STRIPS] {
        assert_eq!(
            strips[half + SHARED],
            strips[half + SHARED_WITH],
            "one texture, one colour"
        );
    }
    assert_eq!(strips[UNDECODED], strips[STRIPS + UNDECODED], "both white");
    for (index, strip) in strips.iter().enumerate() {
        if index % STRIPS == SHARED || index == STRIPS + UNDECODED {
            continue;
        }
        assert!(
            !strips[..index].contains(strip),
            "strip {index} draws {strip:?} like an earlier one: {strips:?}"
        );
    }
    let got = pixels(&device, &queue, &arrays);
    assert_eq!(centres(&got), strips, "the strips through the arrays");
    assert!(
        got == expected,
        "every pixel as the per-slot groups draw it"
    );
}

/// **A drawable on the arrays draws first in a pass.** The overlay lists
/// (`draw_overlay`, the plume and shield path) bound the arrays and pushed slot 0's row
/// before their first `set_pipeline`; wgpu checks immediates against the current
/// pipeline's layout, and a fresh pass has none, so a split-screen view whose pass began
/// with an overlay died with "immediate data ... overruns the immediate data range with a
/// size of 0". Every earlier test drew an opaque drawable first, whose pipeline was set.
/// The same pixels as the per-slot groups, and no validation error.
#[test]
fn an_overlay_on_the_arrays_draws_first_in_a_pass() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    if !mesh_render::bindless::supported(&device) {
        eprintln!("no binding arrays on this adapter: nothing to check");
        return;
    }
    let overlay = |drawable: &Drawable, pass: &mut wgpu::RenderPass<'_>| {
        drawable.draw_overlay(pass, ViewBlock::NONE);
    };
    let arrays = [0, 1].map(|half| drawable(&device, &queue, half, true));
    assert!(arrays.iter().all(Drawable::binds_arrays));
    let per_slot = [0, 1].map(|half| drawable(&device, &queue, half, false));
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let got = pixels_with(&device, &queue, &arrays, overlay);
    let error = pollster::block_on(scope.pop());
    assert!(error.is_none(), "validation error: {error:?}");
    assert!(
        got == pixels_with(&device, &queue, &per_slot, overlay),
        "every pixel as the per-slot groups draw it"
    );
}
