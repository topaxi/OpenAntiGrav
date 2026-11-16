//! Draws one absorb-shell triangle through the mesh pipeline on a fixture needing no game
//! content, and pins the program's closing `/2`.
//!
//! `hd_absorbinternal`'s fogged blocks #2 to #9 all end on `MAD /2 H0.xyz`
//! (`docs/ghidra/functions/ps3-hdfury-eu/absorb-feedback.md`, "The fogged blocks close on
//! `/2`"), so the shell's colour is the texture tap times `Light::output_scale`: `0.5` on HD,
//! `1.0` where a title has no such scale. Skips when there is no adapter. Runs in `just test`.

use oag_gpu::formats::SceneStore;
use oag_mesh::mesh_render::Scene;

use crate::hd_flame_encoded_tap::model;
use crate::hd_holo_billboard_lit_path::draw_into;

/// A grey `128` texel through the shell program reads `0.502` at scale 1 and `0.251` at scale
/// 0.5. Drop the multiply in `shade.wesl`'s absorb branch and both read `0.502`.
#[test]
fn the_absorb_shell_is_halved_by_the_output_scale() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let mut shell = model(128);
    shell.flame = None;
    shell.absorb_shell = true;
    for vertex in &mut shell.vertices {
        vertex.colour = [1.0; 4];
    }
    let at = |output_scale: f32| {
        let mut scene = Scene::off();
        scene.fog.camera = [0.0, 0.0, 100.5];
        scene.light.output_scale = output_scale;
        draw_into(
            &device,
            &queue,
            &shell,
            scene,
            wgpu::TextureFormat::Rgba8Unorm,
            SceneStore::Computed,
        )[0]
    };
    let (full, half) = (at(1.0), at(0.5));
    assert!(full > 0.2, "the shell drew nothing at scale 1: {full}");
    assert!(
        (half / full - 0.5).abs() < 0.03,
        "scale 0.5 gave {half} against {full} at scale 1"
    );
}
