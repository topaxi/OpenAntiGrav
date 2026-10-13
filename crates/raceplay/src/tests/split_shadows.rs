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
