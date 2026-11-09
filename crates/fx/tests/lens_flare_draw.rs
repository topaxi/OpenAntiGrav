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

/// The red channel of every texel after a flare over black, once per sun in `suns`, each drawn
/// and submitted on its own as a split's views are. `depth` is the value the scene's depth
/// holds everywhere (1.0 is the sky, 0.0 something against the camera); `fresh_pipeline` builds
/// a new pipeline for every sun instead of reusing the one.
fn flares_over_black(
    depth: f32,
    tint: f32,
    suns: &[Vec3],
    fresh_pipeline: bool,
) -> Option<Vec<Vec<u8>>> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu validation: {e}")));
    let atlas = Atlas {
        width: 2,
        height: 2,
        levels: vec![vec![255; 16]],
    };
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let build = || {
        Pipeline::new(&device, &queue, format, SceneStore::Computed, false, &atlas)
            .expect("a 2x2 atlas builds")
    };
    let mut pipeline = build();
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
    // One depth view for every draw: a reused pipeline keeps its bind group while the view
    // is the same, which is the case a stale uniform would show in.
    let depth_texture = texture(
        DEPTH_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let depth_view = depth_texture.create_view(&Default::default());
    let mut out = Vec::new();
    for sun in suns {
        if fresh_pipeline {
            pipeline = build();
        }
        let colour = texture(
            format,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );
        let colour_view = colour.create_view(&Default::default());
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
        let placement = Placement::new(
            sun.normalize().into(),
            [tint; 3],
            view,
            projection,
            [0.0, 0.0, SIZE as f32, SIZE as f32],
            Random::FLAGGED,
        )
        .expect("the sun is in view");
        pipeline.encode(
            &device,
            &queue,
            &mut encoder,
            &colour_view,
            &depth_view,
            &placement,
        );
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
        out.push(bytes.chunks(4).map(|texel| texel[0]).collect());
    }
    Some(out)
}

const OPEN_SUN: Vec3 = Vec3::new(-0.3, 0.2, -1.0);

#[test]
fn an_open_sun_adds_light_and_a_covered_one_adds_none() {
    let Some(open) = flares_over_black(1.0, 1.0, &[OPEN_SUN], false) else {
        eprintln!("no adapter: skipped");
        return;
    };
    let lit = open[0].iter().filter(|&&r| r > 0).count();
    assert!(lit > SIZE as usize * SIZE as usize / 2, "{lit} lit texels");
    let covered = flares_over_black(0.0, 1.0, &[OPEN_SUN], false)
        .expect("the adapter was there a moment ago");
    assert!(
        covered[0].iter().all(|&r| r == 0),
        "light added over a covered sun"
    );
}

/// The uniform buffer is kept between draws, so a second draw must overwrite the first's
/// rings: each frame of one pipeline equals the same sun drawn by a pipeline of its own.
#[test]
fn a_reused_pipeline_draws_each_sun_as_a_fresh_one_does() {
    let suns = [
        OPEN_SUN,
        Vec3::new(0.4, -0.1, -1.0),
        Vec3::new(-0.3, 0.2, -1.0),
    ];
    let Some(reused) = flares_over_black(1.0, 0.1, &suns, false) else {
        eprintln!("no adapter: skipped");
        return;
    };
    let fresh =
        flares_over_black(1.0, 0.1, &suns, true).expect("the adapter was there a moment ago");
    assert_ne!(fresh[0], fresh[1], "the two suns must draw differently");
    assert_eq!(reused, fresh);
}
