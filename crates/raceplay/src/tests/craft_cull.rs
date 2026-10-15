//! A hull the camera cannot see is left out of the frame's draws, and one it
//! can see is not: the wiring of `scene::craft_cull` into `Scene::render`.

use super::scene_build::{bare_scene_of, livery, test_gpu};
use super::*;
use oag_display::display::{Fov, MotionBlur, Shadows};

fn target(device: &wgpu::Device) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("craft cull test target"),
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

/// The triangles one frame submits with `cull` as given and craft 1 at `offset` from craft 0.
fn triangles(cull: bool, offset: Vec3) -> u32 {
    let Some((device, queue)) = test_gpu() else {
        return u32::MAX;
    };
    let mut race = race_with_a_grid();
    let liveries: Vec<_> = (0..race.ship_count()).map(|_| livery()).collect();
    let scene = bare_scene_of(&device, &queue, &liveries);
    let view = target(&device);
    race.tick(&oag_gameplay::PlayerInputs::none());
    let origin = race.sim.world.ships[0].physics.body.position;
    race.sim.world.ships[1].physics.body.position = origin + offset;
    let mut encoder = device.create_command_encoder(&Default::default());
    let stats = scene.render(
        &device,
        &queue,
        &mut encoder,
        &view,
        &race,
        (0.0, 0.0, 64.0, 64.0),
        Fov::AUTHORED,
        cull,
        true,
        None,
        MotionBlur::Off,
        Shadows::Off,
        None,
        &[],
        None,
        None,
        None,
    );
    queue.submit([encoder.finish()]);
    stats.triangles
}

/// The track is one triangle too, so what differs between the frames is the
/// second craft's hull: one triangle when it is drawn, none when it is culled.
/// Drop the test in `Scene::render` and the far-behind craft is drawn again.
#[test]
fn a_hull_out_of_view_is_not_drawn_and_one_in_view_is() {
    let beside = triangles(true, Vec3::Y * 5.0);
    let behind = triangles(true, -Vec3::X * 4000.0);
    if beside == u32::MAX {
        return;
    }
    assert_eq!(
        beside,
        behind + 1,
        "the craft in view adds its one triangle ({beside}), the one behind adds none ({behind})"
    );
}

/// With frustum culling off the setting is honoured: every hull is drawn.
#[test]
fn with_culling_off_a_hull_behind_the_camera_is_still_drawn() {
    let off = triangles(false, -Vec3::X * 4000.0);
    if off == u32::MAX {
        return;
    }
    assert_eq!(off, triangles(false, Vec3::Y * 5.0));
    assert!(off > triangles(true, -Vec3::X * 4000.0));
}
