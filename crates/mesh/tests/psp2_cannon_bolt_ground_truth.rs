//! 2048's and Omega's Detonator cannon bolt is drawn by its own material program.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! `oag_mesh::mesh::rcs::psp2`'s `cannon_bolt` plan reads `HD_Detonator_CannonBolt_Halo`'s
//! fragment program (`oag_rcs::gxp::cannon_bolt` on 2048, `oag_rcs::gcn::cannon_bolt` on Omega)
//! and binds the band as the diffuse and the ring strip second. Remove the builder's
//! `cannon_bolt::plan` call and the bound count reads zero (and the ring strip is the diffuse).
//! `docs/rendering/hd-unlit-programs.md`, "The Detonator cannon bolt on 2048 and Omega".

use std::path::Path;

use oag_mesh::mesh::PROGRAM_CANNON_BOLT;
use oag_mesh::mesh::rcs::psp2;
use oag_rcs::rcsmodel::psp2 as container;

/// Archives newest first, so a patch's material wins as it does on the console.
const VITA: [&str; 3] = [
    "vita/PCSF00007/patch-v104/PSP2/data2.psarc",
    "vita/PCSF00007/patch-v104/PSP2/data1.psarc",
    "vita/PCSF00007/base/PSP2/data.psarc",
];
const OMEGA: [&str; 3] = [
    "ps4/omega-eu-patch/uroot/data08.psarc",
    "ps4/omega-eu-patch/uroot/data05.psarc",
    "ps4/omega-eu/uroot/data00.psarc",
];

fn check(archives: [&str; 3], models: [&str; 2], tint: [f32; 3]) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
    let mut open: Vec<_> = archives
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect();
    if open.len() != archives.len() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {archives:?} is not extracted"
        );
        println!("skipping: {archives:?} not extracted");
        return;
    }
    for path in models {
        let blob = open
            .iter_mut()
            .find_map(|a| a.read_path(path).ok())
            .unwrap_or_else(|| panic!("{path} ships"));
        let (model, report) = psp2::build(path, &blob, None, &mut |p| {
            open.iter_mut().find_map(|a| a.read_path(p).ok())
        })
        .expect("builds");
        assert_eq!(
            (report.cannon_bolt_materials, report.cannon_bolt_unread),
            (1, 0),
            "{path}: {}",
            report.describe()
        );
        let entry = model
            .emissive
            .iter()
            .find(|e| e.program[0] == PROGRAM_CANNON_BOLT)
            .unwrap_or_else(|| panic!("{path}: no bolt entry"));
        assert_eq!(entry.program[1], 1.0, "the colour tap is decoded: {path}");
        assert!(
            (entry.offset - 0.498_74).abs() < 1.0e-4,
            "fresnelBase: {path}"
        );
        for (got, want) in entry.tint.iter().zip(tint) {
            assert!((got - want).abs() < 1.0e-5, "{path}: {:?}", entry.tint);
        }
        // `Uv1_Uv2`'s second pair is the noise and colour coordinate, its first the ring's.
        let decoded = container::parse(&blob).expect("parses");
        let halves = &decoded.submeshes[0].packed_texcoords;
        assert_eq!(halves.len(), model.vertices.len(), "{path}");
        for (vertex, &[a, b, c, d]) in model.vertices.iter().zip(halves) {
            assert_eq!(vertex.texcoord, [c, d], "{path}");
            assert_eq!(vertex.lightmap_texcoord[..2], [a, b], "{path}");
        }
        let draw = model
            .draws
            .iter()
            .chain(&model.transparent_draws)
            .chain(&model.alpha_tested_draws)
            .next()
            .expect("the bolt draws");
        let texture = draw.texture.expect("bound");
        let label = |s: &oag_mesh::mesh::TextureSlots| {
            s.get(texture)
                .and_then(Option::as_ref)
                .map_or_else(String::new, |t| t.label.to_ascii_lowercase())
        };
        let diffuse = label(&model.textures);
        assert!(
            diffuse.contains("cannonbolt") && !diffuse.contains("glow"),
            "{path}: the band is the diffuse, not {diffuse}"
        );
        assert!(label(&model.lightmaps).contains("glow"), "{path}");
    }
}

#[test]
#[ignore = "needs data/extracted/vita"]
fn both_2048_bolt_models_bind_their_own_program() {
    check(
        VITA,
        [
            "data/weapons/detonator_cannonbolt.rcsmodel",
            "data/weapons2048/detonator_cannonbolt.rcsmodel",
        ],
        [0.061_92, 0.744_628_9, 1.0],
    );
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn both_omega_bolt_models_bind_their_own_program() {
    check(
        OMEGA,
        [
            "Data/weapons/detonator_cannonbolt.rcsmodel",
            "Data/weapons2048/detonator_cannonbolt.rcsmodel",
        ],
        [0.061_907_5, 0.744_53, 1.0],
    );
}
