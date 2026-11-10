//! Omega's tangent-carrying submeshes draw with their normal map bound.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! Every circuit model Omega ships (`track.final.rcsmodel`, base and patch) is built through
//! `psp2::build`. A submesh whose declaration carries a `tangent` reads a normal map; the
//! material's own pixel programs say which sampler (`oag_rcs::gcn`, the `2x - 1` decode) and the
//! instance says which file. Remove the `normal_map::plan` call in `psp2::build` and every count
//! of `bound` reads zero, and the third binding stays empty.
//!
//! The same pass checks Tech De Ra's see-through materials, read off each material's own state
//! word (`omega_circuits_bind_the_normal_map_their_programs_decode` calls
//! `tech_de_ras_glass_and_cutouts_leave_the_opaque_list`'s checks): a PS4 material carries HD's
//! state word at header `+0x22`, its low two bits select blended (1) or alpha-tested (2), and the
//! build routes a textured draw of either kind out of the opaque list. Remove
//! `psp2::transparency::route` and those assertions fail: the glass tubes and road panels are
//! 100 % opaque draws again. The model is the one the game boots, the patch's where it ships one.

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

/// Tech De Ra's glass and cut-outs leave the opaque list.
fn tech_de_ras_glass_and_cutouts_leave_the_opaque_list(
    model: &oag_mesh::mesh::Model,
    report: &psp2::Report,
) {
    assert!(model.transparent_draws.len() > 50, "{}", report.describe());
    assert_eq!(report.blended_draws, model.transparent_draws.len());
    assert!(
        model
            .transparent_draws
            .iter()
            .all(|d| d.blend_state.is_some() && d.texture.is_some())
    );
    // The blend is HD's authored pair where HD has one for the name: some draws
    // are not alpha-over, and the count in the report is the draws that are not.
    let default = Some(oag_mesh::mesh_render::TRANSPARENT_BLEND);
    let off_default = model
        .transparent_draws
        .iter()
        .filter(|d| d.blend_state != default)
        .count();
    assert!(off_default > 0, "{}", report.describe());
    assert_eq!(report.inherited_blend_draws, off_default);
    assert!(
        !model.alpha_tested_draws.is_empty(),
        "{}",
        report.describe()
    );
    assert_eq!(model.alpha_test_ref, Some(0.5));
    assert!(model.draws.len() > model.transparent_draws.len());
}

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
    let mut checked_tech_de_ra = false;
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
        if path
            .to_ascii_lowercase()
            .ends_with("data/environments/tech_de_ra/track.final.rcsmodel")
        {
            tech_de_ras_glass_and_cutouts_leave_the_opaque_list(&model, &report);
            checked_tech_de_ra = true;
        }
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
    assert!(
        checked_tech_de_ra,
        "Tech De Ra's circuit model was not among the models built"
    );
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
        (24793, 25977, 771, 413, 0, 0, 0),
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
