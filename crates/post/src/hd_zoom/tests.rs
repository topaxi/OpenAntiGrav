//! The pulse laws against the numbers read off the running original, the ring
//! against its live CPU mesh, and the pass end to end on a flat scene.

use super::*;
use crate::hd_bloom::{Chain, Glow, Params};

/// HD's numbers, as `oag_hd::race::ZOOM_RING` carries them.
fn tuning() -> Tuning {
    Tuning {
        history_weight: 0.25,
        history_weight_cap: 0.95,
        history_crop: 0.05,
        inner_radius: 0.62,
        outer_radius: 1.5,
        segments: 24,
        tap_pull: [0.15, 0.95],
        boost_start: 0.8,
        boost_fall_rate: 8.0 / 7.0,
        boost_slope: 1.25,
        boost_offset: 0.075,
        boost_gain: 5.0,
        boost_decay_per_update: 0.035,
        damage_tint: [0.9, 0.08],
        damage_decay_per_second: 1.666_7,
        jitter_retain: 0.75,
        jitter_step: 0.000_2,
        jitter_range: 100,
    }
}

const DT: f32 = 1.0 / 60.0;

/// `E` after `updates` updates of a boost fired this tick.
fn boost_after(updates: u32) -> f32 {
    let mut pulse = Pulse::new(tuning());
    pulse.fire_boost();
    for _ in 0..updates {
        pulse.advance(DT);
    }
    pulse.frame().boost
}

/// The ramp, read on RPCS3 at 2 ms steps from the glow state and the live `E`
/// (the capture's `A0` and `E` columns): `A0 = 0.7238 -> E = 0.1011`, `0.7048 ->
/// 0.2202`, `0.6857 -> 0.3394`, and `E = 1.0` once `A0` is at or below 0.58.
/// `A0` falls `8/7 * dt` per update, so those are the fourth, fifth and sixth
/// update after the trigger and the twelfth.
#[test]
fn the_boost_ramp_is_the_one_read_off_the_original() {
    for (updates, read) in [(4, 0.1011), (5, 0.2202), (6, 0.3394), (7, 0.4622)] {
        let e = boost_after(updates);
        assert!(
            (e - read).abs() < 0.005,
            "{updates} updates in, E is {e}, the original read {read}"
        );
    }
    assert!(
        (boost_after(12) - 1.0).abs() < 1e-6,
        "the ramp tops out at 1"
    );
}

/// The hold ends when `A0` runs out, 0.7 s after the trigger, and `E` then
/// falls `0.035` per **update** - the live reads `0.965, 0.790, 0.685` are steps
/// of exactly that - to zero by about 1.2 s.
#[test]
fn the_boost_holds_then_decays_per_update() {
    assert!(
        (boost_after(30) - 1.0).abs() < 1e-6,
        "still holding at 0.5 s"
    );
    let after = |n| boost_after(n);
    let steps: Vec<f32> = (44..48).map(after).collect();
    for pair in steps.windows(2) {
        assert!(
            ((pair[0] - pair[1]) - 0.035).abs() < 1e-5,
            "the decay is not 0.035 an update: {steps:?}"
        );
    }
    assert_eq!(boost_after(80), 0.0, "the pulse is over by 1.33 s");
    let live = Pulse::new(tuning()).frame();
    assert!(!live.active(), "a quiet pulse draws no ring");
}

/// `P` is raised to the larger of itself and the hit, and decays `1.6667` a
/// second.
#[test]
fn damage_raises_to_the_larger_and_decays_by_the_second() {
    let mut pulse = Pulse::new(tuning());
    pulse.hit(0.3);
    pulse.hit(0.2);
    assert!((pulse.frame().damage - 0.3).abs() < 1e-6);
    pulse.hit(5.0);
    assert_eq!(pulse.frame().damage, 1.0, "the raise is clamped to 1");
    for _ in 0..30 {
        pulse.advance(DT);
    }
    assert!(
        (pulse.frame().damage - (1.0 - 0.5 * 1.666_7)).abs() < 1e-3,
        "half a second later P is {}",
        pulse.frame().damage
    );
}

/// The history draw's alpha and crop at the values the buffers were fitted at:
/// `E = 1` gave `w = 0.25`, `c = 0.05`, and the cap is only the executable's.
#[test]
fn the_history_weight_and_crop_follow_e() {
    let t = tuning();
    assert_eq!(history(&t, 0.0), (0.0, 0.0));
    let (w, c) = history(&t, 1.0);
    assert!((w - 0.25).abs() < 1e-6 && (c - 0.05).abs() < 1e-6);
    assert_eq!(history(&t, 10.0).0, 0.95, "the weight is capped");
}

/// The ring against the live mesh read from RPCS3 (record 0, 1 and 23 of
/// `FunkLayer + 0x130`'s object, `E = 0.825`): positions, both taps' texture
/// coordinates and the colour of each of a quad's four vertices.
#[test]
fn the_ring_is_the_live_cpu_mesh() {
    let frame = Frame {
        boost: 0.825,
        damage: 0.0,
        size: [1.0, 1.0],
        tick: 1,
    };
    let mesh = ring(&tuning(), &frame);
    assert_eq!(mesh.len(), 96);
    let near = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 2e-4 && (a[1] - b[1]).abs() < 2e-4;
    let r0 = &mesh[0..4];
    assert!(near(r0[0].position, [0.0, 0.62]) && near(r0[0].uv0, [0.5, 0.19]));
    assert!(near(r0[1].position, [0.0, 1.5]));
    assert!(near(r0[1].uv0, [0.5, -0.184]) && near(r0[1].uv1, [0.5, 0.168]));
    assert!(near(r0[2].position, [0.3882, 1.4489]));
    assert!(near(r0[2].uv0, [0.6770, -0.1607]) && near(r0[2].uv1, [0.5859, 0.1793]));
    assert!(near(r0[3].position, [0.1605, 0.5989]) && near(r0[3].uv1, [0.5802, 0.2006]));
    assert_eq!(r0[0].colour, [1.0, 1.0, 1.0, 0.0]);
    assert_eq!(r0[1].colour, [1.0, 1.0, 1.0, 0.825]);
    let r23 = &mesh[92..96];
    assert!(near(r23[0].position, [-0.1605, 0.5989]));
    assert!(near(r23[2].position, [0.0, 1.5]) && near(r23[3].position, [0.0, 0.62]));
}

/// The damage tint on the live mesh: `P = 0.8238` read `(0.259, 0.934, 1.0)` and
/// `P = 0.3445` read `(0.690, 0.972, 1.0)`, alpha the larger of `E` and `P`.
#[test]
fn the_damage_tint_is_the_live_one() {
    for (p, rgb) in [(0.8238, [0.259, 0.934, 1.0]), (0.3445, [0.690, 0.972, 1.0])] {
        let frame = Frame {
            boost: 0.37,
            damage: p,
            size: [1.0; 2],
            tick: 1,
        };
        let outer = ring(&tuning(), &frame)[1].colour;
        for k in 0..3 {
            assert!((outer[k] - rgb[k]).abs() < 1e-3, "P {p}: {outer:?}");
        }
        assert_eq!(outer[3], p.max(0.37));
    }
}

/// The size jitter keeps inside its bound `0.079 E` and is exactly 1 with no
/// pulse.
#[test]
fn the_size_jitter_stays_inside_its_bound() {
    let mut pulse = Pulse::new(tuning());
    for _ in 0..50 {
        pulse.advance(DT);
    }
    assert_eq!(pulse.frame().size, [1.0, 1.0]);
    pulse.fire_boost();
    for _ in 0..40 {
        pulse.advance(DT);
        let size = pulse.frame().size;
        assert!(size.iter().all(|s| (1.0..=1.08).contains(s)), "{size:?}");
    }
}

fn params() -> Params {
    Params {
        alpha_contribution: 1.0,
        frame_contribution: 1.0,
        frame_exponent: 1.0,
        horizontal_size: 1.0,
        vertical_size: 1.0,
        adaption_rate: 0.1,
        adaption_boost: 1.0,
        tone_adaption_boost: 0.0,
        tone_darkening_clamp: 1.0,
        tone_maximum_brightness: 1.0,
        zoom: Some(tuning()),
    }
}

/// Resolves a flat grey scene once per frame of `frames`, one chain throughout so
/// the history carries from one to the next, and returns the last frame's corner
/// pixel and centre pixel (red). `None` with no adapter.
fn resolve(frames: &[Option<Frame>]) -> Option<(u8, u8)> {
    let instance = wgpu::Instance::default();
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY));
    let adapter = adapters.first()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).ok()?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let size = (64u32, 64u32);
    let chain = Chain::new(&device, format, size, params(), Glow::Suppressed).expect("builds");
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hd zoom test output"),
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
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hd zoom test readback"),
        size: u64::from(size.0 * size.1 * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    for (n, &frame) in frames.iter().enumerate() {
        let last = n + 1 == frames.len();
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("hd zoom test scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: chain.scene_view(),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.2,
                        g: 0.2,
                        b: 0.2,
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
        chain.set_zoom_frame(frame);
        chain.run(&queue, &mut encoder, &view, (0.0, 0.0), size, None);
        if last {
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
        }
        queue.submit(Some(encoder.finish()));
    }
    readback.slice(..).map_async(wgpu::MapMode::Read, |r| {
        r.expect("the readback maps");
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the GPU");
    let data = readback.slice(..).get_mapped_range().expect("mapped");
    let at = |x: u32, y: u32| data[((y * size.0 + x) * 4) as usize];
    Some((at(1, 1), at(32, 32)))
}

/// A boost brightens the periphery of a flat scene and leaves the centre alone:
/// at the corner the ring's alpha is `1 - (1 - 0.9)^2`, and two taps of a flat
/// scene sum to twice it, so the corner of a `0.2` scene goes from `s` toward
/// `2 s`; the centre sits inside the transparent inner radius.
#[test]
fn a_boost_brightens_the_rim_and_not_the_centre() {
    let pulse = Frame {
        boost: 1.0,
        damage: 0.0,
        size: [1.0, 1.0],
        tick: 1,
    };
    let quiet = Frame {
        boost: 0.0,
        tick: 1,
        ..pulse
    };
    let pulse = Frame { tick: 2, ..pulse };
    let (Some((corner_off, centre_off)), Some((corner_on, centre_on))) =
        (resolve(&[None]), resolve(&[Some(quiet), Some(pulse)]))
    else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert_eq!(centre_on, centre_off, "the centre moved");
    assert!(
        f32::from(corner_on) > 1.3 * f32::from(corner_off),
        "the rim did not brighten: {corner_off} -> {corner_on}"
    );
}

/// With no pulse the ring draws nothing: the frame is the chain's own.
#[test]
fn a_quiet_pulse_changes_nothing() {
    let quiet = Frame {
        boost: 0.0,
        damage: 0.0,
        size: [1.0, 1.0],
        tick: 1,
    };
    let (Some(off), Some(on)) = (resolve(&[None]), resolve(&[Some(quiet)])) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert_eq!(off, on);
}
