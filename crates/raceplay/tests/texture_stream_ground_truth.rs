//! A load given a device leaves no decoded texture on the CPU.
//!
//! `race::load` decodes every texture of every model before the scene builds a
//! single drawable, so Tech De Ra's peak was all 2.3 GiB of its BC7 blocks at
//! once. With a [`TextureSink`] open each texture goes up as it is decoded and
//! the model keeps a view in its place. These tests fail if the sink is dropped
//! anywhere between the worker and the decode: the control run has texels, the
//! streamed run has none.
//!
//! `#[ignore]`d: they read the Omega extraction and the HD disc, and need a GPU
//! adapter (they skip without one).

use oag_mesh::mesh::{Model, Texels};
use oag_raceplay as race;
use oag_raceplay::{LoadWorker, Loaded, TextureSink};

/// A sink on a device that asks for wgpu's **default** limits, whose
/// `max_texture_dimension_2d` is 8,192 - whatever the adapter could do.
fn sink() -> Option<TextureSink> {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: adapter.features() & wgpu::Features::TEXTURE_COMPRESSION_BC,
        ..Default::default()
    }))
    .expect("requesting the device");
    Some(TextureSink { device, queue })
}

/// `(uploaded, still on the CPU, CPU bytes)` over one model's distinct textures.
fn census(model: &Model, seen: &mut std::collections::HashSet<usize>) -> (usize, usize, u64) {
    let mut out = (0, 0, 0);
    for texture in model
        .textures
        .iter()
        .chain(model.lightmaps.iter())
        .flatten()
        .filter(|t| seen.insert(std::sync::Arc::as_ptr(t) as usize))
    {
        if matches!(texture.texels, Texels::Uploaded { .. }) {
            out.0 += 1;
        } else {
            out.1 += 1;
        }
        out.2 += texture.cpu_bytes();
    }
    out
}

fn models(loaded: &Loaded) -> Vec<&Model> {
    let mut all = vec![&loaded.track_model];
    all.extend(loaded.sky_model.as_ref());
    all.extend(loaded.liveries.iter().map(|livery| &livery.hull));
    all
}

fn totals(loaded: &Loaded) -> (usize, usize, u64) {
    let mut seen = std::collections::HashSet::new();
    models(loaded).into_iter().fold((0, 0, 0), |acc, model| {
        let one = census(model, &mut seen);
        (acc.0 + one.0, acc.1 + one.1, acc.2 + one.2)
    })
}

fn load(options: race::Options, sink: Option<TextureSink>) -> Loaded {
    LoadWorker::spawn(options, None, sink)
        .join()
        .expect("a worker that was joined once")
        .expect("the circuit loads")
}

fn omega_options() -> Option<race::Options> {
    let source = oag_testdata::exact("data/extracted/ps4")?;
    Some(race::Options {
        source: source.display().to_string(),
        track: Some(r"Data\environments\tech_de_ra\track.vex".to_string()),
        ..race::Options::default()
    })
}

/// Tech De Ra on Omega: 461 albedo + 51 lightmap + 6 sky + the craft, all BC7,
/// which since 2026-10-07 decode once per file: 200 distinct textures and
/// 1.4 GiB held without a sink (`docs/architecture/load-time.md`).
#[test]
#[ignore = "needs the Omega extraction and a GPU adapter"]
fn an_omega_circuit_loaded_through_a_sink_holds_no_texels() {
    let Some(options) = omega_options() else {
        return;
    };
    let Some(sink) = sink() else { return };

    let control = totals(&load(options.clone(), None));
    assert!(
        control.1 > 150 && control.0 == 0 && control.2 > 1 << 30,
        "without a sink the load keeps its decoded blocks (uploaded, held, bytes): {control:?}"
    );

    let streamed = totals(&load(options, Some(sink)));
    assert_eq!(
        (streamed.1, streamed.2),
        (0, 0),
        "with a sink no texture may keep texels (uploaded, held, bytes): {streamed:?}"
    );
    assert_eq!(
        streamed.0, control.1,
        "every texture the control decoded went up instead"
    );
}

/// HD shares the RCS path through `skin::decode_texture`, a different decode
/// site from Omega's `psp2::decode_material_texture`.
#[test]
#[ignore = "needs the Wipeout HD disc and a GPU adapter"]
fn a_hd_circuit_loaded_through_a_sink_holds_no_track_texels() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let Some(sink) = sink() else { return };
    let options = race::Options {
        source: image.display().to_string(),
        ..race::Options::default()
    };
    let loaded = load(options, Some(sink));
    let mut seen = std::collections::HashSet::new();
    let track = census(&loaded.track_model, &mut seen);
    assert!(track.0 > 100, "the track's textures went up: {track:?}");
    assert_eq!(
        (track.1, track.2),
        (0, 0),
        "no track texture keeps texels: {track:?}"
    );
}

/// Talon's Junction ships `ds_floor_cs.gnf` 16,384 texels wide.
fn talons_junction_options() -> Option<race::Options> {
    let source = oag_testdata::exact("data/extracted/ps4")?;
    Some(race::Options {
        source: source.display().to_string(),
        track: Some(r"Data\environments\talons_junction\track.vex".to_string()),
        ..race::Options::default()
    })
}

/// `(base width, levels left out)` of every uploaded texture of the track named
/// `ds_floor_cs`.
fn floor_texture(loaded: &Loaded) -> Vec<(u32, u32)> {
    loaded
        .track_model
        .textures
        .iter()
        .flatten()
        .filter(|texture| texture.label.contains("ds_floor_cs"))
        .map(|texture| match &texture.texels {
            Texels::Uploaded { dropped_levels, .. } => (texture.width, *dropped_levels),
            other => panic!("the floor is not on the GPU: {other:?}"),
        })
        .collect()
}

/// The panic this exists for: a device at the default 8,192 limit used to
/// reject the 16,384-wide floor in `create_texture`. It now goes up from level
/// 1 (chosen, not measured) and the load completes.
#[test]
#[ignore = "needs the Omega extraction and a GPU adapter"]
fn talons_junction_loads_on_a_device_too_small_for_its_floor() {
    let Some(options) = talons_junction_options() else {
        return;
    };
    let Some(sink) = sink() else { return };
    assert_eq!(sink.device.limits().max_texture_dimension_2d, 8192);
    let floor = floor_texture(&load(options, Some(sink)));
    assert!(!floor.is_empty(), "the circuit names ds_floor_cs.gnf");
    assert!(
        floor
            .iter()
            .all(|&(width, dropped)| width == 16384 && dropped == 1),
        "the 16,384-wide floor goes up from level 1 on an 8,192 device: {floor:?}"
    );
}

/// The device every window and capture asks for takes the adapter's own limit,
/// so on an adapter that allows it the floor goes up whole.
#[test]
#[ignore = "needs the Omega extraction and a GPU adapter"]
fn talons_junction_floor_goes_up_whole_on_the_renderers_own_device() {
    let Some(options) = talons_junction_options() else {
        return;
    };
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) = pollster::block_on(adapter.request_device(
        &oag_mesh::mesh_render::device_descriptor("talon floor", &adapter),
    ))
    .expect("requesting the device");
    let limit = device.limits().max_texture_dimension_2d;
    let floor = floor_texture(&load(options, Some(TextureSink { device, queue })));
    let expected = u32::from(limit < 16384);
    assert!(
        floor
            .iter()
            .all(|&(width, dropped)| width == 16384 && dropped == expected),
        "on a device limited to {limit}: {floor:?}"
    );
}
