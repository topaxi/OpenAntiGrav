//! What building a [`Scene`] costs on the GPU side, as opposed to what it draws.

use super::*;
use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex};

fn vertex(position: [f32; 3]) -> GpuVertex {
    GpuVertex {
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
    }
}

/// One opaque triangle: enough geometry that every pool builds a drawable.
fn triangle() -> Model {
    Model {
        vertices: vec![
            vertex([-1.0, 0.0, 0.0]),
            vertex([1.0, 0.0, 0.0]),
            vertex([0.0, 1.0, 0.0]),
        ],
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
            moving: false,
            blend: None,
            blend_state: None,
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

fn livery() -> Livery {
    Livery {
        team: "Test".to_string(),
        hull: triangle(),
        nozzle: None,
        boost: None,
        flare: None,
        shield: None,
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
    }
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
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(
        &mesh_render::device_descriptor("scene build test", &adapter),
    )) else {
        eprintln!("no GPU device: skipping");
        return;
    };
    let scene = Scene::new(
        &device,
        &queue,
        triangle(),
        &[livery()],
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
        None,
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
        Vec::new(),
        Vec::new(),
        Default::default(),
    )
    .expect("the scene builds");

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

/// **A circuit's advert cards are what its placeholder quads sample.**
///
/// Builds the real scene of each disc's default circuit and counts the track
/// materials `Scene::new` pointed at a card. Drop the rebinding and every
/// placeholder shows its 8x8 digit stub (or, once stripped, nothing), and this
/// reads zero.
fn adverts_rebound(source: &str) -> Option<(usize, usize)> {
    let image = oag_testdata::image(source)?;
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
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
        None,
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
        Vec::new(),
        Vec::new(),
        Default::default(),
    )
    .expect("the scene builds");
    Some((wanted, scene.adverts_rebound()))
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU"]
fn pulse_psp_points_every_served_placeholder_at_its_card() {
    let Some((wanted, rebound)) = adverts_rebound("data/images/pulse-psp-usa.chd") else {
        return;
    };
    assert!(wanted > 0, "the circuit authors no placeholder with a card");
    assert_eq!(rebound, wanted, "placeholders rebound to a card");
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd and a GPU"]
fn pulse_ps2_points_every_served_placeholder_at_its_card() {
    let Some((wanted, rebound)) = adverts_rebound("data/images/pulse-ps2-eu.chd") else {
        return;
    };
    assert!(wanted > 0, "the circuit authors no placeholder with a card");
    assert_eq!(rebound, wanted, "placeholders rebound to a card");
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU"]
fn hd_points_every_served_placeholder_at_its_card() {
    let Some((wanted, rebound)) = adverts_rebound("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    assert!(wanted > 0, "the circuit authors no placeholder with a card");
    assert_eq!(rebound, wanted, "placeholders rebound to a card");
}
