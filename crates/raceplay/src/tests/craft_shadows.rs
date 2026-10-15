//! HD's `original` tier gives every craft a shadow map of its own at the side its
//! distance rank earns, which the road then reads through that craft's record.

use super::scene_build::{bare_scene_of, livery, test_gpu};
use super::*;
use oag_display::display::{Fov, Shadows};
use oag_render::shadow::map::{ATLAS_HEIGHT, ATLAS_WIDTH, RANK_SIDES};

fn target(device: &wgpu::Device) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("craft shadows test target"),
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

/// A craft's atlas side in texels, off its record's rectangle.
fn side_of(record: &oag_mesh::mesh_render::CraftShadow) -> u32 {
    (record.rect[2] * ATLAS_WIDTH as f32).round() as u32
}

/// Every craft on the grid draws a map, the squares are all different, and
/// their sides are the original's by distance from the eye: the craft nearest
/// the camera takes a 512 square and the farthest the smallest side the field
/// reaches. Fails if the scene stops handing the receiver the records (every
/// rect reads empty), or ranks by anything but the distance.
#[test]
fn every_craft_takes_the_square_its_distance_rank_earns() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let liveries: Vec<_> = (0..4).map(|_| livery()).collect();
    let scene = bare_scene_of(&device, &queue, &liveries);
    let view = target(&device);
    let race = race_with_a_grid();
    let craft = usize::from(race.ship_count());
    assert!(craft >= 4, "the grid fields several craft: {craft}");
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
        WindowEffects::default(),
        &mut [],
    );
    let records = scene.craft_shadows();
    let eye = race.camera_position();
    let distance = |slot: usize| {
        (race
            .ship_model_matrix_of(slot)
            .transform_point3(oag_core::math::Vec3::ZERO)
            - eye)
            .length_squared()
    };
    let mut by_distance: Vec<usize> = (0..craft).collect();
    by_distance.sort_by(|a, b| distance(*a).total_cmp(&distance(*b)).then(a.cmp(b)));
    for (rank, slot) in by_distance.iter().enumerate() {
        assert_eq!(
            side_of(&records[*slot]),
            RANK_SIDES[rank],
            "slot {slot} is rank {rank} by distance from the eye"
        );
    }
    let squares: std::collections::BTreeSet<_> = records[..craft]
        .iter()
        .map(|record| {
            (
                (record.rect[0] * ATLAS_WIDTH as f32).round() as u32,
                (record.rect[1] * ATLAS_HEIGHT as f32).round() as u32,
            )
        })
        .collect();
    assert_eq!(squares.len(), craft, "no two craft share a square");
}
