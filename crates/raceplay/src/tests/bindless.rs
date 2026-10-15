//! Group 1 as binding arrays draws every material slot's own texture: the same pixels as
//! the per-slot groups, slot for slot (`oag_mesh::mesh_render::bindless`).

use super::scene_build::{test_gpu, vertex};
use super::*;
use oag_mesh::mesh::{Bounds, DrawCall};

const SIZE: u32 = 64;
const STRIPS: usize = 8;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SHARED: usize = 5;
const SHARED_WITH: usize = 2;
const UNDECODED: usize = 6;

/// `STRIPS` vertical strips across the screen in clip space, strip `i` drawn as its own
/// draw call from texture slot `i`, a 1x1 of its own colour, except that slot [`SHARED`]
/// names slot [`SHARED_WITH`]'s texture (the arrays hold it once) and slot [`UNDECODED`]
/// has none (the white fallback every such slot shares).
fn strips() -> Model {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut draws = Vec::new();
    let width = 2.0 / STRIPS as f32;
    for strip in 0..STRIPS {
        let left = -1.0 + width * strip as f32;
        let base = vertices.len() as u32;
        for (x, y) in [(left, -1.0), (left + width, -1.0), (left + width, 1.0), (left, 1.0)] {
            vertices.push(vertex([x, y, 0.5]));
        }
        let start = indices.len() as u32;
        indices.extend([0, 1, 2, 0, 2, 3].map(|i| base + i));
        draws.push(DrawCall {
            moving: false,
            blend: None,
            blend_state: None,
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
            let rgba = vec![
                (strip * 31 % 256) as u8,
                (255 - strip * 29) as u8,
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

fn drawable(device: &wgpu::Device, queue: &wgpu::Queue, bindless: bool) -> Drawable {
    Drawable::new_with(
        device,
        queue,
        strips(),
        FORMAT,
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

/// `drawable` drawn over the whole target with identity matrices, read back row by row.
fn pixels(device: &wgpu::Device, queue: &wgpu::Queue, drawable: &Drawable) -> Vec<u8> {
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
    drawable.write(
        &drawable::Uploads::immediate(queue),
        Mat4::IDENTITY,
        Mat4::IDENTITY,
        Mat4::IDENTITY,
    );
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
        drawable.draw(&mut pass, ViewBlock::NONE, None, None, None, None);
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
    slice.get_mapped_range().expect("the readback is mapped").to_vec()
}

/// Each strip's centre pixel, left to right.
fn centres(pixels: &[u8]) -> Vec<[u8; 4]> {
    let y = SIZE as usize / 2;
    (0..STRIPS)
        .map(|strip| {
            let x = (2 * strip + 1) * SIZE as usize / (2 * STRIPS);
            let at = (y * SIZE as usize + x) * 4;
            [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
        })
        .collect()
}

/// **A draw on the arrays reads its own slot.** Eight draws of eight slots, drawn once
/// through the arrays and once through the per-slot groups: every pixel equal, and the
/// strips distinct but for the one pair that shares a texture, so a draw reading another
/// slot (the instance off by one, the white fallback at 0 forgotten, a table row or a role
/// in it shifted) changes a strip. Fails, too, if a device that has the features stops
/// taking the arrays path.
#[test]
fn each_draw_on_the_binding_arrays_reads_its_own_material_slot() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let arrays = drawable(&device, &queue, true);
    if !mesh_render::bindless::supported(&device) {
        assert!(!arrays.binds_arrays(), "no features, no arrays");
        eprintln!("no binding arrays on this adapter: the per-slot path only");
        return;
    }
    assert!(arrays.binds_arrays(), "the device has the features");
    let per_slot = drawable(&device, &queue, false);
    assert!(!per_slot.binds_arrays());
    let expected = pixels(&device, &queue, &per_slot);
    let strips = centres(&expected);
    assert_eq!(strips[SHARED], strips[SHARED_WITH], "one texture, one colour");
    for (index, strip) in strips.iter().enumerate() {
        if index == SHARED {
            continue;
        }
        assert!(
            !strips[..index].contains(strip),
            "strip {index} draws {strip:?} like an earlier one: {strips:?}"
        );
    }
    let got = pixels(&device, &queue, &arrays);
    assert_eq!(centres(&got), strips, "the strips through the arrays");
    assert!(got == expected, "every pixel as the per-slot groups draw it");
}
