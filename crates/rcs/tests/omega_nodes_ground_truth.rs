//! Validates the PS4 node table - [`oag_rcs::rcsmodel::psp2::nodes`] read in
//! its 64-bit layout - against every `.rcsmodel` in Wipeout: Omega
//! Collection's five base archives.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! The table's header pointers were read off `ag_systems\ship.rcsmodel` by
//! hand, and a census that says every submesh reaches a mesh object only
//! validates the mesh table, the submesh list and the record offset: swap the
//! node-hash array with the node-id array and it still passes. So each pointer
//! gets an invariant of its own, over every file, that a wrong offset cannot
//! keep:
//!
//! - **mesh name pointer and hash**: every mesh object's stored name hash is
//!   the `~crc32` of the string its name pointer reaches - one in 2^32 per mesh
//!   to survive by chance, and it holds on all 124,709 meshes;
//! - **node-hash array and mesh-to-node index**: a shape `X:ThingShape` bound to
//!   node `n` has `~crc32("X:Thing")` as node `n`'s name hash on well over half
//!   of the bound meshes, which is the Vita's own rate (54 %) - the other
//!   half are shapes named otherwise, not misplaced;
//! - **bind matrices**: every written matrix is affine (`0 0 0 1` in the last
//!   column), 7,678 of 7,678 on `data00` alone, and the written ones are
//!   exactly the first `+0x12` nodes - a wrong `+0x28` lands on floats that
//!   are not affine;
//! - **node-id array**: it appears verbatim, in order, in the sibling
//!   `.rcsskeleton`, which is how the Vita's table was found in the first
//!   place;
//! - **the record walk**: every submesh record is reachable from exactly one
//!   mesh object, every submesh links back to it, and the records the mesh
//!   objects list beyond the submeshes the reader found are exactly the ones
//!   whose buffer pointers went unpaired (two pointers, one record).
//!
//! One archive per test, so the sweep parallelises across tests.

use std::path::{Path, PathBuf};

use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2;

fn package(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/ps4/omega-eu/uroot")
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

/// What one archive's sweep found.
#[derive(Default, Debug)]
struct Survey {
    files_with_a_table: usize,
    meshes: usize,
    node_bound_meshes: usize,
    /// Bound meshes whose `<name minus "Shape">` hashes to their node's hash.
    node_hashes_matched: usize,
    nodes: usize,
    binds_written: usize,
    skeleton_files: usize,
    /// Skeleton siblings that carry the model's id array verbatim.
    ids_in_skeleton: usize,
}

fn sweep(name: &str) -> Option<Survey> {
    let path = package(name)?;
    let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
    let models: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
        .cloned()
        .collect();
    let mut survey = Survey::default();
    for model_path in models {
        let blob = archive.read_path(&model_path).expect("the model reads");
        let model = psp2::parse(&blob).unwrap_or_else(|e| panic!("{model_path}: {e}"));
        let scene = &model.scene;
        if scene.nodes.is_empty() && scene.meshes.is_empty() {
            continue;
        }
        survey.files_with_a_table += 1;

        for mesh in &scene.meshes {
            survey.meshes += 1;
            assert_eq!(
                mesh.name_hash,
                name_hash(&mesh.name),
                "{model_path}: {} carries a name hash that is not its own crc32",
                mesh.name
            );
            let Some(node) = mesh.node else { continue };
            survey.node_bound_meshes += 1;
            let stem = mesh.name.strip_suffix("Shape").unwrap_or(&mesh.name);
            survey.node_hashes_matched +=
                usize::from(name_hash(stem) == scene.nodes[node].name_hash);
        }

        // Geometry-less files (no GPU section) list mesh objects with no
        // records; the walk closes only where there is geometry to close on.
        if !model.submeshes.is_empty() {
            let reachable: usize = scene.meshes.iter().map(|m| m.submesh_records.len()).sum();
            // A record whose two buffer pointers the relocation search could
            // not pair is a submesh the reader has no geometry for, but its
            // mesh object still lists it: two unpaired pointers, one record.
            assert_eq!(
                reachable,
                model.submeshes.len() + model.unpaired_pointers / 2,
                "{model_path}: every record is reachable from one mesh object"
            );
            assert!(
                model.submeshes.iter().all(|s| s.mesh.is_some()),
                "{model_path}: every submesh links back to its mesh object"
            );
        }

        // The header's own written count.
        let cpu = model.sections[0];
        let cpu = &blob[cpu.at..cpu.at + cpu.len];
        let written = usize::from(u16::from_le_bytes([cpu[0x12], cpu[0x13]]));
        for (i, node) in scene.nodes.iter().enumerate() {
            survey.nodes += 1;
            match node.bind {
                Some(m) => {
                    survey.binds_written += 1;
                    assert!(
                        i < written,
                        "{model_path}: node {i} has a bind past the count"
                    );
                    assert!(
                        m[3] == 0.0 && m[7] == 0.0 && m[11] == 0.0 && m[15] == 1.0,
                        "{model_path}: node {i}'s bind is not affine: {m:?}"
                    );
                }
                // Past the count the array simply ends: what follows is other
                // data (denormal-float noise on a `trackZone`, on the Vita as
                // well), so these nodes have no matrix and are not read.
                None => assert!(i >= written, "{model_path}: node {i} lost its written bind"),
            }
        }

        let skeleton_path = model_path.replace(".rcsmodel", ".rcsskeleton");
        if let Ok(skeleton) = archive.read_path(&skeleton_path) {
            survey.skeleton_files += 1;
            let ids: Vec<u8> = scene
                .nodes
                .iter()
                .flat_map(|n| n.id.to_le_bytes())
                .collect();
            if !ids.is_empty() && skeleton.windows(ids.len()).any(|w| w == ids.as_slice()) {
                survey.ids_in_skeleton += 1;
            }
        }
    }
    println!("{name}: {survey:#?}");
    Some(survey)
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn data00_node_tables_close() {
    let Some(s) = sweep("data00.psarc") else {
        return;
    };
    assert!(s.files_with_a_table >= 620, "{}", s.files_with_a_table);
    assert!(s.meshes >= 18_000, "{}", s.meshes);
    assert!(s.nodes >= 9_000, "{}", s.nodes);
    // 4,972 of 7,557 bound meshes: 66 %, against the Vita's 54 %.
    assert!(s.node_hashes_matched >= 4_900, "{}", s.node_hashes_matched);
    assert!(s.ids_in_skeleton >= 45, "{}", s.ids_in_skeleton);
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn data01_node_tables_close() {
    let Some(s) = sweep("data01.psarc") else {
        return;
    };
    assert!(s.files_with_a_table >= 309, "{}", s.files_with_a_table);
    assert!(s.meshes >= 18_000, "{}", s.meshes);
    assert!(s.nodes >= 8_800, "{}", s.nodes);
    // Every node of this archive carries a written matrix.
    assert_eq!(s.binds_written, s.nodes);
    assert!(s.node_hashes_matched >= 4_200, "{}", s.node_hashes_matched);
}

/// The archive that holds `tech_de_ra`, the circuit a race starts on.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn data02_node_tables_close() {
    let Some(s) = sweep("data02.psarc") else {
        return;
    };
    assert!(s.files_with_a_table >= 32, "{}", s.files_with_a_table);
    assert!(s.meshes >= 36_000, "{}", s.meshes);
    assert_eq!(s.binds_written, s.nodes);
    assert!(s.node_hashes_matched >= 4_200, "{}", s.node_hashes_matched);
    assert!(s.ids_in_skeleton >= 2, "{}", s.ids_in_skeleton);
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn data03_node_tables_close() {
    let Some(s) = sweep("data03.psarc") else {
        return;
    };
    assert!(s.files_with_a_table >= 144, "{}", s.files_with_a_table);
    assert_eq!(s.binds_written, s.nodes);
    // 604 of 732: these are the craft, whose shapes are named for their nodes.
    assert!(s.node_hashes_matched >= 600, "{}", s.node_hashes_matched);
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn data04_node_tables_close() {
    let Some(s) = sweep("data04.psarc") else {
        return;
    };
    assert!(s.files_with_a_table >= 40, "{}", s.files_with_a_table);
    assert!(s.meshes >= 51_000, "{}", s.meshes);
    assert!(s.node_hashes_matched >= 10_200, "{}", s.node_hashes_matched);
    assert!(s.ids_in_skeleton >= 14, "{}", s.ids_in_skeleton);
}

/// `tech_de_ra`'s circuit, in numbers: what a race is drawn from. Every node
/// carries a written matrix, so every node-bound submesh can be placed by the
/// model alone - no skeleton is needed, and none is readable on PS4.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn tech_de_ras_circuit_places_every_bound_submesh_by_its_own_matrices() {
    let Some(path) = package("data02.psarc") else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
    let blob = archive
        .read_path("Data/environments/tech_de_ra/track.final.rcsmodel")
        .expect("the circuit reads");
    let model = psp2::parse(&blob).expect("the circuit decodes");
    assert_eq!(model.scene.nodes.len(), 1_702);
    assert!(model.scene.nodes.iter().all(|n| n.bind.is_some()));
    assert_eq!(model.scene.meshes.len(), 2_659);
    assert_eq!(model.submeshes.len(), 3_186);
    let bound = model.submeshes.iter().filter(|s| s.node.is_some()).count();
    assert_eq!(bound, 1_684);
    // The first bound mesh is a crowd figure: node-space geometry that a bind
    // matrix, not the file's own vertices, puts in the stands.
    let crowd = model
        .scene
        .meshes
        .iter()
        .find(|m| m.node.is_some())
        .expect("a bound mesh");
    assert!(crowd.name.contains("crowdentity"), "{}", crowd.name);
}
