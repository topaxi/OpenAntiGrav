//! A split screen blurs per view: each view measures its velocity against its own camera,
//! the velocity and depth attachments keep every view's, and the blur runs over each view's
//! own rectangle ([`Scene::blur_split_views`]).

use super::scene_build::{livery, scene_over, test_gpu, triangle, vertex};
use super::*;
use oag_display::display::{Fov, MotionBlur, Shadows};

const SIZE: u32 = 64;
const AREA: (f32, f32, f32, f32) = (0.0, 0.0, SIZE as f32, SIZE as f32);

fn scene(device: &wgpu::Device, queue: &wgpu::Queue, race: &Race) -> Scene {
    // The craft is the one thing here that draws bright (the track draws black in this
    // fixture), so each hull is a few big triangles ahead of the craft along its model's
    // forward (-z), at several sizes: whichever the chase camera frames shows in the view.
    let hull = || {
        let mut livery = livery();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for (scale, ahead) in [(4.0f32, 3.0f32), (2.0, 6.0), (3.0, 5.0)] {
            let base = vertices.len() as u32;
            vertices.extend(
                [[-1.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
                    .map(|[x, y, z]| vertex([x * scale, y * scale, z * scale - ahead])),
            );
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 1]);
        }
        livery.hull.vertices = vertices;
        livery.hull.draws[0].range = 0..indices.len() as u32;
        livery.hull.indices = indices;
        livery.hull.draws[0].bounds.radius = 30.0;
        livery.hull.radius = 30.0;
        livery
    };
    let liveries: Vec<_> = (0..race.ship_count()).map(|_| hull()).collect();
    scene_over(device, queue, triangle(), &liveries)
}

fn target(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("split blur test target"),
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
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// The target's pixels, `SIZE * SIZE * 4` bytes.
fn pixels(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let row = (SIZE * 4) as usize;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("split blur readback"),
        size: (row * SIZE as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row as u32),
                rows_per_image: Some(SIZE),
            },
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the readback finishes");
    let view = buffer
        .slice(..)
        .get_mapped_range()
        .expect("the readback is mapped");
    view.to_vec()
}

/// The rows `rect` covers, as a slice of `pixels`' bytes.
fn rows<'a>(pixels: &'a [u8], rect: (f32, f32, f32, f32)) -> &'a [u8] {
    let row = (SIZE * 4) as usize;
    &pixels[rect.1 as usize * row..(rect.1 + rect.3) as usize * row]
}

/// Draws the split into `target` with `blur`.
fn draw(
    (device, queue): (&wgpu::Device, &wgpu::Queue),
    scene: &Scene,
    race: &Race,
    target: &wgpu::Texture,
    blur: MotionBlur,
) {
    scene.render_viewports(
        device,
        queue,
        &target.create_view(&Default::default()),
        race,
        AREA,
        Fov::AUTHORED,
        true,
        true,
        None,
        Shadows::Off,
        &[],
        WindowEffects {
            motion_blur: blur,
            ..Default::default()
        },
        &mut [],
    );
}

/// Whether anything was drawn in `rows` (a black view would make every comparison vacuous).
fn drawn(rows: &[u8]) -> bool {
    rows.iter().any(|&byte| byte != 0)
}

/// Two frames of a two-view race in which only craft `mover` travels, the blur off and on
/// drawn from the same state (a repeat of one tick measures against the same previous pose),
/// and the two views' rows of each picture. Returns `(off, on)` per view.
fn two_frames(mover: usize) -> Option<[[Vec<u8>; 2]; 2]> {
    let (device, queue) = test_gpu()?;
    let gpu = (&device, &queue);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2);
    let scene = scene(&device, &queue, &race);
    let (off, on) = (target(&device), target(&device));
    // Let the chase cameras settle on the grid: a camera still catching up is motion.
    for _ in 0..90 {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    for _ in 0..3 {
        let forward = race.sim.world.ships[mover].physics.body.forward();
        let ahead = race.sim.world.ships[mover].physics.body.position + forward * 6.0;
        race.sim.world.ships[mover].physics.body.position = ahead;
        race.tick(&oag_gameplay::PlayerInputs::none());
        draw(gpu, &scene, &race, &off, MotionBlur::Off);
        draw(gpu, &scene, &race, &on, MotionBlur::High);
    }
    let rects = race.view_rects(AREA);
    let (off, on) = (pixels(&device, &queue, &off), pixels(&device, &queue, &on));
    let of = |picture: &[u8], view: usize| rows(picture, rects[view]).to_vec();
    Some([[of(&off, 0), of(&on, 0)], [of(&off, 1), of(&on, 1)]])
}

/// The first view's craft travels and the second sits still: the first view blurs, the second
/// is byte for byte what it was with the blur off. Fails when a later view clears the first
/// view's velocity and depth (the first view then reads no motion), when the chain runs over
/// the whole picture (the second view is touched) and when no chain runs at all.
#[test]
fn the_view_that_moves_blurs_and_the_one_that_sits_still_does_not() {
    let Some([first, second]) = two_frames(0) else {
        return;
    };
    assert!(
        drawn(&first[0]) && drawn(&second[0]),
        "both views show a craft"
    );
    assert!(first[0] != first[1], "the moving view is blurred");
    assert!(second[0] == second[1], "the view at rest is not touched");
}

/// The reverse, with the moving view drawn last: the blur is not just the first (or the last)
/// view's. Fails when only the first view's chain, or only the last view's rectangle, is run.
#[test]
fn it_is_the_view_that_moves_wherever_it_is_drawn() {
    let Some([first, second]) = two_frames(1) else {
        return;
    };
    assert!(
        drawn(&first[0]) && drawn(&second[0]),
        "both views show a craft"
    );
    assert!(first[0] == first[1], "the view at rest is not touched");
    assert!(second[0] != second[1], "the moving view is blurred");
}

/// Each seat's view keeps the history of its own camera, apart from every window's: a seat's
/// previous view-projection is its own last one, the two seats look through different
/// cameras, window 0's history is not written by a split, and no upscaler record is kept for
/// the split.
#[test]
fn every_view_keeps_the_history_of_its_own_camera() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let gpu = (&device, &queue);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2);
    let scene = scene(&device, &queue, &race);
    let target = target(&device);
    let mut last = [None; 2];
    for frame in 0..4 {
        for slot in 0..2 {
            let forward = race.sim.world.ships[slot].physics.body.forward();
            race.sim.world.ships[slot].physics.body.position += forward * (3.0 + slot as f32);
        }
        race.tick(&oag_gameplay::PlayerInputs::none());
        draw(gpu, &scene, &race, &target, MotionBlur::Medium);
        for (slot, last) in last.iter_mut().enumerate() {
            let (previous, current) = scene.split_motion_probe(slot).expect("a history");
            if let Some(before) = *last {
                assert_eq!(
                    previous, before,
                    "frame {frame}, seat {slot}: its previous camera is its own last one"
                );
            }
            *last = Some(current);
        }
        assert_ne!(
            last[0], last[1],
            "the two seats look through different cameras"
        );
        assert!(
            scene.motion_probe(0).is_none(),
            "the split does not write window 0's history"
        );
        assert!(scene.temporal(0).is_none(), "a split feeds no upscaler");
    }
}
