//! Omega's tangent-carrying submeshes draw with their normal map bound.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! Every circuit model Omega ships (`track.final.rcsmodel`, base and patch) is built through
//! `psp2::build`. A submesh whose declaration carries a `tangent` reads a normal map; the
//! material's own pixel programs say which sampler (`oag_rcs::gcn`, the `2x - 1` decode) and the
//! instance says which file. Remove the `normal_map::plan` call in `psp2::build` and every count
//! of `bound` reads zero, and the third binding stays empty.

use std::path::Path;

use oag_mesh::mesh::rcs::psp2;

const ARCHIVES: [&str; 9] = [
    "ps4/omega-eu-patch/uroot/data08.psarc",
    "ps4/omega-eu-patch/uroot/data09.psarc",
    "ps4/omega-eu-patch/uroot/data07.psarc",
    "ps4/omega-eu-patch/uroot/data05.psarc",
    "ps4/omega-eu/uroot/data00.psarc",
    "ps4/omega-eu/uroot/data01.psarc",
    "ps4/omega-eu/uroot/data02.psarc",
    "ps4/omega-eu/uroot/data03.psarc",
    "ps4/omega-eu/uroot/data04.psarc",
];

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu and omega-eu-patch"]
fn omega_circuits_bind_the_normal_map_their_programs_decode() {
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
    let mut models: Vec<String> = archives
        .iter()
        .flat_map(|a| a.paths().to_vec())
        .filter(|p| p.to_ascii_lowercase().ends_with("track.final.rcsmodel"))
        .collect();
    models.sort();
    models.dedup();
    assert!(!models.is_empty(), "no Omega circuit model found");
    let mut total = psp2::NormalMapCensus::default();
    for path in models {
        let index = archives
            .iter()
            .position(|a| a.contains(&path))
            .expect("listed");
        let blob = archives[index].read_path(&path).expect("the model");
        let (model, report) = psp2::build(&path, &blob, None, &mut |p| {
            archives.iter_mut().find_map(|a| a.read_path(p).ok())
        })
        .expect("builds");
        let c = &report.normal_maps;
        println!("{path}: {}", c.describe());
        for (material, why) in &c.unbound {
            println!("    {why}: {material}");
        }
        total.submeshes += c.submeshes;
        total.bound += c.bound;
        total.ambiguous += c.ambiguous;
        total.absent += c.absent;
        total.unread += c.unread;
        total.third_taken += c.third_taken;
        total.unresolved_samples += c.unresolved_samples;
        let carried = model.vertices.iter().filter(|v| v.tangent != 0).count();
        if c.bound > 0 {
            assert!(
                carried > 0,
                "{path}: a map is bound but no vertex carries a tangent"
            );
            assert!(
                model.pad_masks.iter().any(Option::is_some),
                "{path}: a map is bound but the third binding is empty"
            );
        } else {
            assert_eq!(carried, 0, "{path}: tangents written with no map bound");
        }
    }
    println!("TOTAL {}", total.describe());
    // The census, exactly: N bound of M tangent-carrying submeshes, and what the rest are.
    // `ambiguous` is a material that blends two normal maps (`DC_Rocks`, `Road_..._panelmap1`),
    // `absent` a material whose programs sample no normal map though its model carries a tangent
    // (`m_Diffuse_Normal_ConstantSpecular`: one alpha texture), `unread` a normal map the
    // archives do not hold or will not decode.
    assert_eq!(
        (
            total.bound,
            total.submeshes,
            total.ambiguous,
            total.absent,
            total.unread,
            total.third_taken,
            total.unresolved_samples
        ),
        (24590, 25977, 771, 413, 203, 0, 0),
        "{}",
        total.describe()
    );
}

/// 2048's Vita circuit: its programs are GXP, whose operands nothing reads, so it neither binds
/// a normal map nor writes a tangent.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_2048_circuit_stays_unbound() {
    let package = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
    if !package.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            package.display()
        );
        println!("skipping: {} not present", package.display());
        return;
    }
    let mut archive =
        oag_assets::psarc::Archive::open(package.to_str().expect("utf-8")).expect("opens");
    let path = "data/art/published/environments/altima/track.rcsmodel";
    let blob = archive.read_path(path).expect("the model");
    let (model, report) =
        psp2::build(path, &blob, None, &mut |p| archive.read_path(p).ok()).expect("builds");
    assert!(report.decoded_tangents > 0, "{}", report.describe());
    assert_eq!(report.normal_maps, psp2::NormalMapCensus::default());
    assert!(model.vertices.iter().all(|v| v.tangent == 0));
    assert!(model.pad_masks.is_empty());
}
