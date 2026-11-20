//! The check that Omega's crowds draw lit by their `Shadowed` and `Ambient` and with the eye
//! frame (`mesh::rcs::psp2::crowd`), run over circuits other disc-backed tests already build
//! (`omega_tarmac_markings_ground_truth.rs`, `omega_panel_map_ground_truth.rs`).
//!
//! Holds that every vertex of a crowd material (the two plain families and their `_shadow`
//! twins; the skinned `_bonecount4` twins the bind-pose rigs add are counted by the report) carries the glow-table entry of [`PROGRAM_CROWD`]
//! with the material's authored `Ambient` as its tint and `Shadowed` as its second program lane
//! (both read straight from the `.rcsmodel` here, not through the builder; the plain ones author
//! neither and read `0` and `1`), and `offset` `1`, the eye frame their programs read; and that
//! the count of such vertices is the count the file holds, so no other plan has ORed a second
//! slot into a crowd vertex. Remove the builder's `crowd::plan` call, or the eye frame, and
//! every count here reads zero or fails.
//!
//! Also holds that the track's crowd-entity shapes (the mesh objects carrying
//! `CROWD_ENTITY_FLAGS`) draw nothing and each is replaced by one bind-pose rig: the report
//! counts exactly the file's submeshes of those shapes and the rigs placed, and the model carries
//! the shapes' vertices fewer and the rigs' more than the file holds (nothing else lost where the
//! file hides no node). Drop the builder's crowd-entity hiding and the count reads zero.

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
        let file = material.name.rsplit(['/', '\\']).next().unwrap_or("");
        let stem = file.to_ascii_lowercase();
        let stem = stem.trim_end_matches(".rcsmaterial");
        [
            "crowd_sprite_diffuse_normal_alpha",
            "crowd_sprite_diffuse_normal_alpha_shadow",
            "crowd_diffuse_normal",
            "crowd_diffuse_normal_shadow",
        ]
        .contains(&stem)
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
        let shadowed = material.param(name_hash("Shadowed")).map_or(1.0, |p| p[0]);
        let ambient = material
            .param(name_hash("Ambient"))
            .map_or([0.0; 3], |p| [0, 1, 2].map(|i| p[i]));
        authored.push((shadowed.to_bits(), ambient.map(f32::to_bits)));
        expected += submesh.positions.len();
    }
    assert!(
        !authored.is_empty(),
        "{name}: the file holds no crowd submesh"
    );

    let mut entries = 0usize;
    let mut rig_entries = 0usize;
    for vertex in &model.vertices {
        let index = slots::material_index(vertex.slots) as usize;
        if index == 0 || model.emissive[index - 1].program[0] != PROGRAM_CROWD {
            continue;
        }
        let entry = &model.emissive[index - 1];
        assert_eq!(entry.offset, 1.0, "{name}: {entry:?} lacks the eye frame");
        let carried = (entry.program[1].to_bits(), entry.tint.map(f32::to_bits));
        // A rig's skinned twin is not in the file; its vertices are the report's.
        if !authored.contains(&carried) {
            rig_entries += 1;
        }
        entries += 1;
    }
    // A rig on a hidden node draws nothing, as the placeholder it replaces does not.
    let rigs = report.crowd_rig_vertices;
    assert!(
        rig_entries <= rigs,
        "{name}: {rig_entries} crowd vertices carry a Shadowed and Ambient no file material \
         authors, beyond the rigs' {rigs}"
    );
    if report.hidden + report.unplaced == 0 {
        assert_eq!(
            entries,
            expected + rigs,
            "{name}: every vertex of a crowd submesh or a bind-pose rig names the crowd entry, and no other does"
        );
    } else {
        assert!(
            (expected..=expected + rigs).contains(&entries),
            "{name}: {entries} crowd vertices of {expected} + {rigs}"
        );
    }
}

/// The submeshes of the file's crowd-entity shapes, and how many vertices they hold.
pub fn crowd_entity_submeshes(decoded: &oag_rcs::rcsmodel::psp2::Model) -> (usize, usize) {
    decoded
        .submeshes
        .iter()
        .filter(|s| {
            s.mesh
                .and_then(|m| decoded.scene.meshes.get(m))
                .is_some_and(|m| m.is_crowd_entity())
        })
        .fold((0, 0), |(n, v), s| (n + 1, v + s.positions.len()))
}

/// What replacing the crowd-entity shapes did to `model`: `report` counts the file's submeshes of
/// them, `model` lost their vertices, gained the rigs' and nothing else.
pub fn check_crowd_entities_hidden(
    name: &str,
    decoded: &oag_rcs::rcsmodel::psp2::Model,
    model: &Model,
    report: &psp2::Report,
) {
    let (submeshes, vertices) = crowd_entity_submeshes(decoded);
    assert!(
        submeshes > 0,
        "{name}: the file holds no crowd-entity shape"
    );
    assert_eq!(report.crowd_entities, submeshes, "{name}");
    assert_eq!(
        (report.crowd_rigs, report.crowd_rigs_missing),
        (submeshes, 0),
        "{name}: a bind-pose rig stands at every crowd entity"
    );
    let file: usize = decoded.submeshes.iter().map(|s| s.positions.len()).sum();
    let emitted = (model.vertices.len() + vertices).saturating_sub(report.crowd_rig_vertices);
    if report.hidden + report.unplaced == 0 {
        assert_eq!(
            emitted, file,
            "{name}: the model holds the file's vertices less the crowd entities'"
        );
    } else {
        assert!(emitted <= file, "{name}: {emitted} of {file}");
    }
}
