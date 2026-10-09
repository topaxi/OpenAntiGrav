//! Scratch probe: what the PS4 `.rcsmodel` of each Omega front-end circuit
//! model names as materials and textures, and whether they resolve.
//!
//! `cargo run -p oag-game --example omega_track_model_probe -- data/extracted/ps4 <entry>`
fn main() {
    let mut args = std::env::args().skip(1);
    let source = args.next().expect("source");
    let entry = args.next().expect("entry");
    let options = oag_game::boot::Options {
        language: None,
        source,
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-omega-track-model-probe"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    let (_, mut archives, _) = oag_game::boot::load_shell(&options).expect("boots");
    let sibling = oag_mesh::mesh::rcs::sibling_name(&entry).expect("sibling");
    let blob = archives.read_name(&sibling).expect("rcsmodel");
    let Ok(model) = oag_rcs::rcsmodel::psp2::parse(&blob) else {
        return bounds(&mut archives, &entry);
    };
    for (i, m) in model.materials.iter().enumerate() {
        println!(
            "material {i}: textures {:?} lightmap {:?}",
            m.textures, m.lightmap
        );
        for p in &m.params {
            let v: Vec<f32> = p.bits.iter().map(|b| f32::from_bits(*b)).collect();
            println!("   param {:#010x} {v:?}", p.hash);
        }
        for t in &m.textures {
            println!("   read {t}: {}", archives.read_name(t).is_ok());
        }
    }
    println!("submeshes {}", model.submeshes.len());
    let vex = archives.read_name(&entry).expect("vex");
    match oag_mesh::mesh::rcs::psp2::build_with_vex(&entry, &blob, &vex, &mut |p| {
        archives.read_name(p).ok()
    }) {
        Ok((m, r)) => println!("build: {} tris; {}", m.indices.len() / 3, r.describe()),
        Err(e) => println!("build error {e:#}"),
    }
    for s in model.submeshes.iter().take(16) {
        println!("  submesh {s:?}");
    }
    bounds(&mut archives, &entry);
}

fn bounds(archives: &mut oag_assets::Archives, entry: &str) {
    let mut built = oag_game::preview::model(archives, entry).expect("builds");
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for v in &built.vertices {
        for c in 0..3 {
            lo[c] = lo[c].min(v.position[c]);
            hi[c] = hi[c].max(v.position[c]);
        }
    }
    println!(
        "{entry}: {} vertices, bounds {lo:?} .. {hi:?}",
        built.vertices.len()
    );
    println!("anim nodes {}", built.anim_nodes.len());
    for (i, m) in built.sample_anim_nodes(1.0).iter().enumerate().take(6) {
        println!("  node {i}: {m:?}");
    }
    let far = built
        .vertices
        .iter()
        .filter(|v| v.position.iter().any(|c| c.abs() > 100.0))
        .count();
    println!(
        "vertices beyond 100: {far}; xform slots used: {}",
        built.vertices.iter().filter(|v| v.xform != 0).count()
    );
    built.vertices.clear();
}
