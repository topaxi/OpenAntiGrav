//! An additive draw into the scene target, stored encoded and stored as computed.
//!
//! Wipeout HD's console packs a scene draw to sRGB before the blend unit adds it
//! (`docs/rendering/hd-track-tone.md`, "HD's blends combine encoded values"), so a
//! grey add lifts every channel of what is stored by the same encoded amount,
//! whatever it lands on. The screen flash is the plainest additive draw there is:
//! one full-screen quad, `SRC_ALPHA`/`ONE`, colour authored in display values.
//!
//! **Skips when there is no adapter.**

use oag_fx::flash::Pipeline;
use oag_gpu::formats::{SCENE_FORMAT, SceneStore};
use oag_mesh::mesh_render::{DEPTH_FORMAT, Velocity};

/// 32 half-float texels are one 256-byte row, the copy's alignment.
const WIDTH: u32 = 32;

/// What is stored before the flash: three different channels, so a lift that
/// depends on what it lands on shows as three different lifts.
const BEHIND: [f64; 3] = [0.2, 0.5, 0.8];

/// The flash's authored grey, in display values.
const GREY: f32 = 0.3;

fn half_to_f32(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let mantissa = f32::from(bits & 0x3ff);
    match exponent {
        0 => sign * mantissa * 2f32.powi(-24),
        31 => sign * f32::INFINITY,
        _ => sign * (1.0 + mantissa / 1024.0) * 2f32.powi(exponent - 15),
    }
}

/// Clears a scene-format target to [`BEHIND`], draws a [`GREY`] flash built for
/// `store` and returns the first texel's RGB.
fn flash_over(store: SceneStore) -> Option<[f32; 3]> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu validation: {e}")));
    let mut flash = Pipeline::new(&device, SCENE_FORMAT, store, 1, Velocity::None);
    flash.upload(&queue, Some([GREY, GREY, GREY, 1.0]));
    let size = wgpu::Extent3d {
        width: WIDTH,
        height: 1,
        depth_or_array_layers: 1,
    };
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("encoded blend"),
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
            label: Some("encoded blend"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &colour_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: BEHIND[0],
                        g: BEHIND[1],
                        b: BEHIND[2],
                        a: 1.0,
                    }),
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
        flash.draw(&mut pass, flash.drawn());
    }
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("encoded blend readback"),
        size: u64::from(WIDTH) * 8,
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
    let channel = |i: usize| half_to_f32(u16::from_le_bytes([bytes[2 * i], bytes[2 * i + 1]]));
    Some([channel(0), channel(1), channel(2)])
}

fn lifts(texel: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|c| texel[c] - BEHIND[c] as f32)
}

/// Stored encoded, the grey adds its display value to every channel alike.
#[test]
fn an_encoded_scene_adds_the_display_value_whatever_it_lands_on() {
    let Some(texel) = flash_over(SceneStore::Encoded) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    for lift in lifts(texel) {
        assert!((lift - GREY).abs() < 4e-3, "{texel:?}: lift {lift}");
    }
}

/// Stored as computed, the same flash adds its linear value, `0.3^2.2`: the
/// law the encoded store replaces on HD, and still Omega's.
#[test]
fn a_computed_scene_adds_the_linear_value() {
    let Some(texel) = flash_over(SceneStore::Computed) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    for lift in lifts(texel) {
        assert!(
            (lift - GREY.powf(2.2)).abs() < 4e-3,
            "{texel:?}: lift {lift}"
        );
    }
}
