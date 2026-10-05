//! What Wipeout 2048's engine trail and engine flare files say, over every one
//! the Vita packages ship - both regions, the base archives, the DLC and the
//! v1.04 patch.
//!
//! The claims `oag_2048`'s `exhaust` and `flare` axes rest on, made executable:
//!
//! - every `EngineFlare.vex` has a `.rcsmodel` sibling that decodes with
//!   [`oag_rcs::rcsmodel::psp2`] and carries geometry;
//! - its node tree hangs exactly the two groups HD's does, `EF_Main` and
//!   `EF_Boost` (case-insensitively: 2048 spells them `ef_Main` on some
//!   craft), and **every drawn mesh object sits under one of them**;
//! - every mesh object's name is the name of a `.vex` node, which is the join
//!   `oag_mesh::mesh::groups::split_psp2` uses;
//! - every flare's material is the one `2048_engine_additive` shared name, and
//!   names the same two textures;
//! - the ribbon template `EngineTrail_BlueRed_triangle` decodes, its material
//!   is `hd_enginetrail_bluered` and names two textures that exist.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use oag_rcs::rcsmodel::psp2;

const PACKAGES: [&str; 9] = [
    "vita/PCSF00007/base/PSP2/data.psarc",
    "vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
    "vita/PCSF00007/patch-v104/PSP2/data1.psarc",
    "vita/PCSF00007/patch-v104/PSP2/data2.psarc",
    "vita/PCSA00015/base/PSP2/data.psarc",
    "vita/PCSA00015/dlc1/PSP2/dlc1.psarc",
    "vita/PCSA00015/patch-v104/PSP2/data1.psarc",
    "vita/PCSA00015/patch-v104/PSP2/data2.psarc",
];

fn package(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted")
        .join(name);
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// What one package's flare files measured.
#[derive(Default, Debug, PartialEq, Eq)]
struct Flares {
    pairs: usize,
    main_meshes: usize,
    boost_meshes: usize,
    ungrouped_meshes: usize,
    unmatched_meshes: usize,
    missing_groups: usize,
    materials: BTreeSet<String>,
    textures: BTreeSet<String>,
    vex_without_sibling: usize,
    group_gaps: Vec<String>,
    flame_test: Vec<String>,
}

fn survey(name: &str) -> Option<Flares> {
    let path = package(name)?;
    let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
        .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let paths: Vec<String> = archive.paths().to_vec();
    let mut out = Flares::default();
    for vex_path in paths
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with("/engineflare.vex"))
    {
        let sibling = format!("{}.rcsmodel", &vex_path[..vex_path.len() - ".vex".len()]);
        let Some(model_path) = paths.iter().find(|p| p.eq_ignore_ascii_case(&sibling)) else {
            out.vex_without_sibling += 1;
            continue;
        };
        let vex_blob = archive.read_path(vex_path).expect("reads the vex");
        let model_blob = archive.read_path(model_path).expect("reads the model");
        let nodes = oag_vex::vex::nodes(&vex_blob).unwrap_or_else(|e| panic!("{vex_path}: {e}"));
        let model = psp2::parse(&model_blob).unwrap_or_else(|e| panic!("{model_path}: {e}"));
        assert!(model.has_geometry(), "{model_path}: no geometry");
        out.pairs += 1;
        let group_of = |wanted: &str| {
            nodes.iter().position(|n| {
                n.name
                    .as_deref()
                    .is_some_and(|n| n.eq_ignore_ascii_case(wanted))
            })
        };
        let (main, boost) = (group_of("EF_Main"), group_of("EF_Boost"));
        out.missing_groups += usize::from(main.is_none()) + usize::from(boost.is_none());
        if main.is_none() || boost.is_none() {
            out.group_gaps.push(format!(
                "{vex_path} main={} boost={}",
                main.is_some(),
                boost.is_some()
            ));
        }
        if model
            .materials
            .iter()
            .any(|m| m.name.contains("flame_test"))
        {
            out.flame_test.push(vex_path.clone());
        }
        for mesh in &model.scene.meshes {
            let Some(node) = nodes.iter().position(|n| {
                n.name
                    .as_deref()
                    .is_some_and(|n| n.eq_ignore_ascii_case(&mesh.name))
            }) else {
                out.unmatched_meshes += 1;
                continue;
            };
            let mut at = nodes[node].parent;
            let mut group = None;
            while let Some(i) = at {
                if Some(i) == main {
                    group = Some(true);
                    break;
                }
                if Some(i) == boost {
                    group = Some(false);
                    break;
                }
                at = nodes[i].parent;
            }
            match group {
                Some(true) => out.main_meshes += 1,
                Some(false) => out.boost_meshes += 1,
                None => out.ungrouped_meshes += 1,
            }
        }
        for material in &model.materials {
            out.materials.insert(material.name.to_ascii_lowercase());
            out.textures
                .extend(material.textures.iter().map(|t| t.to_ascii_lowercase()));
        }
    }
    Some(out)
}

const GAP: &str = "data/art/published/hdships/Harimau/Engineflare.vex main=true boost=false";
const ADDITIVE: &str = "data/art/published/ships/materials/2048_engine_additive.rcsmaterial";
const FLAME_TEST: &str = "data/art/published/materials/ships/engines/flame_test.rcsmaterial";

/// `pairs` flare pairs; `flame_test` of them still on HD's own material.
fn check(name: &str, pairs: usize, flame_test: usize) {
    let Some(f) = survey(name) else { return };
    println!(
        "{name}: {} pair(s), {} main / {} boost mesh(es)",
        f.pairs, f.main_meshes, f.boost_meshes
    );
    assert_eq!(f.pairs, pairs, "{name}: flare pairs");
    assert_eq!(
        f.vex_without_sibling, 0,
        "{name}: a flare .vex with no .rcsmodel"
    );
    assert_eq!(
        f.unmatched_meshes, 0,
        "{name}: a mesh object no .vex node names"
    );
    assert_eq!(
        f.ungrouped_meshes, 0,
        "{name}: a mesh object under neither group"
    );
    assert_eq!(f.flame_test.len(), flame_test, "{name}: {:?}", f.flame_test);
    if pairs == 0 {
        assert_eq!(f.missing_groups, 0);
        return;
    }
    // Harimau authors no boost group at all: its boost half draws nothing.
    assert_eq!(f.group_gaps, [GAP], "{name}: trees missing a group");
    assert!(f.main_meshes > 0 && f.boost_meshes > 0, "{name}");
    let mut materials = vec![ADDITIVE];
    if flame_test > 0 {
        materials.insert(0, FLAME_TEST);
    }
    assert_eq!(
        f.materials.iter().map(String::as_str).collect::<Vec<_>>(),
        materials,
        "{name}"
    );
}

#[test]
#[ignore = "needs data/extracted/vita"]
fn eu_base_flares() {
    check(PACKAGES[0], 58, 13);
}

#[test]
#[ignore = "needs data/extracted/vita"]
fn eu_dlc_and_patch_flares() {
    check(PACKAGES[1], 0, 0);
    check(PACKAGES[2], 0, 0);
    check(PACKAGES[3], 0, 0);
    // The patch re-ships 24 hdships flares, every one on 2048's own material.
    check(PACKAGES[4], 24, 0);
}

#[test]
#[ignore = "needs data/extracted/vita"]
fn usa_flares() {
    check(PACKAGES[5], 58, 13);
    check(PACKAGES[6], 0, 0);
    check(PACKAGES[7], 0, 0);
    check(PACKAGES[8], 24, 0);
}

/// Every ribbon template under `data/ribboneffects` decodes, names a material
/// and two textures that are in the same archive; the one the executable names,
/// `enginetrail_bluered_triangle`, is `hd_enginetrail_bluered`.
#[test]
#[ignore = "needs data/extracted/vita"]
fn ribbon_templates() {
    for name in [PACKAGES[0], PACKAGES[5]] {
        let Some(path) = package(name) else { continue };
        let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string()).unwrap();
        let paths: Vec<String> = archive.paths().to_vec();
        let lower: BTreeSet<String> = paths.iter().map(|p| p.to_ascii_lowercase()).collect();
        let templates: Vec<&String> = paths
            .iter()
            .filter(|p| {
                let l = p.to_ascii_lowercase();
                l.starts_with("data/ribboneffects/") && l.ends_with("_triangle.rcsmodel")
            })
            .collect();
        assert_eq!(templates.len(), 8, "{name}: ribbon templates");
        for template in templates {
            let model = psp2::parse(&archive.read_path(template).unwrap())
                .unwrap_or_else(|e| panic!("{template}: {e}"));
            assert!(model.has_geometry(), "{template}");
            let material = model
                .materials
                .first()
                .unwrap_or_else(|| panic!("{template}: no material"));
            assert_eq!(model.materials.len(), 1, "{template}");
            assert!(!material.textures.is_empty(), "{template}");
            for texture in &material.textures {
                assert!(
                    lower.contains(&texture.to_ascii_lowercase()),
                    "{template}: {texture} not in the archive"
                );
            }
            if template
                .eq_ignore_ascii_case("data/ribboneffects/enginetrail_bluered_triangle.rcsmodel")
            {
                assert_eq!(material.textures.len(), 2, "{template}");
                assert_eq!(
                    material.name.to_ascii_lowercase(),
                    "data/ribboneffects/materials/hd_enginetrail_bluered.rcsmaterial"
                );
            }
        }
        assert!(
            lower.contains("data/tex/engineflare/engine_flare_rich.gxt"),
            "{name}: sprite flare"
        );
    }
}

/// The Omega check: how many of each file the PS4 archives ship, and whether
/// the same reader decodes them.
#[test]
#[ignore = "needs data/extracted/ps4"]
fn omega_census() {
    let mut flares = 0;
    let mut decoded = 0;
    let mut ribbons = 0;
    for name in [
        "ps4/omega-eu/uroot/data00.psarc",
        "ps4/omega-eu/uroot/data01.psarc",
        "ps4/omega-eu/uroot/data02.psarc",
        "ps4/omega-eu/uroot/data03.psarc",
        "ps4/omega-eu/uroot/data04.psarc",
        "ps4/omega-eu-patch/uroot/data05.psarc",
        "ps4/omega-eu-patch/uroot/data07.psarc",
        "ps4/omega-eu-patch/uroot/data08.psarc",
        "ps4/omega-eu-patch/uroot/data09.psarc",
    ] {
        let Some(path) = package(name) else { continue };
        let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string()).unwrap();
        let paths: Vec<String> = archive.paths().to_vec();
        for p in &paths {
            let l = p.to_ascii_lowercase();
            if l.ends_with("/engineflare.rcsmodel") {
                flares += 1;
                let blob = archive.read_path(p).unwrap();
                decoded += usize::from(psp2::parse(&blob).is_ok_and(|m| m.has_geometry()));
            }
            ribbons += usize::from(
                l.starts_with("data/ribboneffects/") && l.ends_with("_triangle.rcsmodel"),
            );
        }
    }
    if flares == 0 {
        return;
    }
    // One of Omega's 69 flare models has no geometry the reader finds.
    assert_eq!((flares, decoded, ribbons), (69, 68, 9));
}
