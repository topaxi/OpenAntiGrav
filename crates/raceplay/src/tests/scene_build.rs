//! What building a [`Scene`] costs on the GPU side, as opposed to what it draws.

use super::*;
use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex};

pub(super) fn vertex(position: [f32; 3]) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0; 4],
        texcoord: [0.0; 2],
        lightmap_texcoord: [0.0; 2],
        lit: 0.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: mesh::slots::DEFAULT,
        specular_exponent: mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
        tangent: 0,
        depth_bias: 0.0,
    }
}

/// One opaque triangle: enough geometry that every pool builds a drawable.
pub(super) fn triangle() -> Model {
    Model {
        vertices: vec![
            vertex([-1.0, 0.0, 0.0]),
            vertex([1.0, 0.0, 0.0]),
            vertex([0.0, 1.0, 0.0]),
        ],
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
            writes_depth: false,
            moving: false,
            blend: None,
            blend_state: None,
            depth_point: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            culled: false,
            range: 0..3,
            texture: None,
            bounds: Bounds {
                centre: [0.0; 3],
                radius: 1.5,
            },
            node: None,
            chunk: None,
            alpha_test_ref: None,
        }],
        ..model(0, 0)
    }
}

pub(super) fn livery() -> Livery {
    Livery {
        team: "Test".to_string(),
        hull: triangle(),
        nozzle: None,
        muzzle: None,
        boost: None,
        flare: None,
        flare_spikes: Vec::new(),
        flare_origins: oag_livery::FlareOrigins::default(),
        shield: None,
        shield_colour: None,
        collision_fx: Vec::new(),
        absorb: Vec::new(),
        absorb_overlay: None,
        leach_overlay: None,
        shine: None,
        wreck: None,
        absorb_shell: None,
        boost_uv: None,
        engine_light: None,
        cannon_flash: [None, None],
        arc_anchor: None,
        wingtip: [None, None],
    }
}

/// A device on whichever adapter this machine has, or `None` (said, not
/// failed) where there is none.
pub(super) fn test_gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    let Ok(pair) = pollster::block_on(adapter.request_device(&mesh_render::device_descriptor(
        "scene build test",
        &adapter,
    ))) else {
        eprintln!("no GPU device: skipping");
        return None;
    };
    Some(pair)
}

/// [`test_gpu`] with the device's `max_buffer_size` lowered to `ceiling` bytes: what a
/// device whose own maximum is that small (a phone, a Steam Deck, WebGPU) is to a model
/// whose vertex data is larger.
pub(super) fn test_gpu_with_buffer_ceiling(ceiling: u64) -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    let mut descriptor = mesh_render::device_descriptor("vertex chunk test", &adapter);
    descriptor.required_limits.max_buffer_size = ceiling;
    pollster::block_on(adapter.request_device(&descriptor)).ok()
}

/// A scene over one triangle for a circuit and for every pool: enough to be
/// drawn, nothing a picture could be judged by.
pub(super) fn bare_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Scene {
    bare_scene_of(device, queue, &[livery()])
}

/// [`bare_scene`] whose circuit stamps the glow mask, as Pulse PSP's does: the scene
/// then builds the recovered bloom.
pub(super) fn glowing_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Scene {
    let mut track = triangle();
    track.stamps_glow = true;
    scene_over(device, queue, track, &[livery()])
}

/// [`bare_scene`] with one hull per entry of `liveries`, slot by slot.
pub(super) fn bare_scene_of(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    liveries: &[Livery],
) -> Scene {
    scene_over(device, queue, triangle(), liveries)
}

/// The one constructor under the rest: a bare scene over `track`, with one hull per entry
/// of `liveries`.
pub(super) fn scene_over(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    track: Model,
    liveries: &[Livery],
) -> Scene {
    scene_over_msaa(
        device,
        queue,
        track,
        liveries,
        oag_display::display::Msaa::Off,
    )
}

/// [`scene_over`] at a multisample level.
pub(super) fn scene_over_msaa(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    track: Model,
    liveries: &[Livery],
    msaa: oag_display::display::Msaa,
) -> Scene {
    scene_over_post(device, queue, track, liveries, msaa, None, None, None)
}

/// [`scene_over_msaa`] with Wipeout HD's post chain, Omega's tone map or 2048's composite,
/// as a circuit that authors an `HDR and Bloom` block, a `Tonemap` block or the
/// `Lighting.Exposure*` keys builds it.
#[allow(clippy::too_many_arguments)]
pub(super) fn scene_over_post(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    track: Model,
    liveries: &[Livery],
    msaa: oag_display::display::Msaa,
    hd_bloom: Option<oag_post::hd_bloom::Params>,
    omega_tonemap: Option<oag_post::omega_tonemap::Params>,
    wo_composite: Option<oag_post::wo_composite::Params>,
) -> Scene {
    Scene::new(
        device,
        queue,
        track,
        liveries,
        None,
        None,
        None,
        None,
        Default::default(),
        true,
        Some(triangle()),
        Some(triangle()),
        Some(triangle()),
        Some(triangle()),
        Default::default(),
        Default::default(),
        None,
        None,
        None,
        None,
        crate::rocket_smoke::RibbonTextures::default(),
        None,
        None,
        None,
        Default::default(),
        None,
        wgpu::TextureFormat::Rgba8Unorm,
        (64, 64),
        Anisotropy::Off,
        None,
        msaa,
        Vec::new(),
        mesh_render::Light::stand_in(),
        None,
        None,
        hd_bloom,
        omega_tonemap,
        wo_composite,
        None,
        Vec::new(),
        Vec::new(),
        Default::default(),
    )
    .expect("the scene builds")
}

/// **Every drawable in a weapon pool shares one pipeline set.**
///
/// A pool is `MAX_PROJECTILES` drawables of one model, and each asks
/// `mesh_render::build` for the same pipelines. With the build cache open the
/// first compiles them and the other 127 reuse them; without it each compiles
/// its own, which measured about 7 MiB of resident memory per drawable - 8.4 GiB
/// for a Pulse race, on the `--screenshot` path that never opened the cache.
/// Fails if `Scene::new` stops opening it, whichever caller it comes from.
#[test]
fn a_weapon_pool_of_drawables_shares_one_set_of_gpu_resources() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let scene = bare_scene(&device, &queue);

    for (pool, (slots, shared)) in scene.weapon_pool_sharing().into_iter().enumerate() {
        assert_eq!(
            slots,
            oag_weapons::projectile::MAX_PROJECTILES,
            "pool {pool} lost a slot"
        );
        assert!(shared, "pool {pool} gave every slot its own geometry copy");
    }
    let (asked, reused, distinct) = scene.build_cache();
    assert!(asked > 0, "no pipeline request went through the cache");
    assert!(
        distinct < 100,
        "{distinct} distinct pipelines for identical models: the cache is not sharing ({reused} of {asked} reused)"
    );
}

/// A scene build ends at its first checkpoint once the process is exiting, so the race build
/// does not hold the exit for a scene nobody will see (`oag_thread::gpu_workers`). `scene_over`
/// expects the build to succeed, so the stop is its panic. The stop is process-wide and one-way:
/// nextest runs this in a process of its own.
#[test]
fn a_scene_build_ends_once_the_process_is_exiting() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let _worker = oag_thread::gpu_workers::Guard::enter("scene-build-test");
    assert!(!oag_thread::gpu_workers::finish_within(
        std::time::Duration::ZERO
    ));
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        bare_scene(&device, &queue);
    }))
    .expect_err("a scene build in a process that is exiting ends early");
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap_or_default();
    assert!(message.contains("the process is exiting"), "{message}");
}

/// **A circuit's advert cards are what its placeholder quads sample.**
///
/// Builds the real scene of each disc's default circuit and counts the track
/// materials `Scene::new` pointed at a card. Drop the rebinding and every
/// placeholder shows its 8x8 digit stub (or, once stripped, nothing), and this
/// reads zero.
fn adverts_rebound(source: &str) -> Option<(usize, usize, usize)> {
    let image = oag_testdata::image(source)?;
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    let (device, queue) = pollster::block_on(adapter.request_device(
        &mesh_render::device_descriptor("advert rebind test", &adapter),
    ))
    .ok()?;
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..crate::Options::default()
    })
    .expect("loading the race");
    let served: Vec<u32> = loaded.billboards.adverts.iter().map(|c| c.slot).collect();
    let wanted = oag_render::gantry::placeholder_texture_slots(&loaded.track_model)
        .iter()
        .filter(|(_, number)| served.contains(number))
        .count();
    let scene = Scene::new(
        &device,
        &queue,
        loaded.track_model,
        &[livery()],
        None,
        None,
        None,
        None,
        loaded.billboards,
        true,
        None,
        None,
        None,
        None,
        Default::default(),
        Default::default(),
        None,
        None,
        None,
        None,
        crate::rocket_smoke::RibbonTextures::default(),
        None,
        None,
        None,
        Default::default(),
        None,
        wgpu::TextureFormat::Rgba8Unorm,
        (64, 64),
        Anisotropy::Off,
        None,
        oag_display::display::Msaa::Off,
        Vec::new(),
        mesh_render::Light::stand_in(),
        None,
        None,
        None,
        None,
        None,
        None,
        Vec::new(),
        Vec::new(),
        Default::default(),
    )
    .expect("the scene builds");
    Some((wanted, scene.adverts_rebound(), scene.adverts_wrapped()))
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU"]
fn pulse_psp_points_every_served_placeholder_at_its_card() {
    let Some((wanted, rebound, wrapped)) = adverts_rebound("data/images/pulse-psp-usa.chd") else {
        return;
    };
    assert!(wanted > 0, "the circuit authors no placeholder with a card");
    assert_eq!(rebound, wanted, "placeholders rebound to a card");
    assert_eq!(
        wrapped, wanted,
        "Pulse's card sampler wraps (measured: TEXWRAP 0 in a GE dump): its quads author UVs past 1 and negate V"
    );
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd and a GPU"]
fn pulse_ps2_points_every_served_placeholder_at_its_card() {
    let Some((wanted, rebound, wrapped)) = adverts_rebound("data/images/pulse-ps2-eu.chd") else {
        return;
    };
    assert!(wanted > 0, "the circuit authors no placeholder with a card");
    assert_eq!(rebound, wanted, "placeholders rebound to a card");
    assert_eq!(
        wrapped, wanted,
        "Pulse PS2's card sampler wraps (PSP measured TEXWRAP 0, PS2 by analogy): its quads author UVs past 1 and negate V"
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU"]
fn hd_points_every_served_placeholder_at_its_card() {
    let Some((wanted, rebound, wrapped)) = adverts_rebound("data/images/hdfury-ps3-eu-dec.iso")
    else {
        return;
    };
    assert!(wanted > 0, "the circuit authors no placeholder with a card");
    assert_eq!(rebound, wanted, "placeholders rebound to a card");
    assert_eq!(
        wrapped, wanted,
        "RPCS3 2026-10-18: HD's track quads sample the card with address 0x60010101 (wrap)"
    );
}
