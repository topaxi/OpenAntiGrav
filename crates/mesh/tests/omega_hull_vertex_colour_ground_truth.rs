//! Omega's hull draws multiply the baked occlusion colour into the light, read off the hull
//! materials' own programs.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! Feisar `c1`'s hull (`hdships/feisar_c1/ship.rcsmodel`, the one the Modesto Heights pair
//! draws) is built through `psp2::build`. Its `diffuse_vcol`, `diffuse_with_specular_from_alpha_n_vcol`
//! and `carbonfibre` submeshes carry `VertexColour1` as a grey occlusion bake
//! (`docs/ghidra/functions/ps4-omega-eu/hull-tone.md`, "Recipe"). Remove
//! `psp2::hull_colour::plan` and every colour is white again, no vertex carries the factor marker
//! and the census names no material.

use oag_assets::psarc::Archive;
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

const HULL: &str = "Data/art/published/hdships/feisar_c1/ship.rcsmodel";

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu and omega-eu-patch"]
fn feisar_c1_draws_its_hull_through_the_baked_occlusion_colour() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
    let mut archives: Vec<Archive> = ARCHIVES
        .iter()
        .filter_map(|a| Archive::open_file(&root.join(a)).ok())
        .collect();
    if archives.len() != ARCHIVES.len() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but Omega is not extracted"
        );
        println!("skipping: Omega not extracted");
        return;
    }
    let blob = archives
        .iter_mut()
        .find_map(|a| a.read_path(HULL).ok())
        .expect("the hull is in the archives");
    let (model, report) = psp2::build("feisar_c1", &blob, None, &mut |p| {
        archives.iter_mut().find_map(|a| a.read_path(p).ok())
    })
    .expect("builds");
    let census = &report.hull_colour;
    println!("{}", report.describe());
    let named = |family: &str| {
        census
            .submeshes
            .keys()
            .any(|m| m.to_ascii_lowercase().contains(family))
    };
    assert!(named("diffuse_vcol"), "{census:?}");
    assert!(
        named("diffuse_with_specular_from_alpha_n_vcol"),
        "{census:?}"
    );
    assert!(named("carbonfibre"), "{census:?}");

    let marked: Vec<_> = model.vertices.iter().filter(|v| v.sun_mask < 0.0).collect();
    assert!(marked.len() > 20_000, "{} marked vertices", marked.len());
    assert!(
        marked
            .iter()
            .all(|v| v.sun_mask == oag_mesh::mesh::factor_sun_mask(1.0) && v.colour[3] == 1.0)
    );
    // A byte over 255, never an sRGB decode: the grey bake runs dark on this hull
    // (means 0.12 to 0.61 per submesh, `hull-tone.md`).
    let mean = marked.iter().map(|v| v.colour[0]).sum::<f32>() / marked.len() as f32;
    assert!((0.15..0.5).contains(&mean), "mean {mean}");
    assert!(
        marked
            .iter()
            .all(|v| v.colour[..3].iter().all(|c| (0.0..=1.0).contains(c)))
    );
    let white = marked.iter().filter(|v| v.colour[..3] == [1.0; 3]).count();
    assert!(
        white * 10 < marked.len(),
        "{white} of {} are white",
        marked.len()
    );
}
