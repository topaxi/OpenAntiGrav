//! [`Chain::run`]'s timestamp pair, end to end.
//!
//! Needs a real device and a real readback, on the same terms
//! `crates/gpu/src/timing/timer/tests.rs`'s own GPU test does: every
//! adapter that claims the feature, skipping those that do not. What this
//! pins is the wiring alone - that the opening write lands on the chain's
//! first pass and the closing one on its last, and that a claimed slot
//! resolves to a positive duration naming the right frame - not the chain's
//! visual output, which is a different kind of test.

use super::*;

/// One circuit's worth of parameters, none of it load-bearing for this test -
/// only that the numbers are finite, so the constants buffer round-trips
/// without producing NaN uniforms a validation layer would reject.
fn params() -> Params {
    Params {
        alpha_contribution: 1.0,
        frame_contribution: 1.0,
        frame_exponent: 1.0,
        horizontal_size: 1.0,
        vertical_size: 1.0,
        adaption_rate: 0.1,
        adaption_boost: 1.0,
        tone_adaption_boost: 1.0,
        tone_darkening_clamp: 1.0,
        tone_maximum_brightness: 1.0,
    }
}

/// The chain's own timestamp pair reads back a positive duration naming the
/// frame it was claimed for, on both the first frame and a second one - the
/// second is what proves a slot is reusable rather than only that claiming
/// one once works, the same reason `timing::timer::tests` runs its own probe
/// twice.
#[test]
fn the_chain_reads_back_naming_its_own_frame() {
    let instance = wgpu::Instance::default();
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY));
    if adapters.is_empty() {
        eprintln!("no GPU adapter: skipping");
        return;
    }
    for adapter in &adapters {
        let info = adapter.get_info();
        let timing = oag_gpu::timing::Timing::of(adapter);
        if !timing.can_time_a_pass() {
            eprintln!("{}: {}, skipping", info.name, timing.describe());
            continue;
        }
        time_the_chain(adapter, &info.name);
    }
}

fn time_the_chain(adapter: &wgpu::Adapter, name: &str) {
    let features = oag_gpu::timing::Timing::of(adapter).features();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("hd bloom timing test"),
        required_features: features,
        ..Default::default()
    }))
    .expect("a device with what was probed");

    let output_format = wgpu::TextureFormat::Rgba8Unorm;
    let size = (16u32, 16u32);
    let chain = Chain::new(&device, output_format, size, params(), Glow::Drawn)
        .expect("the pipelines build");
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hd bloom timing test output"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: output_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = output.create_view(&Default::default());

    let mut timer = oag_gpu::timing::PassTimer::new(&device, &queue)
        .expect("a timer on a device that has the bit");

    for frame in [21u64, 22] {
        timer.begin(frame);
        let mut encoder = device.create_command_encoder(&Default::default());
        // The chain reads whatever `scene_view()` last had drawn into it -
        // its own cleared-on-creation contents here, since this test is
        // about the timing wiring and not the picture.
        let timestamps = Some(ChainTimestamps {
            begin: timer
                .half_writes(oag_gpu::timing::Half::Begin)
                .expect("claimed above"),
            end: timer
                .half_writes(oag_gpu::timing::Half::End)
                .expect("claimed above"),
        });
        chain.run(&queue, &mut encoder, &view, (0.0, 0.0), size, timestamps);
        timer.resolve(&mut encoder);
        queue.submit(Some(encoder.finish()));

        let mut reading = None;
        for _ in 0..1_000 {
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("the GPU");
            if let Some(got) = timer.read(&device) {
                reading = Some(got);
                break;
            }
        }
        let reading = reading.unwrap_or_else(|| panic!("{name}: frame {frame} never came back"));
        assert_eq!(
            reading.frame, frame,
            "{name}: the reading named the wrong frame"
        );
        // The ceiling only rejects a garbage reading (a wrapped or unscaled
        // counter), not a slow one: llvmpipe on a loaded machine measured this
        // 16x16 chain at 1.07 s during a gate.
        assert!(
            reading.seconds > 0.0 && reading.seconds < 60.0,
            "{name}: the chain measured {} s",
            reading.seconds
        );
        eprintln!(
            "{name}: hd bloom chain, frame {frame}: {:.3} ms",
            reading.seconds * 1000.0
        );
    }
}

/// The player's `[graphics] bloom` switch reaches this chain.
///
/// It did not until 2026-09-09: `race::Scene` gated only `post::bloom`, the
/// PSP chain, so Wipeout HD - the only title whose bloom *this* module draws -
/// bloomed whatever the player had set. Measured on the Talon's Junction grid
/// at the time: flipping the setting moved a Pulse frame's clipped-white share
/// from 2.07 % to 0.41 % and left an HD frame byte-identical. See
/// [`Glow`].
///
/// A white scene is what makes the two runs separable at all: the resolve is
/// `saturate(scene * exposure + bloom)`, so a black scene resolves to the same
/// black either way and would pass this test with the switch unwired.
#[test]
fn suppressing_the_glow_dims_the_resolve() {
    let instance = wgpu::Instance::default();
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY));
    if adapters.is_empty() {
        eprintln!("no GPU adapter: skipping");
        return;
    }
    let adapter = &adapters[0];
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
        .expect("a device with no extra features");
    let drawn = resolve_a_white_scene(&device, &queue, Glow::Drawn);
    let suppressed = resolve_a_white_scene(&device, &queue, Glow::Suppressed);
    assert!(
        suppressed < drawn,
        "the suppressed glow resolved no dimmer than the drawn one \
         ({suppressed} against {drawn}) - the switch is not reaching the chain",
    );
}

/// Clears the chain's scene target to white, runs the chain, and returns the
/// mean of the resolved frame's red channel.
///
/// 64x64 because a readback's rows are 256-byte aligned and that is exactly
/// one `Rgba8Unorm` row, so the copy needs no padding arithmetic.
fn resolve_a_white_scene(device: &wgpu::Device, queue: &wgpu::Queue, glow: Glow) -> f64 {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let size = (64u32, 64u32);
    let chain = Chain::new(device, format, size, params(), glow).expect("the pipelines build");
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hd bloom glow test output"),
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
    let view = output.create_view(&Default::default());
    let bytes = u64::from(size.0) * u64::from(size.1) * 4;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hd bloom glow test readback"),
        size: bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("hd bloom glow test scene"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: chain.scene_view(),
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    chain.run(queue, &mut encoder, &view, (0.0, 0.0), size, None);
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &output,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.0 * 4),
                rows_per_image: Some(size.1),
            },
        },
        wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |r| {
        r.expect("the readback maps");
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the GPU");
    let data = readback
        .slice(..)
        .get_mapped_range()
        .expect("the buffer is mapped");
    let sum: f64 = data.iter().step_by(4).map(|&byte| f64::from(byte)).sum();
    let mean = sum / f64::from(size.0 * size.1);
    drop(data);
    readback.unmap();
    mean
}

/// The "hd encode" pass writes at the scene's own letterbox offset, not
/// always at the canvas's `(0, 0)`.
///
/// The scene target is uniformly white, so a correct and a buggy placement
/// would resolve to the same *value* wherever each draws - only *where* they
/// draw differs, which is what this pins. `viewport` (32x32) is drawn at
/// `origin` (16, 8) inside a 64x64 canvas, so the pre-fix placement (always
/// `(0, 0)`) and the correct one overlap on `[16, 32) x [8, 32)` and disagree
/// on `[0, 16) x [0, 8)`: a pixel there reads bright under the bug (the
/// misplaced draw lands on it) and stays at the pass's own clear colour once
/// `origin` is honoured. See `oag_post::hd_bloom::Chain::run`'s own
/// doc comment for the black-strip evidence this mirrors.
#[test]
fn the_encode_pass_honours_the_scenes_own_offset() {
    let instance = wgpu::Instance::default();
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY));
    if adapters.is_empty() {
        eprintln!("no GPU adapter: skipping");
        return;
    }
    let adapter = &adapters[0];
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
        .expect("a device with no extra features");

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let canvas = (64u32, 64u32);
    let origin = (16.0f32, 8.0f32);
    let viewport = (32u32, 32u32);
    let chain =
        Chain::new(&device, format, canvas, params(), Glow::Drawn).expect("the pipelines build");
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hd bloom offset test output"),
        size: wgpu::Extent3d {
            width: canvas.0,
            height: canvas.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = output.create_view(&Default::default());
    let bytes = u64::from(canvas.0) * u64::from(canvas.1) * 4;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hd bloom offset test readback"),
        size: bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    // The whole internal scene is white - only the resolve's *placement* is
    // under test, not what it reads.
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("hd bloom offset test scene"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: chain.scene_view(),
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    chain.run(&queue, &mut encoder, &view, origin, viewport, None);
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &output,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(canvas.0 * 4),
                rows_per_image: Some(canvas.1),
            },
        },
        wgpu::Extent3d {
            width: canvas.0,
            height: canvas.1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |r| {
        r.expect("the readback maps");
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the GPU");
    let data = readback
        .slice(..)
        .get_mapped_range()
        .expect("the buffer is mapped");
    let red_at = |x: u32, y: u32| -> u8 { data[((y * canvas.0 + x) * 4) as usize] };
    // Inside the true rectangle: the resolved white scene, unmistakably lit.
    let inside = red_at(32, 24);
    // Inside the pre-fix rectangle (`(0, 0)`-anchored) but outside the true
    // one - bright only if the encode pass ignored `origin`.
    let mislanded = red_at(4, 4);
    // Outside both rectangles either way - a sanity check that nothing is
    // drawing everywhere regardless of viewport.
    let untouched = red_at(60, 60);
    drop(data);
    readback.unmap();

    assert!(
        inside > 50,
        "the true rectangle ({inside}) should read the resolved white scene"
    );
    assert_eq!(
        mislanded, 0,
        "a pixel outside the true rectangle read {mislanded}, not 0 - the \
         encode pass drew at the canvas origin instead of the scene's own \
         offset"
    );
    assert_eq!(
        untouched, 0,
        "a pixel outside every candidate rectangle should stay at the \
         pass's own clear colour"
    );
}

/// The luminance reduction is an exact linear mean whose result is read as
/// encoded bytes, whatever the level sizes.
///
/// Half of an 80x76 scene is linear 0 and half 0.4, so the ladder's odd levels
/// (20x19, 10x10, 5x5, 3x3, 2x2) must still return 0.2 and the readback encodes
/// it: `adapted = 0.2^(1/2.2)`. The resolve's exposure `max - adapted * boost`
/// turns that into the right half's byte (203). A mean of the *linear* scene
/// (0.2), a mean taken after encoding each texel (0.33) or a ladder that
/// dropped a row or column (205 on the edge-duplicating one) all land more than
/// the tolerance away. See `renderer.md`, "The adaptation reads encoded bytes".
#[test]
fn the_adaptation_is_the_encoded_linear_mean() {
    let instance = wgpu::Instance::default();
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY));
    if adapters.is_empty() {
        eprintln!("no GPU adapter: skipping");
        return;
    }
    let adapter = &adapters[0];
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
        .expect("a device with no extra features");

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let size = (80u32, 76u32);
    let chain_params = Params {
        tone_adaption_boost: 1.0,
        tone_darkening_clamp: 10.0,
        tone_maximum_brightness: 2.0,
        ..params()
    };
    let chain = Chain::new(&device, format, size, chain_params, Glow::Suppressed)
        .expect("the pipelines build");
    // The left half black, the right half 0.4 (as the half float 0x3666,
    // 0.3999023), written straight into the scene texture.
    let value = |linear: f32| (if linear == 0.0 { 0u16 } else { 0x3666 }).to_le_bytes();
    let mut texels = Vec::new();
    for _row in 0..size.1 {
        for column in 0..size.0 {
            let linear = if column < size.0 / 2 { 0.0 } else { 0.4 };
            for channel in [linear, linear, linear, 0.0] {
                texels.extend_from_slice(&value(channel));
            }
        }
    }
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: chain.scene_texture(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &texels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size.0 * 8),
            rows_per_image: Some(size.1),
        },
        wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
    );
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hd bloom adaptation test output"),
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
    let view = output.create_view(&Default::default());
    let stride = (size.0 * 4).next_multiple_of(256);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hd bloom adaptation test readback"),
        size: u64::from(stride) * u64::from(size.1),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    chain.run(&queue, &mut encoder, &view, (0.0, 0.0), size, None);
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &output,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(size.1),
            },
        },
        wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |r| {
        r.expect("the readback maps");
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the GPU");
    let data = readback
        .slice(..)
        .get_mapped_range()
        .expect("the buffer is mapped");
    let at = (size.1 / 2 * stride + (size.0 * 3 / 4) * 4) as usize;
    let got = f64::from(data[at]);
    drop(data);
    readback.unmap();

    let stored = 0.399_902_3f64;
    let adapted = (stored / 2.0).powf(1.0 / 2.2);
    let scale = 2.0 - adapted;
    let want = (stored * scale).min(1.0).powf(1.0 / 2.2) * 255.0;
    assert!(
        (got - want).abs() <= 2.5,
        "the right half resolved to byte {got}, the encoded linear mean predicts {want:.1}"
    );
}
