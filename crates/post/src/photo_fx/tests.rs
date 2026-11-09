//! The focus law, the uniform, and the pass drawn on a real device.

use super::*;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1.0e-3 * b.abs().max(1.0)
}

#[test]
fn the_focus_curve_runs_three_to_four_hundred_ninety_three() {
    assert!(close(Edges::new(0.0).distance, 3.0));
    assert!(close(Edges::new(0.5).distance, 23.0));
    assert!(close(Edges::new(1.0).distance, 493.0));
    assert!(close(Edges::new(-1.0).distance, 3.0), "clamped");
    // Monotone across the whole slider.
    let mut last = 0.0;
    for step in 0..=100 {
        let d = Edges::new(step as f32 * 0.01).distance;
        assert!(d >= last, "step {step}: {d} < {last}");
        last = d;
    }
}

#[test]
fn the_edges_bracket_the_focus_with_hds_exponents() {
    let e = Edges::new(0.2);
    let c = e.distance.powf(0.55);
    let wide = c.powf(1.7);
    assert!(close(e.near_focus, e.distance - c));
    assert!(close(e.far_focus, e.distance + 2.0 * c));
    assert!(close(e.near_edge, e.distance - (c + wide)));
    assert!(close(e.far_edge, e.distance + 2.0 * (c + wide)));
    assert!(e.near_edge < e.near_focus && e.far_focus < e.far_edge);
}

#[test]
fn nothing_to_apply_is_inactive_and_neutral_values_are_nothing() {
    let none = Params {
        grade: None,
        dof: None,
        near: 1.0,
        far: 1000.0,
    };
    assert!(!none.active());
    let neutral = Params {
        grade: Some(Grade {
            exposure: 0.0,
            saturation: 1.0,
        }),
        ..none
    };
    assert!(!neutral.active(), "a neutral grade is not applied");
    let moved = Params {
        grade: Some(Grade {
            exposure: 0.1,
            saturation: 1.0,
        }),
        ..none
    };
    assert!(moved.active());
    let focus = Params {
        dof: Some(Dof {
            focus: 0.2,
            aperture: 0.4,
        }),
        ..none
    };
    assert!(focus.active());
}

#[test]
fn the_uniform_is_sixty_four_bytes_and_a_neutral_grade_switches_the_colour_off() {
    assert_eq!(size_of::<Constants>(), 64);
    let params = Params {
        grade: Some(Grade {
            exposure: 0.0,
            saturation: 1.0,
        }),
        dof: Some(Dof {
            focus: 0.2,
            aperture: 1.0,
        }),
        near: 1.0,
        far: 500.0,
    };
    let c = Constants::new((0.0, 0.0, 1920.0, 1080.0), (1920, 1080), &params);
    assert_eq!(c.rect, [0, 0, 1919, 1079]);
    assert_eq!(c.grade[3], 0.0, "the colour pass is off at neutral");
    assert_eq!(c.depth[3], 1.0);
    assert!(close(c.depth[2], Dof::MAX_RADIUS * 1080.0));
}

struct Device {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn device() -> Option<Device> {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    Some(Device { device, queue })
}

const SIZE: u32 = 64;
const NEAR: f32 = 1.0;
const FAR: f32 = 1000.0;

/// The 0..1 depth of a perspective projection for an eye distance.
fn z_of(distance: f32) -> f32 {
    FAR * (distance - NEAR) / (distance * (FAR - NEAR))
}

struct Fixture {
    scene: wgpu::Texture,
    view: wgpu::TextureView,
    depth: wgpu::TextureView,
}

/// A frame of one-pixel black and white stripes, the left half at eye distance `left` and the
/// right half at `right`. Written as `Rgba8Unorm`, so no transfer curve sits between a byte
/// written and a byte asserted.
fn fixture(d: &Device, left: f32, right: f32) -> Fixture {
    let scene = d.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let mut pixels = Vec::new();
    for _ in 0..SIZE {
        for x in 0..SIZE {
            let v = if x % 2 == 0 { 255 } else { 0 };
            pixels.extend_from_slice(&[v, v, v, 255]);
        }
    }
    d.queue.write_texture(
        scene.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE * 4),
            rows_per_image: Some(SIZE),
        },
        scene.size(),
    );
    let depth = d.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: scene.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut values = Vec::new();
    for _ in 0..SIZE {
        for x in 0..SIZE {
            values.push(z_of(if x < SIZE / 2 { left } else { right }));
        }
    }
    d.queue.write_texture(
        depth.as_image_copy(),
        bytemuck::cast_slice(&values),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE * 4),
            rows_per_image: Some(SIZE),
        },
        depth.size(),
    );
    Fixture {
        view: scene.create_view(&wgpu::TextureViewDescriptor::default()),
        scene,
        depth: depth.create_view(&wgpu::TextureViewDescriptor::default()),
    }
}

fn run(d: &Device, fx: &mut PhotoFx, f: &Fixture, params: Params) -> (bool, Vec<u8>) {
    let mut encoder = d.device.create_command_encoder(&Default::default());
    let drawn = fx.render(
        &d.device,
        &d.queue,
        &mut encoder,
        &Frame {
            scene: &f.view,
            depth: &f.depth,
            sample_count: 1,
            size: (SIZE, SIZE),
            viewport: (0.0, 0.0, SIZE as f32, SIZE as f32),
            params,
        },
    );
    let padded = (SIZE * 4) as usize;
    let readback = d.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * SIZE as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        f.scene.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(SIZE),
            },
        },
        f.scene.size(),
    );
    d.queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    d.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");
    let bytes = readback
        .slice(..)
        .get_mapped_range()
        .expect("mapping")
        .to_vec();
    (drawn, bytes)
}

fn red(bytes: &[u8], x: u32, y: u32) -> u8 {
    bytes[((y * SIZE + x) * 4) as usize]
}

fn params(dof: Option<Dof>, grade: Option<Grade>) -> Params {
    Params {
        grade,
        dof,
        near: NEAR,
        far: FAR,
    }
}

#[test]
fn with_nothing_to_apply_the_pass_encodes_nothing_and_the_frame_is_untouched() {
    let Some(d) = device() else { return };
    let f = fixture(&d, 5.0, 400.0);
    let mut fx = PhotoFx::new(&d.device, wgpu::TextureFormat::Rgba8Unorm).expect("pipelines");
    let (before_drawn, before) = run(&d, &mut fx, &f, params(None, None));
    assert!(!before_drawn);
    let neutral = Grade {
        exposure: 0.0,
        saturation: 1.0,
    };
    let (drawn, after) = run(&d, &mut fx, &f, params(None, Some(neutral)));
    assert!(!drawn, "a neutral grade encodes nothing");
    assert_eq!(before, after);
}

/// Focus at `d = 3` (sharp from 1.2 to 8.5): the left half, 5 units away, is in focus and the
/// right half, 400 units away, is far beyond the far edge.
#[test]
fn depth_of_field_blurs_the_far_field_and_leaves_the_near_craft_alone() {
    let Some(d) = device() else { return };
    let f = fixture(&d, 5.0, 400.0);
    let mut fx = PhotoFx::new(&d.device, wgpu::TextureFormat::Rgba8Unorm).expect("pipelines");
    let dof = Dof {
        focus: 0.0,
        aperture: 1.0,
    };
    let (_, bytes) = run(&d, &mut fx, &f, params(Some(dof), None));
    for y in [4, 32, 59] {
        for x in 0..(SIZE / 2 - 6) {
            assert_eq!(
                red(&bytes, x, y),
                if x % 2 == 0 { 255 } else { 0 },
                "the sharp half changed at ({x}, {y})"
            );
        }
        // The pixels at the seam, on the sharp side, must not take the blurred half's colour:
        // a halo of far blur over the near edge is what reads as wrong.
        for x in (SIZE / 2 - 6)..(SIZE / 2) {
            assert_eq!(
                red(&bytes, x, y),
                if x % 2 == 0 { 255 } else { 0 },
                "a halo on the near side at ({x}, {y})"
            );
        }
        let far: Vec<u8> = ((SIZE / 2 + 8)..SIZE).map(|x| red(&bytes, x, y)).collect();
        let spread = far.iter().max().unwrap() - far.iter().min().unwrap();
        assert!(
            spread < 120,
            "the far stripes are still sharp at row {y}: {far:?}"
        );
    }
}

#[test]
fn more_aperture_blurs_more() {
    let Some(d) = device() else { return };
    let mut fx = PhotoFx::new(&d.device, wgpu::TextureFormat::Rgba8Unorm).expect("pipelines");
    let spread = |aperture: f32, fx: &mut PhotoFx| {
        let f = fixture(&d, 5.0, 400.0);
        let dof = Dof {
            focus: 0.0,
            aperture,
        };
        let (_, bytes) = run(&d, fx, &f, params(Some(dof), None));
        let far: Vec<u8> = ((SIZE / 2 + 8)..SIZE).map(|x| red(&bytes, x, 32)).collect();
        far.iter().max().unwrap() - far.iter().min().unwrap()
    };
    let small = spread(0.1, &mut fx);
    let large = spread(1.0, &mut fx);
    assert!(
        large < small,
        "aperture 1.0 spread {large}, 0.1 spread {small}"
    );
}

fn mean(bytes: &[u8]) -> f32 {
    let sum: u32 = bytes.chunks(4).map(|p| u32::from(p[0])).sum();
    sum as f32 / (SIZE * SIZE) as f32
}

#[test]
fn exposure_up_raises_the_mean_and_down_lowers_it() {
    let Some(d) = device() else { return };
    let mut fx = PhotoFx::new(&d.device, wgpu::TextureFormat::Rgba8Unorm).expect("pipelines");
    let grade = |exposure| {
        Some(Grade {
            exposure,
            saturation: 1.0,
        })
    };
    // A mid grey, so neither direction clips.
    let at = |exposure: f32, fx: &mut PhotoFx| {
        let f = fixture(&d, 5.0, 400.0);
        d.queue.write_texture(
            f.scene.as_image_copy(),
            &vec![128; (SIZE * SIZE * 4) as usize],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE * 4),
                rows_per_image: Some(SIZE),
            },
            f.scene.size(),
        );
        mean(&run(&d, fx, &f, params(None, grade(exposure))).1)
    };
    let (dark, mid, bright) = (at(-1.0, &mut fx), at(0.5, &mut fx), at(1.0, &mut fx));
    assert!(
        dark < 128.0 && 128.0 < mid && mid < bright,
        "{dark} {mid} {bright}"
    );
}

#[test]
fn zero_saturation_is_grey_and_more_saturation_is_further_from_it() {
    let Some(d) = device() else { return };
    let mut fx = PhotoFx::new(&d.device, wgpu::TextureFormat::Rgba8Unorm).expect("pipelines");
    let colour = |saturation: f32, fx: &mut PhotoFx| {
        let f = fixture(&d, 5.0, 400.0);
        let pixels: Vec<u8> = (0..SIZE * SIZE).flat_map(|_| [180, 90, 60, 255]).collect();
        d.queue.write_texture(
            f.scene.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE * 4),
                rows_per_image: Some(SIZE),
            },
            f.scene.size(),
        );
        let grade = Grade {
            exposure: 0.0,
            saturation,
        };
        let (_, bytes) = run(&d, fx, &f, params(None, Some(grade)));
        [bytes[0], bytes[1], bytes[2]]
    };
    let grey = colour(0.0, &mut fx);
    assert!(
        grey[0].abs_diff(grey[1]) <= 1 && grey[1].abs_diff(grey[2]) <= 1,
        "{grey:?}"
    );
    let vivid = colour(2.0, &mut fx);
    assert!(
        i32::from(vivid[0]) - i32::from(vivid[2]) > 180 - 60,
        "saturation 2 widens the red-blue gap: {vivid:?}"
    );
}
