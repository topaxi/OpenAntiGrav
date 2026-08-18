//! `.rcsmodel` **as a container**, read off the PS3 disc and checked against
//! the `.vex` beside it.
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
//! What a *vertex* holds is the other binary,
//! `rcsmodel_vertex_ground_truth.rs`; the two were one file until it passed a
//! thousand lines.
//!
//! # What makes this a measurement rather than a restatement
//!
//! Every assertion here is against something **the `.vex` says and the
//! `.rcsmodel` does not**, or against the whole disc rather than the three
//! files every other claim rests on. A reader that decoded the wrong bytes
//! cannot pass by being self-consistent - which is the failure mode a container
//! test has to be built against, and the one that made `docs/formats/psarc.md`'s
//! per-entry digest check worth writing.
//!
//! Four claims:
//!
//! 1. **Every `Mesh` node's hash resolves to a chunk.** The link is the whole
//!    reason the two files can be read together at all.
//! 2. **Every index is in range and every count is a multiple of three.**
//! 3. **There is no shared props model**, so the 56 nodes that address no chunk
//!    are not waiting in one. A negative result, and it closes a hypothesis
//!    this format page carried from the day it was written.
//! 4. **Every model on the disc parses**, in one of two chunk layouts.

mod rcsmodel_common;

use oag_formats::rcsmodel;
use rcsmodel_common::*;

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

/// Claim 8: the 56 `Mesh` nodes that address no chunk in their own circuit are
/// **not** waiting in a shared props file, and 38 of them are nowhere on the
/// disc at all.
///
/// **A negative result, and it closes a hypothesis this page had carried since
/// the format was read.** `rcsmodel.md` recorded "a shared props archive
/// elsewhere on the disc is the obvious guess and has not been looked for". It
/// has now been looked for and it does not exist: every `.rcsmodel` under
/// `/data/environments/` is a circuit's own `track`, its `track_reversed`, or a
/// 128-byte `padreplacement` with no meshes in it.
///
/// What the sweep did find is stranger and is why this is a test rather than a
/// note. **18 of the 56 hashes are carried by *other circuits'* model files** -
/// the sky traffic (`Skycar_1Shape`, `tanker1aShape`, `shipintersteller1Shape`)
/// by `amphiseum` and `tech_de_ra`, and eight `pCube*` nodes by three circuits
/// that share nothing else with Talon's Junction.
///
/// **13 of the 18 have a donor whose geometry is demonstrably the node's, and
/// 5 have none.** Every donor is judged at the stride its own chunk declares,
/// over the vertices a triangle names, against the box the Talon's Junction
/// node authors - and the check is per donor rather than per hash, because a
/// hash can appear in several circuits' models and be the right mesh in only
/// one of them: `Skycar_1Shape` fits in `tech_de_ra` and not in `amphiseum`.
///
/// So the identity is settled and the *mechanism* is not. **Nothing is wired.**
/// What says the original resolves a node against another circuit's model file,
/// or draws these nodes at all, is unrecovered, and a prop drawn because this
/// project found its geometry somewhere is a picture nobody can check. The
/// nodes stay undrawn and the load report keeps saying so.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_nodes_that_address_no_chunk_are_not_in_a_shared_props_file() {
    let Some((blob, model_blob)) = pair(
        "DATA00.PSARC",
        "/data/environments/talons_junction/track.vex",
        "/data/environments/talons_junction/track.rcsmodel",
    ) else {
        return;
    };
    let model = rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");

    // The circuit's own unresolved nodes, with the box each one authors.
    let unresolved: Vec<Orphan> = mesh_nodes(&blob)
        .into_iter()
        .filter(|(_, hash, ..)| model.mesh(*hash).is_none())
        .map(|(name, hash, min, max)| (name, hash, (min, max)))
        .collect();
    assert_eq!(
        unresolved.len(),
        56,
        "the shortfall this test is about changed"
    );

    // Every `.rcsmodel` on the disc, and where each wanted hash turns up.
    let image = image().expect("checked by `pair`");
    let mut donors: std::collections::BTreeMap<u32, Vec<String>> = Default::default();
    let (mut files, mut chunks, mut unparsed) = (0usize, 0usize, 0usize);
    let mut environment_models: std::collections::BTreeSet<String> = Default::default();
    for archive in [
        "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
    ] {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            if let Some(rest) = path.strip_prefix("/data/environments/") {
                // The directory, not the file name: a shared props model would
                // be the one that does not sit under a circuit's own folder.
                environment_models.insert(rest.split('/').next().unwrap_or("").to_string());
            }
            let Ok(bytes) = open.read_path(&path) else {
                continue;
            };
            let Ok(m) = rcsmodel::Model::parse(&bytes) else {
                unparsed += 1;
                continue;
            };
            files += 1;
            chunks += m.meshes.len();
            for mesh in &m.meshes {
                if unresolved.iter().any(|(_, h, _)| h == &mesh.hash) {
                    donors
                        .entry(mesh.hash)
                        .or_default()
                        .push(format!("{archive}:{path}"));
                }
            }
        }
    }
    println!(
        "indexed {files} .rcsmodel file(s) and {chunks} chunk(s), {unparsed} did not parse; \
         /data/environments/ holds {} directory(ies): {environment_models:?}",
        environment_models.len()
    );

    // 1. **There is no shared props model**, because every `.rcsmodel` under
    //    `/data/environments/` sits in a circuit's own directory. 16 circuits,
    //    16 directories, and no seventeenth for anything they have in common.
    assert_eq!(
        environment_models.len(),
        16,
        "/data/environments/ has a directory that is not one of the 16 \
         circuits, which is where a shared props model would be"
    );

    // 2. Some of the wanted hashes are in other circuits' models.
    let elsewhere = unresolved
        .iter()
        .filter(|(_, hash, _)| donors.contains_key(hash))
        .count();
    println!(
        "{elsewhere} of {} unresolved nodes resolve in another circuit's model",
        unresolved.len()
    );
    assert!(
        (10..=30).contains(&elsewhere),
        "{elsewhere} is outside the range this finding was measured at"
    );

    // 3. And which of them is actually the node's mesh.
    //
    // **Every donor, not the first one.** A hash turns up in several circuits'
    // models and is the right mesh in only some of them, so taking whichever
    // file the archive sweep reached first answers a question about iteration
    // order. The stride is the donor chunk's own declared one where it has
    // (which is every chunk on the disc that is `Layout::Described`) and the
    // search otherwise.
    let (mut fits, mut checked) = (0usize, 0usize);
    for (name, hash, bounds) in &unresolved {
        let Some(where_) = donors.get(hash) else {
            continue;
        };
        checked += 1;
        let mut any = false;
        for spec in where_ {
            let (archive, path) = spec.split_once(':').expect("archive:path");
            let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
            let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
            let bytes = open.read_path(path).expect("the donor reads");
            let m = rcsmodel::Model::parse(&bytes).expect("the donor parses");
            let donor = m.mesh(*hash).expect("the hash that put it in this list");
            let stride = donor
                .declared_stride()
                .or_else(|| donor.solve_stride(&bytes, *bounds, TOLERANCE));
            // **Tightness, not containment.** A chunk small enough to sit
            // inside the box is inside every larger box too, and
            // `Skycar_1Shape`'s 40-vertex chunk in `amphiseum` passes that way
            // while its real donor - 848 vertices in `tech_de_ra` - fills the
            // box exactly. The oracle is computed here rather than borrowed
            // from `solve_stride`, so it is a second reading of the same claim.
            let fitted = stride.is_some_and(|stride| {
                let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                let mut any = false;
                for submesh in donor.submeshes.iter().filter(|s| s.vertex_count > 0) {
                    let (Ok(points), Ok(indices)) = (
                        donor.positions(&bytes, submesh, stride),
                        donor.indices(&bytes, submesh),
                    ) else {
                        continue;
                    };
                    for point in indices.iter().filter_map(|i| points.get(*i as usize)) {
                        any = true;
                        for k in 0..3 {
                            lo[k] = lo[k].min(point[k]);
                            hi[k] = hi[k].max(point[k]);
                        }
                    }
                }
                any && (0..3).all(|k| {
                    (lo[k] - bounds.0[k]).abs() <= TOLERANCE
                        && (hi[k] - bounds.1[k]).abs() <= TOLERANCE
                })
            });
            if fitted {
                any = true;
                println!("  {name} ({hash:#010x}) is the chunk in {path}");
                break;
            }
        }
        if any {
            fits += 1;
        } else {
            println!(
                "  {name} ({hash:#010x}) matches no donor's geometry, in {} file(s)",
                where_.len()
            );
        }
    }
    println!("{fits} of {checked} unresolved nodes have a donor that is their mesh");
    assert_eq!(
        (fits, checked),
        (13, 18),
        "the split this finding is: 13 identified, 5 not"
    );
}

/// Claim 9: every `.rcsmodel` on the disc parses, in one of **two** chunk
/// layouts.
///
/// **This started as a ratchet at 419 of 643 and is now 643 of 643.** The 224
/// that failed all failed identically - `OutOfBounds` on the submesh
/// descriptors - and the reason was a second chunk layout nobody had noticed,
/// because the three files every other claim on this page rests on do not use
/// it. Byte `+0x06` of a chunk selects:
///
/// - `0x05`: a submesh count at `+0x50` and a table of `0x80`-byte descriptors
///   at `+0x60`. The layout this reader was written against.
/// - `0x01`: **no descriptor table at all.** One submesh, its buffers named in
///   the chunk header - vertex offset `+0x54`, index count `+0x58`, index
///   offset `+0x5c`, vertex count `+0x6c`. The word at `+0x50` is a file-wide
///   pointer here, and reading it as a submesh count is what produced counts in
///   the hundreds and descriptors past the end of a 1 KiB file.
///
/// **What settles the inline field mapping is claim 2, not this test.** Every
/// index in every submesh of all 643 files is below its own submesh's vertex
/// count, and those two numbers come from different fields - so a wrong offset
/// for either would show up as an out-of-range index rather than as a
/// plausible-looking mesh.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_model_on_the_disc_parses_in_one_of_two_chunk_layouts() {
    let Some(image) = image() else {
        return;
    };
    let (mut total, mut parsed, mut versioned, mut header_ok) = (0usize, 0usize, 0usize, 0usize);
    let mut layouts: std::collections::BTreeMap<u8, usize> = Default::default();
    let mut errors: Vec<String> = Vec::new();
    let (mut submeshes, mut in_range, mut triangles) = (0usize, 0usize, 0usize);
    for archive in [
        "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
    ] {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let bytes = open.read_path(&path).expect("the entry reads");
            total += 1;
            let big = oag_formats::ByteOrder::Big;
            versioned += usize::from(big.u32(&bytes, 0) == rcsmodel::VERSION);
            let count = big.u32(&bytes, 0x1c) as usize;
            let table = big.u32(&bytes, 0x20) as usize;
            header_ok += usize::from(big.u32(&bytes, 0x04) as usize == table + count * 4);
            for i in 0..count {
                let at = big.u32(&bytes, table + i * 4) as usize;
                if let Some(&byte) = bytes.get(at + 0x06) {
                    *layouts.entry(byte).or_default() += 1;
                }
            }
            match rcsmodel::Model::parse(&bytes) {
                Ok(model) => {
                    parsed += 1;
                    // **The check that settles the inline field mapping.** The
                    // vertex count and the index buffer come from different
                    // fields, so a wrong offset for either shows up here as an
                    // out-of-range index rather than as a plausible mesh.
                    for mesh in &model.meshes {
                        for submesh in &mesh.submeshes {
                            if submesh.index_count == 0 {
                                continue;
                            }
                            submeshes += 1;
                            triangles += usize::from(submesh.index_count % 3 == 0);
                            in_range += usize::from(mesh.indices(&bytes, submesh).is_ok());
                        }
                    }
                }
                Err(e) => {
                    if errors.len() < 5 {
                        errors.push(format!("{path}: {e}"));
                    }
                }
            }
        }
    }
    println!(
        "{parsed} of {total} .rcsmodel files parse; {versioned} carry the version word, \
         {header_ok} satisfy the header arithmetic; chunk layouts {layouts:?}; \
         {in_range} of {submeshes} submeshes have every index in range, \
         {triangles} a whole number of triangles"
    );
    for e in &errors {
        println!("  {e}");
    }
    assert_eq!(
        versioned, total,
        "the version claim is disc-wide, or it is not"
    );
    assert_eq!(
        header_ok, total,
        "the directory layout is disc-wide, or it is not"
    );
    assert_eq!(parsed, total, "not every model parses any more");
    assert_eq!(
        layouts.keys().copied().collect::<Vec<_>>(),
        vec![0x01, 0x05],
        "a third chunk layout turned up, and it is being read as one of the two"
    );
    assert_eq!(
        in_range,
        submeshes,
        "{} submesh(es) name a vertex they do not have, so a buffer field is \
         being read from the wrong offset",
        submeshes - in_range
    );
    assert_eq!(
        triangles,
        submeshes,
        "{} submesh(es) do not hold a whole number of triangles",
        submeshes - triangles
    );
}
