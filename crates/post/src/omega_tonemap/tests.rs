//! What [`super`] is asserted to do: the law against numbers the original's
//! own microcode produced, and the chain drawn on a real device against the
//! law.

use super::law::{Curve, Params, adapt};
use super::*;

fn params() -> Params {
    Params {
        luminance_a: 0.18,
        luminance_b: 0.05,
        exposure_minimum: 0.25,
        exposure_maximum: 4.0,
        exposure_response: 3.0,
        exposure_time: 4.0,
        source_end_a: 2.0,
        source_end_b: 0.5,
        start_angle: 10.0,
        end_angle: -20.0,
    }
}

fn close(a: f32, b: f32, what: &str) {
    assert!(
        (a - b).abs() <= 1e-5 * b.abs().max(1.0),
        "{what}: {a} against {b}"
    );
}

/// `pal_ToneMapCoefficientsFilter_fp`'s 123 instructions, run on one lane of
/// a scratch GCN emulator with `ToneMap_ApplyEnvSettings`'s constants
/// (`a1t0 = b1t0 = a3s0 = b3s0 = 0`, `a4s1 = 1`, `cL = 1`), produced these
/// two texels. The law has to land on them; dropping any term - the exposure
/// target, the rate limit, either slope, the `t1` window - moves at least one.
#[test]
fn the_law_reproduces_the_original_coefficient_shader() {
    let p = params();
    let lavg = adapt(0.3, 0.4, p.step());
    close(lavg, 0.35, "LAvg");
    let curve = Curve::fit(&p, lavg);
    let expected = [0.0, 0.656_619_8, -0.068_194_72, -0.010_257_87];
    for (got, want) in curve.abcd.iter().zip(expected) {
        close(*got, want, "abcd");
    }
    close(curve.t0, 0.0, "t0");
    close(curve.t1, 2.175, "t1");
    close(curve.exposure, 0.564_285_7, "exposure");

    let p = Params {
        luminance_a: 0.3,
        luminance_b: 0.0,
        exposure_minimum: 0.5,
        exposure_maximum: 2.0,
        exposure_response: 60.0,
        source_end_a: 1.5,
        source_end_b: 0.0,
        start_angle: -15.0,
        end_angle: 30.0,
        ..params()
    };
    let curve = Curve::fit(&p, adapt(0.9, 0.9, p.step()));
    let expected = [0.0, 0.384_900_2, -0.838_556, 0.684_266_9];
    for (got, want) in curve.abcd.iter().zip(expected) {
        close(*got, want, "abcd");
    }
    close(curve.t1, 1.5, "t1");
    close(curve.exposure, 0.5, "exposure");
}

#[test]
fn the_curve_runs_from_black_to_white_across_its_window() {
    let p = params();
    let curve = Curve::fit(&p, 0.35);
    close(curve.map(0.0), 0.0, "foot");
    close(curve.map(curve.t1 / curve.exposure), 1.0, "shoulder");
    // Past the window the clamp holds it at white.
    close(curve.map(100.0), 1.0, "past the shoulder");
}

#[test]
fn history_is_a_byte_of_frames_and_never_zero() {
    let with = |exposure_time| Params {
        exposure_time,
        ..params()
    };
    assert_eq!(with(4.0).history_frames(), 240);
    assert_eq!(with(0.0).history_frames(), 1);
    // 300 frames wraps to 44, as the executable's `& 0xff` does.
    assert_eq!(with(5.0).history_frames(), 44);
    // 256 wraps to zero, which the executable reads as one.
    assert_eq!(with(256.0 / 60.0).history_frames(), 1);
}

#[test]
fn adaptation_moves_at_most_one_step_a_frame() {
    close(adapt(0.2, 5.0, 0.05), 0.25, "up");
    close(adapt(0.2, 0.0, 0.05), 0.15, "down");
    close(adapt(0.2, 0.21, 0.05), 0.21, "within a step");
    close(adapt(2000.0, 2000.0, 0.05), 1000.0, "ceiling");
}

/// Reads one pixel of an `Rgba8Unorm` texture back.
fn read_pixel(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut encoder: wgpu::CommandEncoder,
    texture: &wgpu::Texture,
    (x, y): (u32, u32),
) -> [u8; 4] {
    let row = 256;
    let size = texture.size();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(row * size.height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        size,
    );
    queue.submit([encoder.finish()]);
    buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the readback completes");
    let bytes = buffer
        .slice(..)
        .get_mapped_range()
        .expect("the mapped range")
        .to_vec();
    let at = (y * row + x * 4) as usize;
    [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]
}

/// The chain on a real device: a flat scene colour, one frame, read back.
///
/// The first frame starts settled, so `LAvg` is the scene's own luma and the
/// expected pixel is the law applied to it. **Skips when there is no
/// adapter**, so a green CI run is not evidence that it ran.
#[test]
fn the_chain_draws_the_law_on_a_real_device() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let size = (8, 8);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let p = params();
    let chain = Chain::new(&device, format, size, p).expect("the chain builds");
    let out = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("out"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let out_view = out.create_view(&Default::default());
    let colour = [0.5_f32, 0.25, 0.1];
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("scene"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: chain.scene_view(),
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: f64::from(colour[0]),
                    g: f64::from(colour[1]),
                    b: f64::from(colour[2]),
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    chain.run(
        &device,
        &queue,
        &mut encoder,
        &out_view,
        (0.0, 0.0),
        size,
        None,
    );
    let pixel = read_pixel(&device, &queue, encoder, &out, (4, 4));

    let luma: f32 = colour.iter().zip(law::LUMA).map(|(c, w)| c * w).sum();
    let curve = Curve::fit(&p, luma);
    let encode = |v: f32| (v.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0).round() as i32;
    for (channel, c) in colour.iter().enumerate() {
        let want = encode(curve.map(*c));
        let got = i32::from(pixel[channel]);
        assert!(
            (got - want).abs() <= 2,
            "channel {channel}: drew {got}, the law says {want}"
        );
    }
    // And it is not the plain encode: the curve did something here.
    let plain = encode(colour[0]);
    assert!(
        (i32::from(pixel[0]) - plain).abs() > 10,
        "red {} is the untoned encode {plain}",
        pixel[0]
    );
}

/// A half-float scene, `(0.6, 0, 0)` on its left half and `(0, 0.6, 0)` on its
/// right, run through the composite twice on a real device: once with no
/// offset texture and once with one holding `(1, 0)` everywhere.
///
/// The offset moves every sample by `DISTORT_SCALE * DISTORT_ASPECT` of the
/// width to the right, 1.14 texels at 64 wide, so the texel just left of the
/// seam reads the right half's colour - and does not without the offset.
/// **Skips when there is no adapter.**
#[test]
fn the_composite_moves_the_scene_sample_by_the_offset_texture() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let size = (64, 8);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let chain = Chain::new(&device, format, size, params()).expect("the chain builds");
    let extent = wgpu::Extent3d {
        width: size.0,
        height: size.1,
        depth_or_array_layers: 1,
    };
    let upload = |texture: &wgpu::Texture, texel: &dyn Fn(u32) -> Vec<u8>, bytes: u32| {
        let rows: Vec<u8> = (0..size.1)
            .flat_map(|_| (0..size.0).flat_map(texel))
            .collect();
        queue.write_texture(
            texture.as_image_copy(),
            &rows,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.0 * bytes),
                rows_per_image: None,
            },
            extent,
        );
    };
    // `Rgba16Float` texels of 0.6 and 0: 0x38cd and 0.
    let half = |bits: u16| bits.to_le_bytes();
    let scene = |x: u32| {
        let (r, g) = if x < size.0 / 2 {
            (0x38cd, 0)
        } else {
            (0, 0x38cd)
        };
        [half(r), half(g), half(0), half(0x3c00)].concat()
    };
    let scene_texture = chain.scene_texture();
    upload(scene_texture, &scene, 8);
    // `Rg16Float` texels of (1, 0).
    let offsets = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offsets"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: oag_gpu::formats::DISTORTION_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    upload(&offsets, &|_| [half(0x3c00), half(0)].concat(), 4);
    let offsets_view = offsets.create_view(&Default::default());

    let pixel_left_of_seam = |distortion: Option<&wgpu::TextureView>| {
        let out = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("out"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let out_view = out.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        chain.run(
            &device,
            &queue,
            &mut encoder,
            &out_view,
            (0.0, 0.0),
            size,
            distortion,
        );
        read_pixel(&device, &queue, encoder, &out, (size.0 / 2 - 1, 4))
    };
    let plain = pixel_left_of_seam(None);
    assert!(
        plain[0] > plain[1],
        "without an offset the seam's left texel is the left half's red: {plain:?}"
    );
    let moved = pixel_left_of_seam(Some(&offsets_view));
    assert!(
        moved[1] > moved[0],
        "with (1, 0) the seam's left texel reads the right half's green: {moved:?}"
    );
}
