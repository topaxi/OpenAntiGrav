//! Wipeout HD's engine-trail tube on HD's race target: its two colour taps are
//! sRGB-decoded on fetch before the scene stores the encoded product.
//!
//! Units 0 and 1 of the tube's live draws carry gamma nibble 7 (RGB decoded,
//! alpha raw; `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`, "The
//! trail's colour taps are decoded on fetch"), and the packer encodes the
//! program's output before the blend, so a uniform texel `t` stores in
//! proportion to `enc(dec(t)) = t`: half the texel, half the stored colour,
//! whatever the fades and the blend do to both alike. Raw taps would store in
//! proportion to `t^(1/2.2)`. The ratio of two draws is what is pinned, so the
//! facing and depth fades need no exact value.
//!
//! **Skips when there is no adapter.**

use oag_core::math::Vec3;
use oag_fx::exhaust::{self, FlareTexture};
use oag_gpu::formats::{SCENE_FORMAT, SceneStore};
use oag_mesh::mesh::{GpuVertex, slots};
use oag_mesh::mesh_render::{DEPTH_FORMAT, Velocity};

/// 32 half-float texels are one 256-byte row, the copy's alignment.
const WIDTH: u32 = 32;

fn half_to_f32(bits: u16) -> f32 {
    let exponent = i32::from((bits >> 10) & 0x1f);
    let mantissa = f32::from(bits & 0x3ff);
    match exponent {
        0 => mantissa * 2f32.powi(-24),
        _ => (1.0 + mantissa / 1024.0) * 2f32.powi(exponent - 15),
    }
}

fn uniform(grey: u8, alpha: u8) -> FlareTexture {
    FlareTexture {
        width: 4,
        height: 4,
        rgba: [grey, grey, grey, alpha].repeat(16),
    }
}

fn vertex(position: [f32; 3]) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        // Facing the camera, so the fin's edge-on fade is 1.
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        // The blue-to-red mix: none, so the colour is the blue texture's.
        lit: 0.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
        tangent: 0,
    }
}

/// Draws one full-width triangle of the tube with both textures a uniform
/// `grey` (the noise's alpha 0, the coverage's 255) into an encoded HD scene
/// target cleared to black, and returns the stored red channel.
fn stored(device: &wgpu::Device, queue: &wgpu::Queue, grey: u8) -> f32 {
    let mut pipeline = exhaust::Pipeline::new(
        device,
        queue,
        SCENE_FORMAT,
        SceneStore::Encoded,
        &uniform(255, 255),
        &uniform(grey, 0),
        Some(&uniform(grey, 255)),
        exhaust::TRAIL_BLEND,
        1,
        Velocity::None,
        false,
    );
    let identity = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let trail = [
        vertex([-3.0, -1.0, 0.5]),
        vertex([3.0, -1.0, 0.5]),
        vertex([0.0, 3.0, 0.5]),
    ];
    pipeline.upload(queue, &identity, Vec3::new(0.0, 0.0, 10.0), &[], &trail);
    let size = wgpu::Extent3d {
        width: WIDTH,
        height: 1,
        depth_or_array_layers: 1,
    };
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hd trail tap"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let colour = target(
        SCENE_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(DEPTH_FORMAT, wgpu::TextureUsages::RENDER_ATTACHMENT);
    let colour_view = colour.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("hd trail tap"),
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
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hd trail tap readback"),
        size: u64::from(WIDTH * 8),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        colour.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(WIDTH * 8),
                rows_per_image: None,
            },
        },
        size,
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");
    let mapped = readback.slice(..).get_mapped_range().expect("mapping");
    // The middle texel of the row, which the triangle covers.
    let at = (WIDTH / 2 * 8) as usize;
    half_to_f32(u16::from_le_bytes([mapped[at], mapped[at + 1]]))
}

#[test]
fn hds_trail_taps_are_decoded_before_the_encoded_store() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu validation: {e}")));
    let full = stored(&device, &queue, 255);
    let half = stored(&device, &queue, 128);
    assert!(full > 0.05, "the tube drew nothing: {full}");
    let ratio = half / full;
    // Decoded on fetch: 128/255 = 0.502. Raw: 0.502^(1/2.2) = 0.731.
    assert!((ratio - 0.502).abs() < 0.02, "{half} / {full} = {ratio}");
}
