//! What the opt-in ASTC re-encode does to a circuit's textures: PSNR, the
//! largest channel error and the share of texels more than 2 levels off, per
//! astcenc preset, with the single-thread encode time
//! (`docs/rendering/texture-compression.md`, "Choosing the preset").
//!
//! A sweep, not a check: `#[ignore]`d and a no-op unless `OAG_SWEEP` is set.
//! `OAG_SWEEP=1 cargo nextest run -p oag-game --run-ignored only astc_quality --no-capture`.

use oag_game::texture_cache::astcenc;
use oag_mesh::mesh::{Model, Texels};
use oag_mesh::mesh_render::astc;
use oag_raceplay as race;
use oag_raceplay::LoadWorker;

/// Every distinct block-compressed texture of the track, sky and craft.
fn block_textures(loaded: &race::Loaded) -> Vec<std::sync::Arc<oag_mesh::mesh::ModelTexture>> {
    let mut models: Vec<&Model> = vec![&loaded.track_model];
    models.extend(loaded.sky_model.as_ref());
    models.extend(loaded.liveries.iter().map(|livery| &livery.hull));
    let mut seen = std::collections::HashSet::new();
    models
        .into_iter()
        .flat_map(|model| {
            model
                .textures
                .iter()
                .chain(model.lightmaps.iter())
                .flatten()
        })
        .filter(|texture| seen.insert(std::sync::Arc::as_ptr(texture) as usize))
        .filter(|texture| matches!(texture.texels, Texels::Blocks { .. }))
        .cloned()
        .collect()
}

fn sweep(name: &str, options: race::Options, presets: &[(&str, f32)]) {
    let loaded = LoadWorker::spawn(options, None, None, false)
        .join()
        .expect("a worker that was joined once")
        .expect("the circuit loads");
    let textures = block_textures(&loaded);
    println!("{name}: {} block-compressed textures", textures.len());
    for &(preset, quality) in presets {
        let mut encoder = astcenc::Encoder::with_quality(quality).expect("astcenc configures");
        let mut psnr = Vec::new();
        let (mut max, mut over, mut texels) = (0u8, 0f64, 0f64);
        let mut seconds = 0.0;
        for texture in &textures {
            let Texels::Blocks { format, levels } = &texture.texels else {
                continue;
            };
            let (width, height) = (texture.width, texture.height);
            if width % astc::BLOCK != 0 || height % astc::BLOCK != 0 {
                continue;
            }
            let Some(rgba) = levels
                .first()
                .and_then(|level| format.decode_level(level, width, height))
            else {
                continue;
            };
            let rgba = rgba.into_flattened();
            let started = std::time::Instant::now();
            let blocks = encoder.encode(width, height, &rgba).expect("encodes");
            seconds += started.elapsed().as_secs_f64();
            let back = encoder.decode(width, height, &blocks).expect("decodes");
            let error = astcenc::error(&rgba, &back);
            psnr.push(error.psnr);
            max = max.max(error.max);
            let n = f64::from(width * height);
            over += error.over_two * n;
            texels += n;
        }
        psnr.sort_by(f64::total_cmp);
        println!(
            "{name} {preset:>8}: {} textures, PSNR median {:.1} dB, worst {:.1} dB, \
             {:.1} % of texels off by more than 2, max {max}, base levels encoded in {seconds:.1} s \
             on one thread",
            psnr.len(),
            psnr[psnr.len() / 2],
            psnr[0],
            100.0 * over / texels,
        );
    }
}

#[test]
#[ignore = "a sweep: needs the Wipeout HD disc and OAG_SWEEP"]
fn astc_quality_of_hd_vineta_k() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    sweep(
        "HD Vineta K",
        race::Options {
            source: image.display().to_string(),
            track: Some("/data/environments/01_vineta_k/track.vex".to_string()),
            ..race::Options::default()
        },
        &[("fast", 10.0), ("medium", 60.0), ("thorough", 98.0)],
    );
}

#[test]
#[ignore = "a sweep: needs the Omega extraction and OAG_SWEEP"]
fn astc_quality_of_omega_tech_de_ra() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    sweep(
        "Omega Tech De Ra",
        race::Options {
            source: source.display().to_string(),
            track: Some(r"Data\environments\tech_de_ra\track.vex".to_string()),
            ..race::Options::default()
        },
        &[("medium", 60.0)],
    );
}
