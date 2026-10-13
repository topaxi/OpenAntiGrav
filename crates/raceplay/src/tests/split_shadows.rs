//! A split screen does a frame's camera-independent work once, however many
//! views it draws: the shadow maps, the shadow gather, the scenery clocks and the
//! advert cards are functions of the craft and the race clock, so every later view
//! reads what the first one left.

use super::scene_build::{bare_scene, test_gpu};
use super::*;
use oag_display::display::{Fov, Shadows};

fn target(device: &wgpu::Device) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("split shadows test target"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

/// Four views do the frame's shared work once, and the next frame once more.
/// The shadow gather is per tick, so the race ticks between the two frames.
/// Drop any `Layer::Next` gate (`render_shadow_passes`, `upload_shadow_geometry`,
/// the scenery clocks and the advert cards in `Scene::render`) and its count reads
/// four for the first frame.
#[test]
fn four_views_do_the_shared_work_once() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let scene = bare_scene(&device, &queue);
    let view = target(&device);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(4), 4, "the grid fields four craft");
    for frame in 1..=2 {
        if frame > 1 {
            race.tick(&oag_gameplay::PlayerInputs::none());
        }
        scene.render_viewports(
            &device,
            &queue,
            &view,
            &race,
            (0.0, 0.0, 64.0, 64.0),
            Fov::AUTHORED,
            true,
            true,
            None,
            Shadows::Original,
            &[],
            &mut [],
        );
        let work = scene.frame_work();
        for (what, count) in [
            ("shadow maps", work.shadow_maps),
            ("shadow gather", work.shadow_gathers),
            ("scenery clocks", work.scenery_clocks),
            ("advert cards", work.advert_cards),
        ] {
            assert_eq!(
                count, frame,
                "frame {frame}: {what} once, not once per view"
            );
        }
    }
}

/// Draws one frame of `race` at `alpha` into `view`, four views to a frame.
fn frame(
    gpu: &(wgpu::Device, wgpu::Queue),
    scene: &Scene,
    view: &wgpu::TextureView,
    race: &mut Race,
    alpha: f32,
) {
    frame_with(gpu, scene, view, race, alpha, Shadows::Original);
}

fn frame_with(
    gpu: &(wgpu::Device, wgpu::Queue),
    scene: &Scene,
    view: &wgpu::TextureView,
    race: &mut Race,
    alpha: f32,
    shadows: Shadows,
) {
    race.set_draw_alpha(alpha);
    scene.render_viewports(
        &gpu.0,
        &gpu.1,
        view,
        race,
        (0.0, 0.0, 64.0, 64.0),
        Fov::AUTHORED,
        true,
        true,
        None,
        shadows,
        &[],
        &mut [],
    );
    race.set_draw_alpha(1.0);
}

/// One gather of the `original` tier per **tick**, however many frames draw
/// inside it and at whatever alpha: a 240 Hz window draws four frames a tick.
/// Fails if the cache is keyed on the frame, or does not see a new tick.
#[test]
fn the_original_tier_gathers_once_per_tick_not_once_per_frame() {
    let Some(gpu) = test_gpu() else {
        return;
    };
    let scene = bare_scene(&gpu.0, &gpu.1);
    let view = target(&gpu.0);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2);
    let none = oag_gameplay::PlayerInputs::none();

    // The first frame has no earlier tick to blend from, so it also gathers the
    // one it draws (the live gather); every tick after is one gather.
    frame(&gpu, &scene, &view, &mut race, 1.0);
    frame(&gpu, &scene, &view, &mut race, 1.0);
    assert_eq!(scene.frame_work().shadow_gathers, 1, "two frames, one tick");
    for tick in 1..=10 {
        race.tick(&none);
        for alpha in [0.1, 0.4, 0.7, 1.0] {
            frame(&gpu, &scene, &view, &mut race, alpha);
        }
        assert_eq!(
            scene.frame_work().shadow_gathers,
            1 + tick,
            "tick {tick}: four frames, one gather"
        );
    }
}

/// Under 60 Hz the race runs several ticks between draws, so no draw has a
/// previous tick's gather to blend from and the cache would only double the
/// work: after a couple of misses it stops seeding, and a frame is one gather.
#[test]
fn a_frame_rate_under_sixty_gathers_once_per_frame() {
    let Some(gpu) = test_gpu() else {
        return;
    };
    let scene = bare_scene(&gpu.0, &gpu.1);
    let view = target(&gpu.0);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2);
    let none = oag_gameplay::PlayerInputs::none();
    let mut frames = |count: u32| {
        for _ in 0..count {
            race.tick(&none);
            race.tick(&none);
            frame(&gpu, &scene, &view, &mut race, 0.5);
        }
    };
    frames(3);
    let warm = scene.frame_work().shadow_gathers;
    frames(10);
    assert_eq!(
        scene.frame_work().shadow_gathers,
        warm + 10,
        "ten frames, ten gathers: the seeding gather is not paid every frame"
    );
}

/// **The `mapped` tier's map is fitted on the craft each view follows**, not on
/// slot 0 for every view. The last view drawn leaves its fit in the map, so with
/// two views that is slot 1's. Fails if a later view stops redrawing the map
/// (it keeps view 0's fit) or fits it on the wrong craft.
#[test]
fn the_mapped_map_follows_the_craft_of_each_view() {
    let Some(gpu) = test_gpu() else {
        return;
    };
    let scene = bare_scene(&gpu.0, &gpu.1);
    let view = target(&gpu.0);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2);
    let (first, second) = (
        scene.mapped_fit_for(&race, 0).matrix(),
        scene.mapped_fit_for(&race, 1).matrix(),
    );
    assert_ne!(
        first, second,
        "the grid puts the two craft in different places"
    );

    frame_with(&gpu, &scene, &view, &mut race, 1.0, Shadows::Mapped);
    assert_eq!(
        scene.mapped_matrix(),
        second,
        "the last view's craft is slot 1"
    );
}
