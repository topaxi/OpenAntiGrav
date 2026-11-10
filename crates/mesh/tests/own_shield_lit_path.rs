//! Draws one triangle through the mesh pipeline on a fixture needing no game content, and pins
//! that `Model::own_shield` reaches the pipeline: the model is shaded by `own_shield.wesl`, which
//! reads no texture and no lighting, where the same model without it is shaded by the ordinary
//! path. Skips when there is no adapter. Runs in `just test`.

use oag_gpu::formats::SceneStore;
use oag_mesh::mesh_render::Scene;

use crate::hd_flame_encoded_tap::model;
use crate::hd_holo_billboard_lit_path::draw_into;

#[test]
fn a_model_with_the_own_shield_flag_is_drawn_by_the_own_shield_shader() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let at = |grey: u8, own_shield: Option<f32>| {
        let mut shell = model(grey);
        shell.flame = None;
        shell.own_shield = own_shield;
        for vertex in &mut shell.vertices {
            vertex.colour = [1.0; 4];
        }
        let mut scene = Scene::off();
        scene.fog.camera = [0.0, 0.0, 100.5];
        draw_into(
            &device,
            &queue,
            &shell,
            scene,
            wgpu::TextureFormat::Rgba8Unorm,
            SceneStore::Computed,
        )
    };
    let plain = at(128, None);
    let own = at(128, Some(0.0));
    assert_ne!(plain, own, "the flag changes the shading");
    assert!(own[0] > 0.0, "the shell draws something: {own:?}");
    assert_eq!(own, at(250, Some(0.0)), "the own shield reads no texture");
}
