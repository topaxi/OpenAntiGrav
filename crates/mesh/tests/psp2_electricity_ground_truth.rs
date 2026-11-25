//! Omega's and 2048's Detonator mine visualiser ring is drawn by its own material program.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! `oag_mesh::mesh::rcs::psp2`'s `electricity` plan reads `electricity.rcsmaterial`'s pixel
//! program (Omega's GCN through `oag_rcs::gcn::electricity`, 2048's USSE through
//! `oag_rcs::gxp::electricity`) and binds its three pictures and two glow-table entries. Both
//! copies of `electric_visualiser` (`weapons/` and `weapons2048/`) on each title must reach
//! `PROGRAM_ELECTRICITY` and `PROGRAM_ELECTRICITY_AUDIO` with the material's authored values,
//! the ring's vertices carrying `VertexColour1.w` as the alpha; on 2048 the second entry says the
//! program squares its taps, and the model's vertex declaration carries every attribute the
//! material's vertex programs read. Remove the builder's `electricity::plan` call and the bound
//! count reads zero.

use std::path::Path;

use oag_mesh::mesh::rcs::psp2;
use oag_mesh::mesh::{PROGRAM_ELECTRICITY, PROGRAM_ELECTRICITY_AUDIO};

const ARCHIVES: [&str; 2] = [
    "ps4/omega-eu-patch/uroot/data05.psarc",
    "ps4/omega-eu/uroot/data00.psarc",
];

const VITA_ARCHIVES: [&str; 3] = [
    "vita/PCSF00007/patch-v104/PSP2/data2.psarc",
    "vita/PCSF00007/patch-v104/PSP2/data1.psarc",
    "vita/PCSF00007/base/PSP2/data.psarc",
];

/// The archives in `names`, every one or none (and then the test skips).
fn open(names: &[&str], what: &str) -> Option<Vec<oag_assets::psarc::Archive>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
    let archives: Vec<_> = names
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect();
    if archives.len() != names.len() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {what} is not extracted"
        );
        println!("skipping: {what} not extracted");
        return None;
    }
    Some(archives)
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu and omega-eu-patch"]
fn both_omega_visualiser_models_bind_the_electricity_program() {
    let Some(mut archives) = open(&ARCHIVES, "Omega") else {
        return;
    };
    for path in [
        "Data/weapons/electric_visualiser.rcsmodel",
        "Data/weapons2048/electric_visualiser.rcsmodel",
    ] {
        binds_the_ring(&mut archives, path, 0.0);
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn both_2048_visualiser_models_bind_the_electricity_program() {
    let Some(mut archives) = open(&VITA_ARCHIVES, "2048") else {
        return;
    };
    for path in [
        "data/Weapons/electric_visualiser.rcsmodel",
        "data/Weapons2048/electric_visualiser.rcsmodel",
    ] {
        binds_the_ring(&mut archives, path, 1.0);
        declares_what_the_vertex_programs_read(&mut archives, path);
    }
}

/// The model at `path` builds with its ring bound; `squared` is the second entry's scale.
fn binds_the_ring(archives: &mut [oag_assets::psarc::Archive], path: &str, squared: f32) {
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
    assert_eq!(
        model.emissive[at + 1].scale,
        squared,
        "squared taps: {path}"
    );
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

/// Every attribute any vertex program of the ring's material reads is in the model's own vertex
/// declaration, by name hash: `position`, `normal`, `VertexColour1`, `Uv3`, `Uv1_Uv2` on the
/// unskinned program.
fn declares_what_the_vertex_programs_read(archives: &mut [oag_assets::psarc::Archive], path: &str) {
    let read = |archives: &mut [oag_assets::psarc::Archive], p: &str| {
        archives
            .iter_mut()
            .find_map(|a| a.read_path(p).ok())
            .unwrap_or_else(|| panic!("{p} ships"))
    };
    let blob = read(archives, path);
    let model = oag_rcs::rcsmodel::psp2::parse(&blob).expect("parses");
    let cpu = model.sections.first().expect("a CPU section");
    let declared: Vec<u32> =
        oag_rcs::rcsmodel::psp2::vertex_decl::find_by_header(&blob[cpu.at..cpu.at + cpu.len])
            .into_values()
            .flat_map(|d| d.attributes.into_iter().map(|a| a.name_hash))
            .collect();
    let material = model
        .materials
        .iter()
        .find(|m| {
            m.name
                .to_ascii_lowercase()
                .ends_with("electricity.rcsmaterial")
        })
        .expect("the ring's material");
    let file = read(archives, &material.name);
    let mut unskinned: Option<Vec<String>> = None;
    for (_, program) in oag_rcs::gxp::programs(&file) {
        let program = program.expect("decodes");
        if program.is_fragment() {
            continue;
        }
        let mut names: Vec<&str> = program
            .parameters
            .iter()
            .filter(|q| q.category == oag_rcs::gxp::Category::Attribute)
            .map(|q| q.name.as_str())
            .collect();
        names.sort_unstable();
        if names.len() == 5 {
            unskinned = Some(names.iter().map(|n| (*n).to_string()).collect::<Vec<_>>());
        }
        let read_here = ["position", "normal", "VertexColour1", "Uv3", "Uv1_Uv2"];
        for name in read_here {
            if names.contains(&name) {
                let hash = oag_rcs::rcsmaterial::name_hash(name);
                assert!(declared.contains(&hash), "{path}: {name} is not declared");
            }
        }
    }
    let want = ["Uv1_Uv2", "Uv3", "VertexColour1", "normal", "position"];
    assert_eq!(unskinned, Some(want.map(str::to_string).to_vec()), "{path}");
}
