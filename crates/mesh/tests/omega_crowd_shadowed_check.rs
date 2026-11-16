//! The check that Omega's `_shadow` crowds draw lit by their `Shadowed` and `Ambient`
//! (`mesh::rcs::psp2::crowd`), run over circuits other disc-backed tests already build
//! (`omega_tarmac_markings_ground_truth.rs`, `omega_panel_map_ground_truth.rs`).
//!
//! Holds that every vertex of a `crowd_*` material that authors `Shadowed` (the `_shadow` twins; the
//! plain ones author neither parameter on the disc and keep the generic reading) carries the
//! glow-table entry of [`PROGRAM_CROWD`] with the material's authored `Ambient` as its tint and
//! `Shadowed` as its second program lane (both read straight from the `.rcsmodel` here, not
//! through the builder), and that the count of such vertices is the count the file holds, so no
//! other plan has ORed a second slot into a crowd vertex. Remove the builder's `crowd::plan` call
//! and every count here reads zero.

use oag_mesh::mesh::rcs::psp2;
use oag_mesh::mesh::{Model, PROGRAM_CROWD, slots};
use oag_rcs::rcsmaterial::name_hash;

pub fn check(
    name: &str,
    decoded: &oag_rcs::rcsmodel::psp2::Model,
    model: &Model,
    report: &psp2::Report,
) {
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
    let mut expected = 0usize;
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
        expected += submesh.positions.len();
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
    assert_eq!(
        entries, expected,
        "{name}: every vertex of a crowd submesh names the crowd entry, and no other does"
    );
}
