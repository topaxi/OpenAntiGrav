//! The scene blocks every drawable of a kind reads: one write per kind per view, in a region
//! of the buffer that belongs to that view alone.

use super::scene_build::{bare_scene, test_gpu};
use super::*;
use crate::drawable::SceneKind;
use oag_display::display::{Fov, Shadows};

fn target(device: &wgpu::Device) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("scene blocks test target"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

/// A two-view frame leaves each view's circuit and craft blocks holding that view's camera:
/// the scene block carries the eye (`Fog::camera`), so two views that shared a block would
/// leave one eye in both. Drop `share_scene_blocks` (no kind is used, no block written) or
/// `write_scene_blocks` and both blocks stay `Scene::off`, whose eye is the origin.
#[test]
fn two_views_leave_a_block_each_holding_their_own_eye() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let scene = bare_scene(&device, &queue);
    let view = target(&device);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2, "the grid fields two craft");
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
        Shadows::Off,
        &[],
        &mut [],
    );
    let eye_of = |slot| {
        race.set_viewpoint(slot);
        let eye = race.camera_position().to_array();
        race.reset_viewpoint();
        eye
    };
    let (eye0, eye1) = (eye_of(0), eye_of(1));
    assert_ne!(eye0, eye1, "the two seats follow different craft");
    for kind in [SceneKind::Circuit, SceneKind::Ship] {
        let first = scene.scene_block(&device, &queue, 0, kind);
        let second = scene.scene_block(&device, &queue, 1, kind);
        assert_eq!(first.fog.camera, eye0, "{kind:?} block of view 0");
        assert_eq!(second.fog.camera, eye1, "{kind:?} block of view 1");
    }
    // The craft's matrices: one staged block a view, written in the view's own region.
    // Drop `share_uniform_blocks` and the slot is `None`; drop the flush and both read zero.
    let hull_in_view = |view: usize, ship: usize| {
        scene
            .ship_matrices(&device, &queue, view, ship)
            .expect("the hulls are on the uniform blocks")
    };
    for view in 0..2 {
        let (view_projection, model) = hull_in_view(view, 0);
        let expected = race.ship_model_matrix_of(0).to_cols_array_2d();
        assert_eq!(model, expected, "craft 0's model matrix in view {view}");
        assert_ne!(view_projection, [[0.0; 4]; 4], "view {view}'s camera");
    }
    assert_ne!(
        hull_in_view(0, 0).0,
        hull_in_view(1, 0).0,
        "two views, two cameras, two regions"
    );
    let untouched = scene.scene_block(&device, &queue, 2, SceneKind::Circuit);
    assert_eq!(
        untouched.fog.camera,
        mesh_render::Scene::off().fog.camera,
        "a view nobody drew still holds Scene::off"
    );
}

/// The stride is a multiple of the largest alignment a device can ask for, and a block never
/// overlaps the next.
#[test]
fn blocks_are_aligned_and_do_not_overlap() {
    assert_eq!(mesh_render::SCENE_STRIDE % 256, 0);
    const { assert!(mesh_render::SCENE_STRIDE >= mesh_render::SCENE_SIZE) };
    assert_eq!(
        crate::drawable::SceneBlocks::view(0),
        crate::drawable::ViewBlock::NONE
    );
    assert_ne!(
        crate::drawable::SceneBlocks::view(0),
        crate::drawable::SceneBlocks::view(1)
    );
}
