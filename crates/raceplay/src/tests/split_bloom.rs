//! A split screen glows once per view: Pulse's bloom is prepared for every view's
//! own rectangle after the last view is drawn, and added back over each view after
//! the HUD ([`Scene::composite_bloom`]), so no view glows from another's pixels.

use super::scene_build::{glowing_scene, test_gpu};
use super::*;
use oag_display::display::{Fov, Shadows};

fn target(device: &wgpu::Device) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("split bloom test target"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
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
        .create_view(&Default::default())
}

fn draw(gpu: &(wgpu::Device, wgpu::Queue), scene: &Scene, view: &wgpu::TextureView, race: &Race) {
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
        Shadows::Original,
        &[],
        &mut [],
    );
}

fn composite(gpu: &(wgpu::Device, wgpu::Queue), scene: &Scene, view: &wgpu::TextureView) {
    let mut encoder = gpu.0.create_command_encoder(&Default::default());
    scene.composite_bloom(&mut encoder, view, (0.0, 0.0, 64.0, 64.0));
    gpu.1.submit(Some(encoder.finish()));
}

/// Each view of two, and of four, prepares its own glow and the HUD-time composite adds
/// each back exactly once. Drop `prepare_split_bloom` from `render_slots` and nothing is
/// prepared (the glow was skipped under a split before); drop the view loop in
/// `composite_bloom` and nothing is composited; and a second `composite_bloom` for the
/// same frame must add nothing.
#[test]
fn every_view_prepares_and_composites_its_own_glow() {
    let Some(gpu) = test_gpu() else {
        return;
    };
    let scene = glowing_scene(&gpu.0, &gpu.1);
    let view = target(&gpu.0);
    let mut race = race_with_a_grid();
    for views in [2usize, 4] {
        assert_eq!(
            race.set_viewport_count(views),
            views,
            "the grid fields them"
        );
        let before = scene.frame_work();
        draw(&gpu, &scene, &view, &race);
        let drawn = scene.frame_work();
        assert_eq!(
            drawn.bloom_views_prepared - before.bloom_views_prepared,
            views as u32,
            "{views} views, {views} glows prepared"
        );
        composite(&gpu, &scene, &view);
        let added = scene.frame_work();
        assert_eq!(
            added.bloom_views_composited - before.bloom_views_composited,
            views as u32,
            "{views} views, {views} glows added"
        );
        composite(&gpu, &scene, &view);
        assert_eq!(
            scene.frame_work().bloom_views_composited,
            added.bloom_views_composited,
            "a second composite adds nothing"
        );
    }
}

/// One view is not a split: `Scene::render` prepares its glow the way it always did
/// and no view lane is touched.
#[test]
fn one_view_prepares_no_view_lane() {
    let Some(gpu) = test_gpu() else {
        return;
    };
    let scene = glowing_scene(&gpu.0, &gpu.1);
    let view = target(&gpu.0);
    let race = race_with_a_grid();
    let mut encoder = gpu.0.create_command_encoder(&Default::default());
    scene.render(
        &gpu.0,
        &gpu.1,
        &mut encoder,
        &view,
        &race,
        (0.0, 0.0, 64.0, 64.0),
        Fov::AUTHORED,
        true,
        true,
        None,
        oag_display::display::MotionBlur::Off,
        Shadows::Original,
        None,
        &[],
        None,
        None,
        None,
    );
    gpu.1.submit(Some(encoder.finish()));
    composite(&gpu, &scene, &view);
    let work = scene.frame_work();
    assert_eq!(work.bloom_views_prepared, 0);
    assert_eq!(work.bloom_views_composited, 0);
}
