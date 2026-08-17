//! `.rcsmodel` read off the PS3 disc, checked against the `.vex` beside it.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The image has to be layer-1 decrypted first - `scripts/ps3iso.py decrypt`,
//! `docs/formats/ps3-disc.md`.
//!
//! # What makes this a measurement rather than a restatement
//!
//! Every assertion here is against something **the `.vex` says and the
//! `.rcsmodel` does not**: the authored bounding box, and the node's own hash.
//! So a reader that decoded the wrong bytes cannot pass by being
//! self-consistent - which is the failure mode a container test has to be built
//! against, and the one that made `docs/formats/psarc.md`'s per-entry digest
//! check worth writing.
//!
//! Four claims, in order of how much they would cost to get wrong:
//!
//! 1. **Every `Mesh` node's hash resolves to a chunk.** The link is the whole
//!    reason the two files can be read together at all.
//! 2. **Every index is in range and every count is a multiple of three.** 1,274
//!    of 1,274 submeshes, which says the index offsets and counts were read
//!    correctly rather than plausibly.
//! 3. **The recovered geometry fills the authored box.** The vertex stride is
//!    not stored anywhere in the file, so this is both how it is found and the
//!    only check that it was found rightly.
//! 4. **The stride the buffer layout gives is the same one.** The fourth is the
//!    one that does not need the `.vex` at all - it is arithmetic on two fields
//!    of the `.rcsmodel` - and it agrees with the box on 19 and with the
//!    compactness rule on 96, contradicting neither anywhere.

use std::path::{Path, PathBuf};

use oag_formats::{rcsmodel, vex};

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

/// A `.vex` and the `.rcsmodel` beside it, with the archive holding both.
const PAIRS: &[(&str, &str, &str)] = &[
    (
        "DATA02.PSARC",
        "/data/ships/assegai/ship.vex",
        "/data/ships/assegai/ship.rcsmodel",
    ),
    (
        "DATA02.PSARC",
        "/data/ships/assegai/ship_lod1.vex",
        "/data/ships/assegai/ship_lod1.rcsmodel",
    ),
    (
        "DATA00.PSARC",
        "/data/environments/talons_junction/track.vex",
        "/data/environments/talons_junction/track.rcsmodel",
    ),
];

/// Two quantisation steps at the 1/128 scale every file on the disc uses.
const TOLERANCE: f32 = 2.0 / 128.0;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(PS3_IMAGE);

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

/// Both halves of one pair, read straight out of the archive.
fn pair(archive: &str, vex_path: &str, model_path: &str) -> Option<(Vec<u8>, Vec<u8>)> {
    let image = image()?;
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    Some((
        open.read_path(vex_path).expect("the .vex reads"),
        open.read_path(model_path).expect("the .rcsmodel reads"),
    ))
}

/// Every `Mesh` node in a `.vex`, as (name, hash, min, max).
fn mesh_nodes(blob: &[u8]) -> Vec<(String, u32, [f32; 3], [f32; 3])> {
    let classes = vex::classes_of(blob).expect("a class table");
    let mesh = classes.mesh.expect("version 6 numbers Mesh");
    let order = vex::byte_order(blob);
    vex::nodes(blob)
        .expect("the node tree walks")
        .into_iter()
        .filter(|node| node.class_id == mesh)
        .filter_map(|node| {
            let payload = &blob[node.payload()];
            // The box pair at +0x10/+0x20 and the chunk hash at +0x30, which is
            // as much of a PS3 `Mesh` payload as anything has recovered.
            if payload.len() < 0x34 {
                return None;
            }
            let read3 = |at: usize| std::array::from_fn(|i| order.f32(payload, at + i * 4));
            Some((
                node.name.clone().unwrap_or_default(),
                order.u32(payload, 0x30),
                read3(0x10),
                read3(0x20),
            ))
        })
        .collect()
}

/// Claim 1: the hash in a `Mesh` payload addresses a chunk in the `.rcsmodel`
/// beside it - on every node of a craft, and on most of a circuit's.
///
/// **The shortfall is measured, not asserted away.** All 15 of Assegai's mesh
/// nodes resolve and all 4 of its LOD1's do; 70 of Talon's Junction's 126 do,
/// and the 56 that do not are scenery - `Skycar_1Shape`, `tanker1aShape`,
/// `shipintersteller1Shape`, `HyperContintentCraft1Shape`. Their geometry is
/// somewhere this project has not found: `track.rcsmodel` carries **983**
/// chunks to the `.vex`'s 126, so it is not short of them, and the directory
/// holds no second model file for the circuit. Whether those nodes address a
/// shared props file, or carry their reference somewhere other than `+0x30`,
/// is open - see `docs/formats/rcsmodel.md`.
///
/// The numbers are asserted as ratios so this fails if a change makes it
/// *worse*, which is what the test is for; the open question is a doc entry
/// rather than a failing test.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn a_mesh_node_addresses_its_chunk_by_hash_on_every_craft_and_most_of_a_circuit() {
    for (archive, vex_path, model_path) in PAIRS {
        let Some((blob, model_blob)) = pair(archive, vex_path, model_path) else {
            return;
        };
        let model = rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");
        let nodes = mesh_nodes(&blob);
        assert!(!nodes.is_empty(), "{vex_path} has no Mesh nodes");

        let missing: Vec<&str> = nodes
            .iter()
            .filter(|(_, hash, ..)| model.mesh(*hash).is_none())
            .map(|(name, ..)| name.as_str())
            .collect();
        let resolved = nodes.len() - missing.len();
        println!(
            "{vex_path}: {resolved} of {} nodes resolve, into {} chunks; unresolved e.g. {:?}",
            nodes.len(),
            model.meshes.len(),
            &missing[..missing.len().min(4)]
        );

        let floor = if vex_path.contains("/ships/") {
            nodes.len()
        } else {
            nodes.len() / 2
        };
        assert!(
            resolved >= floor,
            "{vex_path}: {resolved} of {} resolve, expected at least {floor}",
            nodes.len()
        );
    }
}

/// Claim 2: the index buffers are big-endian `u16` triangle lists, in range.
///
/// **Checked over the whole file rather than the referenced meshes**, because
/// this claim is about the container: `talons_junction` carries 983 chunks
/// where its `.vex` names 126, so most of what is measured here is geometry no
/// node points at - and it decodes identically, which is the stronger result.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_index_buffer_is_a_triangle_list_within_its_own_vertex_count() {
    let mut total = 0usize;
    for (archive, vex_path, model_path) in PAIRS {
        let Some((_, model_blob)) = pair(archive, vex_path, model_path) else {
            return;
        };
        let model = rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");
        for mesh in &model.meshes {
            for submesh in &mesh.submeshes {
                if submesh.index_count == 0 {
                    continue;
                }
                assert_eq!(
                    submesh.index_count % 3,
                    0,
                    "{model_path}: {} indices is not a whole number of triangles",
                    submesh.index_count
                );
                // `indices` raises `IndexOutOfRange` itself, so this is the
                // assertion as well as the read.
                mesh.indices(&model_blob, submesh)
                    .unwrap_or_else(|e| panic!("{model_path}: {e}"));
                total += 1;
            }
        }
    }
    assert!(total >= 1_200, "only {total} submeshes checked");
    println!("{total} submeshes are in-range triangle lists");
}

/// Claim 4: the stride read out of the file's own buffer layout is the same one
/// the authored box gives, and the same one the compactness rule gives.
///
/// **This is the claim that turned the stride from a heuristic into a
/// reading.** [`rcsmodel::Mesh::solve_stride_by_layout`] measures the step from
/// one submesh's vertex buffer to the next and divides by the vertex count -
/// arithmetic on two numbers the file states outright, decoding nothing. So it
/// is independent of both of the other rules: of the box, which is in a
/// different file, and of the compactness rule, which is about what the bytes
/// look like once read.
///
/// Two agreements, and a **zero** that matters more than either: the rule must
/// never contradict an oracle. It does not.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_stride_the_buffer_layout_gives_is_the_one_the_box_and_the_span_give() {
    let (mut vs_box, mut vs_span, mut disagreements) = (0, 0, 0);
    for (archive, vex_path, model_path) in PAIRS {
        let Some((blob, model_blob)) = pair(archive, vex_path, model_path) else {
            return;
        };
        let model = rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");

        for (_, hash, min, max) in mesh_nodes(&blob) {
            let Some(mesh) = model.mesh(hash) else {
                continue;
            };
            let (Some(layout), Some(boxed)) = (
                mesh.solve_stride_by_layout(),
                mesh.solve_stride(&model_blob, (min, max), TOLERANCE),
            ) else {
                continue;
            };
            vs_box += 1;
            disagreements += usize::from(layout != boxed);
            assert_eq!(
                layout, boxed,
                "{vex_path}: the layout says {layout} where the authored box says {boxed}"
            );
        }

        // Over every chunk, referenced or not - which on a circuit is mostly
        // the unreferenced ones, and the whole reason either rule exists.
        for mesh in &model.meshes {
            let (Some(layout), Some(span)) = (
                mesh.solve_stride_by_layout(),
                mesh.solve_stride_by_extent(&model_blob),
            ) else {
                continue;
            };
            vs_span += 1;
            disagreements += usize::from(layout != span);
            assert_eq!(
                layout, span,
                "{model_path}: the layout says {layout} where the span says {span}"
            );
        }
        let decided = model
            .meshes
            .iter()
            .filter(|m| m.solve_stride_without_a_box(&model_blob).is_some())
            .count();
        let span_only = model
            .meshes
            .iter()
            .filter(|m| m.solve_stride_by_extent(&model_blob).is_some())
            .count();
        println!(
            "{model_path}: {decided} of {} chunks decide a stride, against {span_only} \
             for the span rule alone",
            model.meshes.len()
        );
        assert!(
            decided >= span_only,
            "{model_path}: adding the layout rule decided fewer chunks, not more"
        );
    }
    println!("the layout rule agrees with the box on {vs_box} and with the span on {vs_span}");
    assert_eq!(disagreements, 0);
    assert!(
        vs_box >= 15 && vs_span >= 90,
        "{vs_box} and {vs_span} is too few to mean anything"
    );
}

/// Claim 3: the recovered vertex stride fills the box the `.vex` authors.
///
/// The number to watch is the unresolved count, not the resolved one: a mesh
/// whose stride no single value explains is reported and drawn as nothing, so
/// this test records how much of the disc that is rather than requiring zero.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_recovered_geometry_fills_the_box_the_vex_authors() {
    for (archive, vex_path, model_path) in PAIRS {
        let Some((blob, model_blob)) = pair(archive, vex_path, model_path) else {
            return;
        };
        let model = rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");
        let nodes = mesh_nodes(&blob);

        let mut solved = 0;
        let mut unresolved = Vec::new();
        let mut widths = std::collections::BTreeMap::new();
        let mut addressed = 0;
        for (name, hash, min, max) in &nodes {
            // The nodes claim 1 records as unresolved have no geometry here to
            // solve a stride for; they are that test's finding, not this one's.
            let Some(mesh) = model.mesh(*hash) else {
                continue;
            };
            addressed += 1;
            match mesh.solve_stride(&model_blob, (*min, *max), TOLERANCE) {
                Some(stride) => {
                    solved += 1;
                    *widths.entry(stride).or_insert(0usize) += 1;
                }
                None => unresolved.push(name.as_str()),
            }
        }

        println!(
            "{vex_path}: {solved} of {addressed} addressed meshes solved a stride, \
             widths {widths:?}, unsolved e.g. {:?}",
            &unresolved[..unresolved.len().min(4)]
        );
        assert!(
            solved * 4 >= addressed * 3,
            "{vex_path}: only {solved} of {addressed} meshes recovered a stride"
        );
        for stride in widths.keys() {
            assert!(
                (14..=22).contains(stride) && stride % 2 == 0,
                "{vex_path}: unexpected stride {stride}"
            );
        }
    }
}
