//! Omega's `_shadow` crowds draw lit by their `Shadowed` and `Ambient` (`mesh::rcs::psp2::crowd`).
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! Builds the `environments2048` circuits that show stands and holds that every vertex of a
//! `crowd_*` material that authors `Shadowed` (the `_shadow` twins; the plain ones author neither
//! parameter on the disc and keep the generic reading) carries the glow-table entry of [`PROGRAM_CROWD`] with the
//! material's authored `Ambient` as its tint and `Shadowed` as its second program lane (both
//! read straight from the `.rcsmodel` here, not through the builder). Remove the builder's
//! `crowd::plan` call and every count here reads zero.

use std::path::Path;

use oag_mesh::mesh::rcs::psp2;
use oag_mesh::mesh::{PROGRAM_CROWD, slots};
use oag_rcs::rcsmaterial::name_hash;

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

fn circuit(name: &str, path: &str) {
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
    assert!(report.crowd_materials > 0, "{name}: {}", report.describe());
    assert_eq!(report.crowd_unread, 0, "{name}: {}", report.describe());

    let crowd = |material: &oag_rcs::rcsmodel::psp2::material::Material| {
        material
            .name
            .rsplit(['/', '\\'])
            .next()
            .is_some_and(|f| f.to_ascii_lowercase().starts_with("crowd_"))
            && material.param(name_hash("Shadowed")).is_some()
    };
    let mut authored: Vec<(u32, [u32; 3])> = Vec::new();
    for submesh in &decoded.submeshes {
        let Some(material) = submesh.material.and_then(|m| decoded.materials.get(m)) else {
            continue;
        };
        if !crowd(material) {
            continue;
        }
        let shadowed = material.param(name_hash("Shadowed")).expect("Shadowed")[0];
        let ambient = material.param(name_hash("Ambient")).expect("Ambient");
        authored.push((shadowed.to_bits(), [0, 1, 2].map(|i| ambient[i].to_bits())));
    }
    assert!(
        !authored.is_empty(),
        "{name}: the file holds no crowd submesh"
    );

    let mut entries = 0usize;
    for vertex in &model.vertices {
        let index = slots::material_index(vertex.slots) as usize;
        if index == 0 || model.emissive[index - 1].program[0] != PROGRAM_CROWD {
            continue;
        }
        let entry = &model.emissive[index - 1];
        let carried = (entry.program[1].to_bits(), entry.tint.map(f32::to_bits));
        assert!(
            authored.contains(&carried),
            "{name}: entry {entry:?} is not a crowd material's Shadowed and Ambient"
        );
        entries += 1;
    }
    assert!(entries > 0, "{name}: no vertex names the crowd entry");
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn sols_stands_are_lit_by_shadowed_and_ambient() {
    circuit("sol", "data/environments2048/sol/track.final.rcsmodel");
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn queens_malls_stands_are_lit_by_shadowed_and_ambient() {
    circuit("mall", "data/environments2048/mall/track.final.rcsmodel");
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn rockway_stadiums_stands_are_lit_by_shadowed_and_ambient() {
    circuit("arena", "data/environments2048/arena/track.final.rcsmodel");
}
