//! Wipeout 2048's Vineta K leaves the meshes named `wohdmeshrefract` out of the main view, as
//! the original does: it draws them only into the half-size target its glass samples as
//! `screenSpaceRefractionTex` (`docs/ghidra/functions/vita-2048-eu-v104/refraction-pass.md`,
//! `TrackPass_FilterSubmesh` at `0x813b9670`, confidence 82).
//!
//! **`#[ignore]`d and never run in CI**; needs `data/extracted/vita/PCSF00007`. Drop the
//! `refraction_only` call in `load::geometry::track_model` and it fails naming the leaked meshes.
//!
//! Omega makes the same two tests in `FUN_01605760` (confidence 80,
//! `docs/ghidra/functions/ps4-omega-eu/refraction-pass.md`); its test needs `data/extracted/ps4`.
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

/// Omega's tag census: `(archive, path, tagged meshes)` for every `track*.rcsmodel` under an
/// `environments` directory of the base and patch packages that carries the tag.
fn omega_census(root: &Path) -> Vec<(String, String, usize)> {
    let mut found = Vec::new();
    for package in ["omega-eu/uroot", "omega-eu-patch/uroot"] {
        let dir = root.join(package);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut archives: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "psarc"))
            .collect();
        archives.sort();
        for path in archives {
            let mut open = oag_assets::psarc::Archive::open_file(&path).expect("opens");
            let models: Vec<String> = open
                .paths()
                .iter()
                .filter(|p| {
                    let p = p.to_ascii_lowercase();
                    p.contains("environments")
                        && p.ends_with(".rcsmodel")
                        && p.rsplit('/').next().is_some_and(|f| f.starts_with("track"))
                })
                .cloned()
                .collect();
            for model in models {
                let blob = open.read_path(&model).expect("reads");
                let Ok(parsed) = psp2::parse(&blob) else {
                    continue;
                };
                let tagged = parsed
                    .scene
                    .meshes
                    .iter()
                    .filter(|m| m.name.to_ascii_lowercase().contains("wohdmeshrefract"))
                    .count();
                if tagged > 0 {
                    let name = path
                        .file_name()
                        .expect("a name")
                        .to_string_lossy()
                        .into_owned();
                    found.push((name, model, tagged));
                }
            }
        }
    }
    found
}

/// Omega's `FUN_01605760` (slot `+0x10` of draw-list context vtable `0x01926470`) rejects flag
/// `0x20` in pass kind 0 as the Vita's filter does
/// (`docs/ghidra/functions/ps4-omega-eu/refraction-pass.md`), so Omega's Vineta K leaves the
/// same tagged meshes out of its main view.
#[test]
#[ignore = "needs data/extracted/ps4"]
fn omega_vineta_k_leaves_its_refraction_only_meshes_out_of_the_main_view() {
    let Some(root) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    // Unlike the Vita, Omega's Sebenco Climb carries one tagged mesh too
    // (`ice_tunnel_wohdmeshrefract2Shape`), so the rule moves that circuit as well.
    let census: Vec<_> = omega_census(&root)
        .into_iter()
        .map(|(a, p, n)| (a, p.to_ascii_lowercase(), n))
        .collect();
    let row = |a: &str, p: &str, n| (a.to_string(), format!("data/environments/{p}"), n);
    assert_eq!(
        census,
        [
            row(
                "data01.psarc",
                "01_vineta_k/track_reversed.final.rcsmodel",
                212
            ),
            row("data01.psarc", "01_vineta_k/track.final.rcsmodel", 212),
            row("data02.psarc", "10_sebenco_climb/track.final.rcsmodel", 1),
            row(
                "data02.psarc",
                "10_sebenco_climb/track_reversed.final.rcsmodel",
                1
            ),
        ],
        "the census over the base and patch packages"
    );

    let mut archives = oag_omega::open(&root.display().to_string()).expect("opening Omega");
    let blob = archives
        .read_name("/data/environments/01_vineta_k/track.final.rcsmodel")
        .expect("the model the race loads");
    let scene = psp2::parse(&blob).expect("parses").scene;
    let named = |f: &dyn Fn(&str) -> bool| -> BTreeSet<u32> {
        (0..scene.meshes.len() as u32)
            .filter(|&i| f(&scene.meshes[i as usize].name.to_ascii_lowercase()))
            .collect()
    };
    let refract = named(&|n| n.contains("wohdmeshrefract"));
    let road = named(&|n| n.contains("wohdtrack"));
    assert_eq!(
        (refract.len(), scene.meshes.len()),
        (212, 2878),
        "the loaded model's census"
    );
    assert!(
        refract.is_disjoint(&road),
        "no road mesh is refraction-only"
    );

    let loaded = race::load(&race::Options {
        source: root.display().to_string(),
        track: Some(r"Data\environments\01_vineta_k\track.vex".to_string()),
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
        "refraction-only meshes drawn: {} of {}",
        leaked.len(),
        refract.len()
    );
    assert!(
        road.iter().filter(|c| drawn.contains(c)).count() * 2 > road.len(),
        "the road is drawn"
    );
    assert!(
        loaded.report.iter().any(|l| l.contains("wohdmeshrefract")),
        "the load report names what it left out"
    );
}
