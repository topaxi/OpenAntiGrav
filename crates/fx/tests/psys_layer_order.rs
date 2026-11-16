//! The particle pass's draw order on a real device, read back.
//!
//! The Pulse Plasma head's arcs are drawn by the original before the dark disc
//! (a source-alpha draw over them), `docs/ghidra/functions/psp-pulse-usa/plasma.md`,
//! "the head's draw order". `Pipeline::upload_underlay` is the layer that lands
//! before the alpha-over particles; drawn after them, the arcs would wash the disc.
//!
//! **Skips when there is no adapter.**

use oag_core::math::camera::perspective;
use oag_fx::psys::Pipeline;
use oag_fx::psys::sprite::{Sheet, Sprite};
use oag_mesh::mesh::GpuVertex;
use oag_mesh::mesh_render::{DEPTH_FORMAT, Velocity};

const SIZE: u32 = 8;

/// A screen-filling quad of one flat colour, `w` away.
fn quad(sheet: &mut Sheet, colour: [f32; 4], w: f32) -> Vec<GpuVertex> {
    let rect = sheet
        .place(&Sprite {
            width: 4,
            height: 4,
            rgba: [255u8; 4].repeat(16).into(),
        })
        .expect("the sprite fits");
    let uv = [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0];
    let corner = |x: f32, y: f32| GpuVertex {
        position: [x * w, y * w, -w],
        normal: [1.0, 0.0, 0.0],
        colour,
        texcoord: uv,
        ..bytemuck::Zeroable::zeroed()
    };
    vec![
        corner(-2.0, -2.0),
        corner(2.0, -2.0),
        corner(-2.0, 2.0),
        corner(2.0, -2.0),
        corner(2.0, 2.0),
        corner(-2.0, 2.0),
    ]
}

/// The red channel at the frame's centre after `underlay` (additive), then `alpha`
/// (alpha-over), then `additive`, were drawn over black.
fn centre_red(underlay: [f32; 4], alpha: [f32; 4], additive: [f32; 4]) -> Option<u8> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu validation: {e}")));
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut pipeline = Pipeline::new(
        &device,
        format,
        oag_gpu::formats::SceneStore::Computed,
        1,
        Velocity::None,
    );
    let mut sheet = Sheet::default();
    let under = quad(&mut sheet, underlay, 10.0);
    let over = quad(&mut sheet, alpha, 10.0);
    let add = quad(&mut sheet, additive, 10.0);
    pipeline.sync_sheet(&queue, &sheet);
    let projection = perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
    pipeline.upload(&queue, &projection.to_cols_array_2d(), 1.0, &add, &over);
    pipeline.upload_underlay(&queue, &under);

    let extent = wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    };
    let texture = |label: &str, format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let target = texture(
        "target",
        format,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = texture(
        "depth",
        DEPTH_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let view = target.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("particles"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
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
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pipeline.draw(&mut pass, pipeline.drawn());
    }
    let row = 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(row * SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        extent,
    );
    queue.submit([encoder.finish()]);
    buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the readback completes");
    let bytes = buffer.slice(..).get_mapped_range().expect("range").to_vec();
    Some(bytes[(SIZE / 2 * row + SIZE / 2 * 4) as usize])
}

/// An opaque black alpha-over particle drawn after the underlay's white additive one
/// covers it. Drawn the other way round (the alpha layer first), the white would
/// land on top and the pixel would read 255.
#[test]
fn the_underlay_is_drawn_before_the_alpha_over_layer() {
    let white = [1.0; 4];
    let black = [0.0, 0.0, 0.0, 1.0];
    let none = [0.0; 4];
    let Some(red) = centre_red(white, black, none) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert!(red < 8, "the disc covers the underlay: red {red}");
}

/// The additive layer after the alpha-over one still adds over the disc.
#[test]
fn the_additive_layer_still_draws_over_the_alpha_over_layer() {
    let Some(red) = centre_red([0.0; 4], [0.0, 0.0, 0.0, 1.0], [0.5, 0.5, 0.5, 1.0]) else {
        return;
    };
    assert!(red > 100, "the glow adds over the disc: red {red}");
}
