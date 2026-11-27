//! The scene blocks every drawable of a kind reads: one write per kind per view, in a region
//! of the buffer that belongs to that view alone.

use super::scene_build::{bare_scene, bare_scene_of, livery, test_gpu, triangle};
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
            texture_binding_view_dimension: None,
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
        crate::scene::WindowEffects::default(),
        &mut [],
    );
    let eye_of = |slot| {
        race.view.set_viewpoint(&race.core, slot);
        let eye = race.view.camera_position(&race.core).to_array();
        race.view.reset_viewpoint();
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
        let expected = race
            .view
            .ship_model_matrix_of(&race.core, 0)
            .to_cols_array_2d();
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

/// A view's blocks go out as one write, and each reads back as the scene it was given.
/// Write them with one `SceneBlocks::write` call each instead and the log holds five writes; leave
/// a kind unused and its block is not written (two runs around it).
#[test]
fn a_views_blocks_are_one_write() {
    use crate::drawable::{SceneBlocks, SceneKind};
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let scene_with = |seconds: f32| mesh_render::Scene {
        time: [seconds; 4],
        ..mesh_render::Scene::off()
    };
    let scenes = [
        scene_with(1.0),
        scene_with(2.0),
        scene_with(3.0),
        scene_with(4.0),
        scene_with(5.0),
    ];
    let block = SceneBlocks::view(3);
    let write = |used: &[SceneKind]| {
        let mut blocks = SceneBlocks::new(&device, &queue);
        for &kind in used {
            blocks.mark_used(kind);
        }
        let log = std::cell::RefCell::new(oag_gpu::writes::Log::default());
        blocks.write_view(
            &log,
            block,
            [&scenes[0], &scenes[1], &scenes[2], &scenes[3], &scenes[4]],
        );
        let writes = log.borrow().len();
        log.borrow_mut().replay(&queue);
        (blocks, writes)
    };
    let (blocks, writes) = write(&SceneBlocks::VIEW_KINDS);
    assert_eq!(writes, 1, "five adjacent blocks, one write");
    for (kind, scene) in SceneBlocks::VIEW_KINDS.into_iter().zip(&scenes) {
        let read = blocks.read(&device, &queue, 3, kind);
        assert_eq!(read.time, scene.time, "{kind:?}");
        assert_eq!(
            bytemuck::bytes_of(&read),
            bytemuck::bytes_of(scene),
            "{kind:?}"
        );
    }
    let (blocks, writes) = write(&[SceneKind::Circuit, SceneKind::Shine, SceneKind::Flame]);
    assert_eq!(writes, 2, "a run either side of the unused kind");
    assert_eq!(
        blocks.read(&device, &queue, 3, SceneKind::Ship).time,
        mesh_render::Scene::off().time,
        "the unused kind's block keeps Scene::off"
    );
    assert_eq!(
        blocks.read(&device, &queue, 3, SceneKind::Shine).time,
        [3.0; 4]
    );
}

/// HD's absorb shell reads the craft's scene block, where the program's closing `/2`
/// (`Light::output_scale`), the fog and the texture's scroll clock live: left on the
/// drawable's own block (`Scene::off`) the shell draws at twice the original's strength,
/// unfogged, on a frozen texture. Drop the shell's loop in `share_scene_blocks` and it
/// keeps no kind.
#[test]
fn the_absorb_shell_reads_the_craft_scene_block() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let mut scene = bare_scene(&device, &queue);
    scene.give_a_shell_for_tests(&device);
    assert_eq!(
        scene.shell_scene_kind_for_tests(),
        None,
        "a fresh copy has no kind"
    );
    scene.share_scene_blocks_for_tests(&device);
    assert_eq!(scene.shell_scene_kind_for_tests(), Some(SceneKind::Ship));
}

/// The shield shell's and cockpit sphere's shaders scroll on `scene.time`
/// (`PROGRAM_SHIELD_LIGHTNING`, `own_shield.wesl`): left on their own block (`Scene::off`) the
/// clock stands at zero and the lattice never moves. Drop the shield loop in
/// `share_scene_blocks` and they keep no kind.
#[test]
fn the_shield_shell_and_sphere_read_the_clocked_block() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let mut scene = bare_scene(&device, &queue);
    scene.give_a_shield_for_tests(&device);
    assert_eq!(
        scene.shield_scene_kinds_for_tests(),
        [None; 2],
        "fresh copies"
    );
    scene.share_scene_blocks_for_tests(&device);
    assert_eq!(
        scene.shield_scene_kinds_for_tests(),
        [Some(SceneKind::Flame); 2]
    );
}

/// The engine flare's rim (`flame_shading`) and our own shield's rim, facets and silhouette
/// are measured from the eye (`to_eye`), so the flame surfaces' block holds the eye of the view
/// being drawn, with the clock and no fog. Drop the camera from `flame_scene` and the eye read
/// back is the origin; move the flare or the shields off the block and their kind is not it.
#[test]
fn the_flame_surfaces_and_shields_read_each_views_eye() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let mut craft = livery();
    craft.flare = Some(triangle());
    let mut scene = bare_scene_of(&device, &queue, &[craft]);
    scene.give_a_shield_for_tests(&device);
    scene.share_scene_blocks_for_tests(&device);
    assert_eq!(scene.flare_scene_kind_for_tests(), Some(SceneKind::Flame));
    assert_eq!(
        scene.shield_scene_kinds_for_tests(),
        [Some(SceneKind::Flame); 2]
    );
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
        crate::scene::WindowEffects::default(),
        &mut [],
    );
    for slot in 0..2 {
        race.view.set_viewpoint(&race.core, slot);
        let eye = race.view.camera_position(&race.core).to_array();
        race.view.reset_viewpoint();
        assert_ne!(eye, [0.0; 3], "view {slot}'s camera is not at the origin");
        let block = scene.scene_block(&device, &queue, slot, SceneKind::Flame);
        assert_eq!(block.fog.camera, eye, "view {slot}'s flame surfaces");
        let circuit = scene.scene_block(&device, &queue, slot, SceneKind::Circuit);
        assert_eq!(block.time, circuit.time, "their clock");
        assert_eq!(block.fog.enabled, 0.0, "they are not fogged");
    }
}

/// HD's flame program closes its colour on `MAD /2`, which is the circuit's
/// `Light::output_scale`, so the flame surfaces' block carries it. Drop it from `flame_scene`
/// and the block reads `Scene::off`'s 1.0 where HD's circuit reads 0.5.
#[test]
fn the_flame_surfaces_close_on_the_circuits_output_scale() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let mut craft = livery();
    craft.flare = Some(triangle());
    let mut scene = bare_scene_of(&device, &queue, &[craft]);
    scene.share_scene_blocks_for_tests(&device);
    scene.set_output_scale_for_tests(0.5);
    assert_eq!(scene.flare_scene_kind_for_tests(), Some(SceneKind::Flame));
    let view = target(&device);
    let race = race_with_a_grid();
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
        crate::scene::WindowEffects::default(),
        &mut [],
    );
    let flame = scene.scene_block(&device, &queue, 0, SceneKind::Flame);
    let circuit = scene.scene_block(&device, &queue, 0, SceneKind::Circuit);
    assert_eq!(circuit.light.output_scale, 0.5, "HD's output scale");
    assert_eq!(
        flame.light.output_scale, circuit.light.output_scale,
        "the flame program's closing `/2`"
    );
}

/// A capture's imposed flame clock (`Options::flame_seconds`, the original's frame context clock at
/// the grab) reaches the flame surfaces' block and nothing else: the circuit keeps the world
/// clock. Read the world clock in `flame_scene`'s caller again and the flame block's time is the
/// circuit's; impose it on the circuit too and the circuit's is the imposed one.
#[test]
fn an_imposed_flame_clock_reaches_the_flame_surfaces_alone() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let mut craft = livery();
    craft.flare = Some(triangle());
    let mut scene = bare_scene_of(&device, &queue, &[craft]);
    scene.share_scene_blocks_for_tests(&device);
    let view = target(&device);
    let mut race = race_with_a_grid();
    let imposed = 122.472_f32;
    race.view.flame_seconds = Some(imposed);
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
        crate::scene::WindowEffects::default(),
        &mut [],
    );
    let flame = scene.scene_block(&device, &queue, 0, SceneKind::Flame);
    let circuit = scene.scene_block(&device, &queue, 0, SceneKind::Circuit);
    assert_eq!(
        flame.time, [imposed; 4],
        "the flame rides the imposed clock"
    );
    assert_eq!(
        circuit.time,
        [race.world_clock_seconds(); 4],
        "the circuit keeps the world clock"
    );
    assert_ne!(flame.time, circuit.time);
}
