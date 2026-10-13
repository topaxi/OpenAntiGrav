//! A split screen encodes a frame's shadow maps once, however many views it
//! draws: they are a function of the craft, not of the camera, so every later
//! view samples what the first one drew.

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

/// Four views encode the shadow maps for the frame once, and the next frame
/// once more. Drop the `Layer::Next` gate in `render_shadow_passes` and the
/// first count reads four.
#[test]
fn four_views_encode_the_shadow_maps_once() {
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
        assert_eq!(
            scene.shadow_frames(),
            frame,
            "frame {frame}: one shadow encode per frame, not one per view"
        );
    }
}
