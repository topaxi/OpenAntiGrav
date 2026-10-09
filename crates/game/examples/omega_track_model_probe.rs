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
    let model = oag_rcs::rcsmodel::psp2::parse(&blob).expect("parses");
    for (i, m) in model.materials.iter().enumerate() {
        println!("material {i}: textures {:?} lightmap {:?}", m.textures, m.lightmap);
        for t in &m.textures {
            println!("   read {t}: {}", archives.read_name(t).is_ok());
        }
    }
    let _ = std::fs::write(std::env::var("DUMP").unwrap_or_default(), &blob);
    println!("submeshes {}", model.submeshes.len());
}
