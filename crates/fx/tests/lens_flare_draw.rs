//! The sun flare's pass over a resolved scene: it adds light around the sun's pixel where the
//! scene's depth is open there, and nothing where something stands in front of the sun.
//!
//! Dropping the pass, the atlas bind or the occlusion samples turns this red: the first case
//! finds no added light, the second finds some. The atlas is a flat opaque white, so the test
//! reads the draw's geometry and occlusion, not the authored texels.
//!
//! **Skips when there is no adapter.**

use oag_core::math::{Vec3, camera};
use oag_fx::lens_flare::{Atlas, Placement, Random, pipeline::Pipeline};
use oag_gpu::formats::SceneStore;
use oag_mesh::mesh_render::DEPTH_FORMAT;

/// 64 RGBA8 texels are one 256-byte row, the copy's alignment.
const SIZE: u32 = 64;

/// The red channel of every texel after a flare over black, `depth` the value the scene's depth
/// holds everywhere (1.0 is the sky, 0.0 something against the camera).
fn flare_over_black(depth: f32) -> Option<Vec<u8>> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu validation: {e}")));
    let atlas = Atlas {
        width: 2,
        height: 2,
        levels: vec![vec![255; 16]],
    };
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let pipeline = Pipeline::new(&device, &queue, format, SceneStore::Computed, false, &atlas)
        .expect("a 2x2 atlas builds");
    let size = wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    };
    let texture = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("lens flare draw"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let colour = texture(
        format,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth_texture = texture(
        DEPTH_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let colour_view = colour.create_view(&Default::default());
    let depth_view = depth_texture.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    // The scene's own passes: black colour, depth everywhere at `depth`.
    drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("lens flare scene"),
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
                load: wgpu::LoadOp::Clear(depth),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    }));
    // A camera looking down -Z at a sun 20 degrees off axis: in the cone, on screen.
    let view = camera::look_at(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
    let projection = camera::perspective(60f32.to_radians(), 1.0, 0.1, 100.0);
    let sun = Vec3::new(-0.3, 0.2, -1.0).normalize();
    let placement = Placement::new(
        sun.into(),
        [1.0; 3],
        view,
        projection,
        [0.0, 0.0, SIZE as f32, SIZE as f32],
        Random::FLAGGED,
    )
    .expect("the sun is in view");
    pipeline.encode(&device, &mut encoder, &colour_view, &depth_view, &placement);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("lens flare readback"),
        size: u64::from(SIZE * SIZE * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        colour.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE * 4),
                rows_per_image: None,
            },
        },
        size,
    );
    queue.submit([encoder.finish()]);
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.expect("mapped"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the readback completes");
    let bytes = readback
        .slice(..)
        .get_mapped_range()
        .expect("mapped")
        .to_vec();
    Some(bytes.chunks(4).map(|texel| texel[0]).collect())
}

#[test]
fn an_open_sun_adds_light_and_a_covered_one_adds_none() {
    let Some(open) = flare_over_black(1.0) else {
        eprintln!("no adapter: skipped");
        return;
    };
    let lit = open.iter().filter(|&&r| r > 0).count();
    assert!(lit > SIZE as usize * SIZE as usize / 2, "{lit} lit texels");
    let covered = flare_over_black(0.0).expect("the adapter was there a moment ago");
    assert!(
        covered.iter().all(|&r| r == 0),
        "light added over a covered sun"
    );
}
