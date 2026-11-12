//! Omega's `tarmac_markings_dirt_markingsnormal[_emis]` roads draw their dirt and markings
//! layers (`mesh::rcs::psp2::tarmac`).
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! Builds two `environments2048` circuits and holds, for every vertex of a road material, that
//! it carries the glow-table entry of [`PROGRAM_TARMAC`], that `texcoord2` is the file's
//! `uvMarkings` and `colour.xy` the dirt pair of `uv1_uvDirt` (both read straight from the
//! `.rcsmodel` here, not through the builder), and that the draw's fifth binding is the decal
//! atlas and its sixth the dirt splat. Remove the builder's `tarmac::plan` call and every count
//! here reads zero.

use std::path::Path;

use oag_mesh::mesh::rcs::psp2;
use oag_mesh::mesh::{Model, PROGRAM_TARMAC, slots};

const ARCHIVES: [&str; 9] = [
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

fn open() -> Option<Vec<oag_assets::psarc::Archive>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
    if !root.join("ps4").exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} has no ps4/",
            root.display()
        );
        println!("skipping: Omega not extracted");
        return None;
    }
    Some(
        ARCHIVES
            .iter()
            .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
            .collect(),
    )
}

/// What a circuit's road vertices look like: the vertices found, and the draws' pictures.
struct Roads {
    vertices: usize,
    markings: Vec<String>,
    dirt: Vec<String>,
}

fn roads(model: &Model, decoded: &oag_rcs::rcsmodel::psp2::Model) -> Roads {
    let mut out = Roads {
        vertices: 0,
        markings: Vec::new(),
        dirt: Vec::new(),
    };
    let label = |slots: &oag_mesh::mesh::TextureSlots, i: usize| {
        slots
            .get(i)
            .and_then(Option::as_ref)
            .map(|t| t.label.to_ascii_lowercase())
    };
    // Every opaque draw is one submesh, in order; the vertex ranges follow the index ranges.
    let drawn = model
        .draws
        .iter()
        .chain(&model.transparent_draws)
        .chain(&model.alpha_tested_draws);
    for draw in drawn {
        let Some(texture) = draw.texture else {
            continue;
        };
        let range = draw.range.start as usize..draw.range.end as usize;
        let tarmac = model.indices[range.clone()].iter().any(|&i| {
            let index = slots::material_index(model.vertices[i as usize].slots) as usize;
            index != 0 && model.emissive[index - 1].program[0] == PROGRAM_TARMAC
        });
        if !tarmac {
            continue;
        }
        out.markings.extend(label(&model.fifth_maps, texture));
        out.dirt.extend(label(&model.sixth_maps, texture));
        out.vertices += model.indices[range]
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len();
    }
    // The file's truth: every road vertex the model carries holds a (decal, dirt) pair that some
    // submesh of the file holds at the same vertex. A hidden submesh pushes no vertices, so the
    // two sides are compared as sets.
    let bits = |a: [f32; 2], b: [f32; 2]| {
        [
            a[0].to_bits(),
            a[1].to_bits(),
            b[0].to_bits(),
            b[1].to_bits(),
        ]
    };
    let file: std::collections::BTreeSet<[u32; 4]> = decoded
        .submeshes
        .iter()
        .flat_map(|s| s.markings_texcoords.iter().zip(&s.dirt_texcoords))
        .map(|(m, d)| bits(*m, *d))
        .collect();
    let mut carried = 0usize;
    for vertex in &model.vertices {
        let index = slots::material_index(vertex.slots) as usize;
        if index == 0 || model.emissive[index - 1].program[0] != PROGRAM_TARMAC {
            continue;
        }
        assert!(
            file.contains(&bits(
                vertex.texcoord2,
                [vertex.colour[0], vertex.colour[1]]
            )),
            "a road vertex's pairs are not the file's"
        );
        assert_eq!([vertex.colour[2], vertex.colour[3]], [1.0, 1.0]);
        carried += 1;
    }
    out.vertices = carried;
    out
}

fn circuit(name: &str, path: &str, expect_road_vertices: usize) {
    let Some(mut archives) = open() else { return };
    let index = (0..archives.len())
        .find(|&i| archives[i].contains(path))
        .unwrap_or_else(|| panic!("{path} is in no archive"));
    let blob = archives[index].read_path(path).expect("the model");
    let decoded = oag_rcs::rcsmodel::psp2::parse(&blob).expect("parses");
    let (model, report) = psp2::build(path, &blob, None, &mut |p| {
        archives.iter_mut().find_map(|a| a.read_path(p).ok())
    })
    .expect("builds");
    assert!(report.tarmac_materials > 0, "{name}: {}", report.describe());
    assert_eq!(report.tarmac_unread, 0, "{name}: {}", report.describe());
    let found = roads(&model, &decoded);
    assert!(
        found.vertices >= expect_road_vertices,
        "{name}: {} road vertices carry the pairs",
        found.vertices
    );
    assert!(!found.markings.is_empty() && !found.dirt.is_empty());
    for label in &found.markings {
        assert!(
            label.contains("road_a_decals_d_a"),
            "{name}: markings {label}"
        );
    }
    for label in &found.dirt {
        assert!(label.contains("road_a_ds_splat"), "{name}: dirt {label}");
    }
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn rockway_stadiums_road_carries_its_markings_and_dirt() {
    circuit(
        "arena",
        "data/environments2048/arena/track.final.rcsmodel",
        1000,
    );
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn queens_malls_road_carries_its_markings_and_dirt() {
    circuit(
        "mall",
        "data/environments2048/mall/track.final.rcsmodel",
        100,
    );
}
