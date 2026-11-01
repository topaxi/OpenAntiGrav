//! Wipeout 2048's Vineta K leaves the meshes named `wohdmeshrefract` out of the main view, as
//! the original does: it draws them only into the half-size target its glass samples as
//! `screenSpaceRefractionTex` (`docs/ghidra/functions/vita-2048-eu-v104/refraction-pass.md`,
//! `TrackPass_FilterSubmesh` at `0x813b9670`, confidence 82).
//!
//! **`#[ignore]`d and never run in CI**; needs `data/extracted/vita/PCSF00007`. Drop the
//! `refraction_only` call in `load::geometry::track_model` and it fails naming the leaked meshes.
//!
//! The rule's set is not HD's: the tunnel's `tunnel_understruts*` pieces carry no tag, so the
//! original draws them in its main view and so does this build.

use std::collections::BTreeSet;
use std::path::Path;

use oag_raceplay as race;
use oag_rcs::rcsmodel::psp2;

const VINETA_K: &str = r"Data\art\published\DLC1\environments\Vineta_K\track.vex";

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn vineta_k_leaves_its_refraction_only_meshes_out_of_the_main_view() {
    let package = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted/vita/PCSF00007");
    let archive = package.join("base/PSP2/data.psarc");
    if !archive.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            archive.display()
        );
        return;
    }
    let mut open =
        oag_assets::psarc::Archive::open(archive.to_str().expect("utf-8")).expect("opens");
    let blob = open
        .read_path("data/art/published/DLC1/environments/Vineta_K/track.rcsmodel")
        .expect("the model");
    let scene = psp2::parse(&blob).expect("parses").scene;
    let named = |f: &dyn Fn(&str) -> bool| -> BTreeSet<u32> {
        (0..scene.meshes.len() as u32)
            .filter(|&i| f(&scene.meshes[i as usize].name.to_ascii_lowercase()))
            .collect()
    };
    let refract = named(&|n| n.contains("wohdmeshrefract"));
    let road = named(&|n| n.contains("wohdtrack"));
    let struts = named(&|n| n.contains("tunnel_understruts"));
    assert_eq!(refract.len(), 238, "the census");
    assert!(
        refract.is_disjoint(&road),
        "no road mesh is refraction-only"
    );
    assert!(refract.is_disjoint(&struts), "the understruts carry no tag");

    let loaded = race::load(&race::Options {
        source: package.display().to_string(),
        class: "VENOM".to_string(),
        track: Some(VINETA_K.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let model = &loaded.track_model;
    let drawn: BTreeSet<u32> = model
        .draws
        .iter()
        .chain(&model.alpha_tested_draws)
        .chain(&model.transparent_draws)
        .chain(&model.shine_draws)
        .filter(|d| !d.range.is_empty())
        .filter_map(|d| d.chunk)
        .collect();
    let leaked: Vec<_> = refract.intersection(&drawn).collect();
    assert!(
        leaked.is_empty(),
        "refraction-only meshes drawn: {leaked:?}"
    );
    assert!(
        road.iter().filter(|c| drawn.contains(c)).count() * 2 > road.len(),
        "the road is drawn"
    );
    assert!(
        struts.iter().any(|c| drawn.contains(c)),
        "the understruts stay in the main view"
    );
    assert!(
        loaded.report.iter().any(|l| l.contains("wohdmeshrefract")),
        "the load report names what it left out"
    );
}
