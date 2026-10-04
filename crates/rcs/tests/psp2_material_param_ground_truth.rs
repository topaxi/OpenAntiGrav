//! Wipeout 2048's per-material uniform table against the shaders it feeds.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run it with `just test-data`.
//!
//! A material record in a Vita `.rcsmodel` carries a table of named uniform
//! values (`oag_rcs::rcsmodel::psp2::material::Param`). Three independent
//! checks that the reading is the table and not a coincidence:
//!
//! - **every uniform hash is a name the material's own shader declares.** The
//!   GXP programs in the `.rcsmaterial` name their uniforms in the clear, and
//!   `~crc32` of one of those names is the entry's hash. A wrong stride or a
//!   wrong offset reads garbage as hashes and fails this at once.
//! - **the Zone colours agree with the materials' own names**: `C_Red` reads
//!   `1 0 0`, `C_Pink` `1 0 1`.
//! - **every name `KNOWN_PSP2_PARAMETER_NAMES` carries is declared by some
//!   shader**, so the table holds preimages and not guesses.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use oag_rcs::gxp;
use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2;

const PACKAGES: [&str; 3] = [
    "vita/PCSF00007/base/PSP2/data.psarc",
    "vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
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

fn open(path: &Path) -> oag_assets::psarc::Archive {
    oag_assets::psarc::Archive::open(path.to_str().expect("utf-8 path")).expect("the package opens")
}

/// Every `.rcsmaterial` in the three packages, lower-cased path to the hashes
/// of every uniform name its programs declare.
fn declared() -> BTreeMap<String, BTreeSet<u32>> {
    let mut out: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    for name in PACKAGES {
        let Some(path) = package(name) else { continue };
        let mut archive = open(&path);
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmaterial"))
            .cloned()
            .collect();
        for entry in paths {
            let blob = archive.read_path(&entry).expect("reads");
            let names = out.entry(entry.to_ascii_lowercase()).or_default();
            for (_, program) in gxp::programs(&blob) {
                if let Ok(program) = program {
                    names.extend(program.parameters.iter().map(|p| name_hash(&p.name)));
                }
            }
        }
    }
    out
}

fn models() -> Vec<(String, Vec<psp2::material::Material>)> {
    let mut out = Vec::new();
    for name in PACKAGES {
        let Some(path) = package(name) else { continue };
        let mut archive = open(&path);
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for entry in paths {
            let blob = archive.read_path(&entry).expect("reads");
            if let Ok(model) = psp2::parse(&blob) {
                out.push((entry, model.materials));
            }
        }
    }
    out
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn every_uniform_hash_is_a_name_the_materials_own_shader_declares() {
    let declared = declared();
    if declared.is_empty() {
        return;
    }
    let (mut checked, mut missed, mut unknown_file) = (0usize, Vec::new(), 0usize);
    let mut with_params = 0usize;
    for (entry, materials) in models() {
        for material in &materials {
            if material.params.is_empty() {
                continue;
            }
            with_params += 1;
            let Some(names) = declared.get(&material.name.to_ascii_lowercase()) else {
                unknown_file += 1;
                continue;
            };
            for param in &material.params {
                checked += 1;
                if !names.contains(&param.hash) {
                    missed.push(format!("{entry}: {} {:08x}", material.name, param.hash));
                }
            }
        }
    }
    println!(
        "{with_params} material(s) with uniforms ({unknown_file} name no .rcsmaterial in the \
         packages), {checked} uniform(s) checked, {} unnamed",
        missed.len()
    );
    assert!(
        checked > 15_000,
        "the sweep reached only {checked} uniforms"
    );
    // **One uniform on one material is the whole of the exception**: Tech de
    // Ra's `tech_de_ra_cloud_effect` authors `981fc3f4`, which none of its
    // programs names - on 4 of 15,565 uniforms, in the two directions of one
    // circuit (`track` and `track_reversed`), twice each. Anything else, or a
    // fifth, is a wrong reading.
    let stray: Vec<&String> = missed
        .iter()
        .filter(|m| !(m.contains("tech_de_ra_cloud_effect") && m.ends_with("981fc3f4")))
        .collect();
    assert!(
        stray.is_empty(),
        "uniform hash(es) its shader does not declare: {stray:?}"
    );
    assert!(missed.len() <= 4, "{} unnamed uniforms", missed.len());
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_zone_materials_colour_agrees_with_its_own_name() {
    let Some(path) = package(PACKAGES[0]) else {
        return;
    };
    let blob = open(&path)
        .read_path("data/art/published/environments/altima/trackZone.rcsmodel")
        .expect("trackZone.rcsmodel");
    let model = psp2::parse(&blob).expect("parses");
    let named = |tag: &str| {
        model
            .materials
            .iter()
            .filter(|m| m.technique.as_deref().is_some_and(|t| t.ends_with(tag)))
            .filter_map(|m| m.zone_colour())
            .collect::<Vec<_>>()
    };
    let red = named("C_RedSG");
    let pink = named("C_PinkSG");
    assert!(
        !red.is_empty() && !pink.is_empty(),
        "the oracle materials were not found"
    );
    assert!(red.iter().all(|c| *c == [1.0, 0.0, 0.0]), "{red:?}");
    assert!(pink.iter().all(|c| *c == [1.0, 0.0, 1.0]), "{pink:?}");
    let distinct: BTreeSet<[u32; 3]> = model
        .materials
        .iter()
        .filter_map(|m| m.zone_colour())
        .map(|c| c.map(f32::to_bits))
        .collect();
    assert!(
        distinct.len() >= 8,
        "only {} distinct zone colours",
        distinct.len()
    );
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn every_known_psp2_parameter_name_is_declared_by_a_shader() {
    let declared = declared();
    if declared.is_empty() {
        return;
    }
    let all: BTreeSet<u32> = declared.values().flatten().copied().collect();
    let missing: Vec<&str> = oag_rcs::rcsmaterial::names::KNOWN_PSP2_PARAMETER_NAMES
        .iter()
        .copied()
        .filter(|n| !all.contains(&name_hash(n)))
        .collect();
    assert!(missing.is_empty(), "no shader declares {missing:?}");
}
