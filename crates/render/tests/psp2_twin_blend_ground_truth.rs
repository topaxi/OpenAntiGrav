//! HD's `nr_twinblend`, inherited by 2048's and Omega's circuits that re-ship it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! Both titles ship HD's material (same four sampler hashes, same `barsUV_blendUV` vertex
//! attribute) and their programs are GXP and GCN, so the draw is HD's law, inherited
//! (`oag_mesh::mesh::rcs::psp2`, `twin_blend`). This walks every model of the three 2048
//! packages and Omega's base and patch archives that names the material and holds, per
//! circuit model, that every such material reaches the mode with its four pictures and its
//! three coordinate pairs. Remove the builder's `twin_blend::plan` call and every count here
//! reads zero.

use std::path::Path;

use oag_mesh::mesh::rcs::psp2;
use oag_mesh::mesh::{Model, PROGRAM_TWIN_BLEND, slots};

const VITA: [&str; 3] = [
    "vita/PCSF00007/base/PSP2/data.psarc",
    "vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];
const OMEGA: [&str; 9] = [
    "ps4/omega-eu/uroot/data00.psarc",
    "ps4/omega-eu/uroot/data01.psarc",
    "ps4/omega-eu/uroot/data02.psarc",
    "ps4/omega-eu/uroot/data03.psarc",
    "ps4/omega-eu/uroot/data04.psarc",
    "ps4/omega-eu-patch/uroot/data05.psarc",
    "ps4/omega-eu-patch/uroot/data07.psarc",
    "ps4/omega-eu-patch/uroot/data08.psarc",
    "ps4/omega-eu-patch/uroot/data09.psarc",
];

/// `(model path, nr_twinblend materials)` per package, the census of the sweep in
/// `docs/formats/rcsmaterial.md`.
const VITA_CENSUS: &[(&str, &str, usize)] = &[
    (VITA[0], "DLC1/environments/Anulpha_Pass/track.rcsmodel", 1),
    (
        VITA[0],
        "DLC1/environments/Anulpha_Pass/track_reversed.rcsmodel",
        1,
    ),
    (
        VITA[0],
        "DLC1/environments/Chenghou_Project/track.rcsmodel",
        2,
    ),
    (
        VITA[0],
        "DLC1/environments/Chenghou_Project/track_reversed.rcsmodel",
        2,
    ),
    (VITA[1], "DLC1/environments/Anulpha_Pass/track.rcsmodel", 1),
    (
        VITA[1],
        "DLC1/environments/Anulpha_Pass/track_reversed.rcsmodel",
        1,
    ),
    (
        VITA[1],
        "DLC1/environments/Chenghou_Project/track.rcsmodel",
        2,
    ),
    (
        VITA[1],
        "DLC1/environments/Chenghou_Project/track_reversed.rcsmodel",
        2,
    ),
    (VITA[1], "DLC1/environments/Metropia/track.rcsmodel", 1),
    (
        VITA[1],
        "DLC1/environments/Metropia/track_reversed.rcsmodel",
        1,
    ),
    (VITA[1], "DLC1/environments/Sebenco_Climb/track.rcsmodel", 2),
    (
        VITA[1],
        "DLC1/environments/Sebenco_Climb/track_reversed.rcsmodel",
        2,
    ),
    (VITA[2], "DLC1/environments/tech_de_ra/track.rcsmodel", 1),
    (
        VITA[2],
        "DLC1/environments/tech_de_ra/track_reversed.rcsmodel",
        1,
    ),
];
const OMEGA_CENSUS: &[(&str, usize)] = &[
    ("Data/environments/15_anulpha_pass/track.final.rcsmodel", 1),
    (
        "Data/environments/15_anulpha_pass/track_reversed.final.rcsmodel",
        1,
    ),
    (
        "Data/environments/04_chenghou_project/track.final.rcsmodel",
        2,
    ),
    (
        "Data/environments/04_chenghou_project/track_reversed.final.rcsmodel",
        2,
    ),
    ("Data/environments/02_metropia/track.final.rcsmodel", 1),
    (
        "Data/environments/02_metropia/track_reversed.final.rcsmodel",
        1,
    ),
    ("Data/environments/tech_de_ra/track.final.rcsmodel", 1),
    (
        "Data/environments/tech_de_ra/track_reversed.final.rcsmodel",
        1,
    ),
    ("Data/environments/10_sebenco_climb/track.final.rcsmodel", 2),
    (
        "Data/environments/10_sebenco_climb/track_reversed.final.rcsmodel",
        2,
    ),
];

fn root() -> Option<std::path::PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
    if root.join("vita").exists() && root.join("ps4").exists() {
        return Some(root);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is incomplete",
        root.display()
    );
    println!("skipping: 2048 or Omega not extracted");
    None
}

/// What one model's twin-blend draws look like: the distinct bound slots with their pictures.
struct Found {
    slots: usize,
    labels: Vec<[String; 4]>,
    pairs_differ: bool,
    unlit: bool,
}

fn found(model: &Model) -> Found {
    let label = |slots: &oag_mesh::mesh::TextureSlots, i: usize| {
        slots
            .get(i)
            .and_then(Option::as_ref)
            .map_or_else(String::new, |t| t.label.to_ascii_lowercase())
    };
    let mut out = Found {
        slots: 0,
        labels: Vec::new(),
        pairs_differ: false,
        unlit: true,
    };
    let mut seen = Vec::new();
    for draw in model
        .draws
        .iter()
        .chain(&model.transparent_draws)
        .chain(&model.alpha_tested_draws)
    {
        let Some(texture) = draw.texture else {
            continue;
        };
        let range = draw.range.start as usize..draw.range.end as usize;
        let vertices: Vec<_> = model.indices[range]
            .iter()
            .map(|&i| model.vertices[i as usize])
            .collect();
        let twin = vertices.iter().any(|v| {
            let index = slots::material_index(v.slots) as usize;
            index != 0 && model.emissive[index - 1].program[0] == PROGRAM_TWIN_BLEND
        });
        if !twin || seen.contains(&texture) {
            continue;
        }
        seen.push(texture);
        out.slots += 1;
        out.unlit &= vertices.iter().all(|v| {
            v.slots & (slots::NO_AMBIENT | slots::NO_SUN) == (slots::NO_AMBIENT | slots::NO_SUN)
                && v.slots & (slots::ADD_SECOND | slots::SECOND_IS_LIGHTMAP) == 0
        });
        out.pairs_differ |= vertices.iter().any(|v| v.lightmap_texcoord != v.texcoord)
            && vertices.iter().any(|v| v.texcoord2 != v.lightmap_texcoord);
        let rate = model.emissive[slots::material_index(vertices[0].slots) as usize - 1].tint[0];
        assert_eq!(rate, psp2::TWIN_BLEND_RATE);
        out.labels.push([
            label(&model.textures, texture),
            label(&model.lightmaps, texture),
            label(&model.pad_masks, texture),
            label(&model.wave_maps, texture),
        ]);
    }
    out
}

fn check(name: &str, expected: usize, model: &Model, report: &psp2::Report) {
    let f = found(model);
    assert_eq!(
        report.twin_blend_materials,
        expected,
        "{name}: {}",
        report.describe()
    );
    assert_eq!(report.twin_blend_unread, 0, "{name}: {}", report.describe());
    assert!(
        f.slots >= expected,
        "{name}: only {} twin-blend slot(s) drawn",
        f.slots
    );
    assert!(
        f.unlit,
        "{name}: a twin-blend vertex is lit or keeps the additive second"
    );
    // A circuit may author the three pairs identical on some of its slots (HD's Chenghou does);
    // Tech de Ra authors them apart on both titles, and a dropped pair read would show there.
    assert!(
        f.pairs_differ || !name.contains("tech_de_ra"),
        "{name}: the wipe's and the bars' pairs equal the picture's"
    );
    for [first, second, wipe, bars] in &f.labels {
        assert!(
            !first.is_empty() && !second.is_empty(),
            "{name}: pictures {first} {second}"
        );
        assert!(wipe.contains("animscaledballs"), "{name}: wipe is {wipe}");
        assert!(bars.contains("glowbars_noisy"), "{name}: bars are {bars}");
    }
}

fn open(root: &Path, names: &[&str]) -> Vec<oag_assets::psarc::Archive> {
    names
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect()
}

/// Builds `path` out of `archives[index]`, every texture resolved through all of them.
fn build(
    archives: &mut [oag_assets::psarc::Archive],
    index: usize,
    path: &str,
) -> (Model, psp2::Report) {
    let blob = archives[index].read_path(path).expect("the model");
    psp2::build(path, &blob, None, &mut |p| {
        archives.iter_mut().find_map(|a| a.read_path(p).ok())
    })
    .expect("builds")
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007 and data/extracted/ps4/omega-eu"]
fn every_2048_and_omega_circuit_that_ships_nr_twinblend_draws_it_by_hds_law() {
    let Some(root) = root() else { return };
    let mut vita = open(&root, &VITA);
    let mut omega = open(&root, &OMEGA);
    assert_eq!((vita.len(), omega.len()), (VITA.len(), OMEGA.len()));
    for (package, path, expected) in VITA_CENSUS {
        let index = VITA.iter().position(|v| v == package).expect("listed");
        let full = format!("data/art/published/{path}");
        let (model, report) = build(&mut vita, index, &full);
        check(&format!("{package} {path}"), *expected, &model, &report);
    }
    for (path, expected) in OMEGA_CENSUS {
        let index = (0..omega.len())
            .find(|&i| omega[i].contains(path))
            .unwrap_or_else(|| panic!("{path} is in no archive"));
        let (model, report) = build(&mut omega, index, path);
        check(path, *expected, &model, &report);
    }
}
