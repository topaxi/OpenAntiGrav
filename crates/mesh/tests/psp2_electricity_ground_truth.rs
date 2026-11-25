//! Omega's Detonator mine visualiser ring is drawn by its own material program.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! `oag_mesh::mesh::rcs::psp2`'s `electricity` plan reads `electricity.rcsmaterial`'s GCN pixel
//! program (`oag_rcs::gcn::electricity`) and binds its three pictures and two glow-table entries.
//! Both Omega copies of `electric_visualiser` (`weapons/` and `weapons2048/`) must reach
//! `PROGRAM_ELECTRICITY` and `PROGRAM_ELECTRICITY_AUDIO` with the material's authored values,
//! the ring's vertices carrying `VertexColour1.w` as the alpha. Remove the builder's
//! `electricity::plan` call and the bound count reads zero.

use std::path::Path;

use oag_mesh::mesh::rcs::psp2;
use oag_mesh::mesh::{PROGRAM_ELECTRICITY, PROGRAM_ELECTRICITY_AUDIO};

const ARCHIVES: [&str; 2] = [
    "ps4/omega-eu-patch/uroot/data05.psarc",
    "ps4/omega-eu/uroot/data00.psarc",
];

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu and omega-eu-patch"]
fn both_omega_visualiser_models_bind_the_electricity_program() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
    let mut archives: Vec<_> = ARCHIVES
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect();
    if archives.len() != ARCHIVES.len() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but Omega is not extracted"
        );
        println!("skipping: Omega not extracted");
        return;
    }
    for path in [
        "Data/weapons/electric_visualiser.rcsmodel",
        "Data/weapons2048/electric_visualiser.rcsmodel",
    ] {
        let blob = archives
            .iter_mut()
            .find_map(|a| a.read_path(path).ok())
            .unwrap_or_else(|| panic!("{path} ships"));
        let (model, report) = psp2::build(path, &blob, None, &mut |p| {
            archives.iter_mut().find_map(|a| a.read_path(p).ok())
        })
        .expect("builds");
        assert_eq!(
            (report.electricity_materials, report.electricity_unread),
            (1, 0),
            "{path}: {}",
            report.describe()
        );
        let programs: Vec<f32> = model.emissive.iter().map(|e| e.program[0]).collect();
        let at = programs
            .iter()
            .position(|&p| p == PROGRAM_ELECTRICITY)
            .unwrap_or_else(|| panic!("{path}: no electricity entry"));
        assert_eq!(
            programs.get(at + 1),
            Some(&PROGRAM_ELECTRICITY_AUDIO),
            "{path}"
        );
        let entry = model.emissive[at];
        assert_eq!(entry.scale, 1.0, "Tempo: {path}");
        assert!(entry.program[1] > 0.0, "FacingPower is authored: {path}");
        assert_eq!(model.vertices.len(), 132, "{path}");
        let alphas: Vec<f32> = model.vertices.iter().map(|v| v.sun_mask).collect();
        assert!(
            alphas.contains(&0.0) && alphas.contains(&1.0),
            "VertexColour1.w is the ring's alpha: {path}"
        );
        let draw = model
            .draws
            .iter()
            .chain(&model.transparent_draws)
            .chain(&model.alpha_tested_draws)
            .next()
            .expect("the ring draws");
        let texture = draw.texture.expect("bound");
        let label = |s: &oag_mesh::mesh::TextureSlots| {
            s.get(texture)
                .and_then(Option::as_ref)
                .map_or_else(String::new, |t| t.label.to_ascii_lowercase())
        };
        assert!(label(&model.textures).contains("bass_treble"), "{path}");
        assert!(label(&model.lightmaps).contains("pitch"), "{path}");
        assert!(label(&model.pad_masks).contains("uv_dist"), "{path}");
    }
}
