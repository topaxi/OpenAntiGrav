//! Wipeout HD's `Weapon Pad` chunks, split out of `build_scene`'s world-space
//! pass rather than baked unremovably into the circuit's own model.
//!
//! **`#[ignore]`d and never run in CI.** It needs a decrypted PS3 disc image,
//! which this project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_weapon_pad_split_ground_truth)'
//! ```
//!
//! # What this pins
//!
//! Reported from play: HD drew weapon pads in every solo mode, not just
//! `SingleRace`. The cause was that `oag_raceplay::load` set
//! `weapon_pad_model` to `None` on the PS3 path unconditionally, so
//! `Scene::new`'s `Mode::weapons_enabled` gate had nothing to act on - HD's
//! weapon pads were never routed through it at all, because they were baked
//! into the circuit's own `track_model` alongside the road, walls and
//! scenery. See `docs/gameplay/race-modes.md` and
//! `crates/mesh/src/mesh/rcs.rs`'s own doc comment on `build_scene`.
//!
//! This checks the fix at the layer it belongs to: the decode, not the
//! render. `crates/render/examples/hd_pads.rs` already established that
//! Talon's Junction authors 9 `Weapon Pad` nodes; this checks `build_scene`
//! excludes exactly those chunks from the circuit model and
//! `mesh::rcs::build_weapon_pads` draws them separately, on the real disc
//! bytes.
//!
//! **The mechanism moved on 2026-09-02, the property this pins did not.**
//! `build_scene` used to split a weapon-pad chunk into a second model of its
//! own, returned alongside the circuit one; it now simply excludes both pad
//! classes' chunks (`mesh::rcs::pads::pad_chunk_hashes`) and leaves drawing
//! them to their own node-ordered pass - see `build_scene`'s own doc
//! comment. That pass is what gives a pad model real per-node
//! `Model::node_vertex_ranges`, which the world-space split never could, and
//! is what `Drawable::tint_weapon_pads` needs to cycle a
//! pad's ready/cooling colour at all.

use std::path::PathBuf;

use oag_mesh::mesh;
use oag_vex::vex;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

/// Talon's Junction: `hd_pads.rs` counts 9 `Weapon Pad` nodes here, all
/// resolving to a chunk this circuit's `.rcsmodel` carries.
const TRACK: &str = "/data/environments/talons_junction/track.vex";
const ARCHIVE: &str = "PS3_GAME/USRDIR/DATA00.PSARC";

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn weapon_pad_chunks_leave_the_circuit_model_and_land_in_their_own() {
    let Some(image) = image() else {
        return;
    };
    let spec = format!("{}:{ARCHIVE}", image.display());
    let vex_data = mesh::read_blob(&spec, TRACK).expect("reading the .vex");
    let model_name = mesh::rcs::sibling_name(TRACK).expect("a .vex name to rewrite");
    let model_blob = mesh::read_blob(&spec, &model_name).expect("reading the .rcsmodel");

    // The independent count: walk `Weapon Pad` nodes directly, the way
    // `hd_pads.rs` does, rather than trusting `build_scene`'s own report to
    // grade itself.
    let nodes = vex::nodes(&vex_data).expect("walking the node tree");
    let classes = vex::classes_of(&vex_data).expect("class table");
    let pad_class = classes.weapon_pad.expect("weapon_pad id recovered for v6");
    let rcs_model = oag_rcs::rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");
    let expected_hashes: Vec<u32> = nodes
        .iter()
        .filter(|n| n.class_id == pad_class)
        .filter_map(|n| {
            let payload = &vex_data[n.payload()];
            (payload.len() >= 0x34).then(|| vex::byte_order(&vex_data).u32(payload, 0x30))
        })
        .filter(|hash| rcs_model.mesh(*hash).is_some())
        .collect();
    assert_eq!(
        expected_hashes.len(),
        9,
        "talons_junction should author 9 placed Weapon Pad chunks, the same \
         count hd_pads.rs reports"
    );

    let (track_model, report) =
        mesh::rcs::build_scene(TRACK, &vex_data, &model_blob, &mut |path| {
            mesh::read_blob(&spec, path).ok()
        })
        .expect("build_scene decodes talons_junction");
    println!("{}", report.describe());

    let (weapon_pad_model, pad_report) =
        mesh::rcs::build_weapon_pads(TRACK, &vex_data, &model_blob, &mut |path| {
            mesh::read_blob(&spec, path).ok()
        })
        .expect("build_weapon_pads decodes talons_junction");
    println!("{}", pad_report.describe());

    assert_eq!(
        pad_report.drawn,
        expected_hashes.len(),
        "build_weapon_pads' own drawn-chunk count against the independent node walk"
    );
    assert!(!weapon_pad_model.indices.is_empty());

    // **The chunk this project cares about most: it must not double-draw.**
    // Every draw call in the *circuit's* model carries the `.rcsmodel` chunk
    // index it came from (`DrawCall::chunk`); none of them may name a chunk
    // this circuit's weapon pads own.
    let pad_chunk_indices: Vec<u32> = expected_hashes
        .iter()
        .filter_map(|&hash| rcs_model.mesh_index(hash))
        .map(|i| u32::try_from(i).expect("a chunk index fits in u32"))
        .collect();
    assert_eq!(pad_chunk_indices.len(), expected_hashes.len());
    for draws in [
        &track_model.draws,
        &track_model.alpha_tested_draws,
        &track_model.transparent_draws,
    ] {
        for draw in draws {
            if let Some(chunk) = draw.chunk {
                assert!(
                    !pad_chunk_indices.contains(&chunk),
                    "the circuit model still carries a draw call for weapon pad chunk {chunk}"
                );
            }
        }
    }
}

/// Talon's Junction's 18 `Speedup Pad` nodes each name a hash no chunk of its
/// `.rcsmodel` carries, so `build_pads`' node pass draws nothing - and the
/// pads are on screen all the same, as 18 chunks on a pad material that
/// `build_scene`'s world-space pass draws. The report has to say that rather
/// than "0 of 18 ... (0 triangle(s))", which read as an absent picture.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn speedup_pads_with_no_addressed_chunk_report_the_world_pass_chunks() {
    let Some(image) = image() else {
        return;
    };
    let spec = format!("{}:{ARCHIVE}", image.display());
    let vex_data = mesh::read_blob(&spec, TRACK).expect("reading the .vex");
    let model_name = mesh::rcs::sibling_name(TRACK).expect("a .vex name to rewrite");
    let model_blob = mesh::read_blob(&spec, &model_name).expect("reading the .rcsmodel");
    let rcs_model = oag_rcs::rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");

    let nodes = vex::nodes(&vex_data).expect("walking the node tree");
    let classes = vex::classes_of(&vex_data).expect("class table");
    let speedup = classes
        .speedup_pad
        .expect("speedup_pad id recovered for v6");
    let order = vex::byte_order(&vex_data);
    let hashes: Vec<u32> = nodes
        .iter()
        .filter(|n| n.class_id == speedup)
        .map(|n| order.u32(&vex_data[n.payload()], 0x30))
        .collect();
    assert_eq!(hashes.len(), 18, "talons_junction authors 18 speedup pads");
    assert!(
        hashes.iter().all(|h| rcs_model.mesh(*h).is_none()),
        "a speedup node now resolves: the world-pass reading below needs revisiting"
    );

    let (pads, report) = mesh::rcs::build_pads(TRACK, &vex_data, &model_blob, &mut |path| {
        mesh::read_blob(&spec, path).ok()
    })
    .expect("build_pads decodes talons_junction");
    println!("{}", report.describe());
    assert!(pads.indices.is_empty(), "the node pass draws no pad");
    assert_eq!(report.routed_chunks, 18, "one world-pass chunk per pad");
    assert!(report.routed_triangles > 0);
    let line = report.describe();
    assert!(!line.contains("(0 triangle(s))"), "{line}");
    assert!(line.contains("18 chunk(s) on a pad material"), "{line}");

    // **The report must be about what is drawn.** Every chunk on a pad
    // material that no pad node names has a draw call in the circuit's own
    // model, and the count of them is the report's.
    let (track_model, _) = mesh::rcs::build_scene(TRACK, &vex_data, &model_blob, &mut |path| {
        mesh::read_blob(&spec, path).ok()
    })
    .expect("build_scene decodes talons_junction");
    let drawn: std::collections::BTreeSet<u32> = [
        &track_model.draws,
        &track_model.alpha_tested_draws,
        &track_model.transparent_draws,
    ]
    .into_iter()
    .flatten()
    .filter_map(|draw| draw.chunk)
    .collect();
    let pad_chunks: Vec<u32> = rcs_model
        .meshes
        .iter()
        .enumerate()
        .filter(|(_, chunk)| {
            chunk.surfaces().any(|s| {
                rcs_model
                    .materials
                    .get(s.material as usize)
                    .is_some_and(|m| m.name.ends_with("weapon_pads.rcsmaterial"))
            })
        })
        .filter(|(_, chunk)| {
            ![classes.weapon_pad.expect("weapon_pad id recovered for v6")]
                .iter()
                .any(|&class| {
                    nodes
                        .iter()
                        .filter(|n| n.class_id == class)
                        .any(|n| order.u32(&vex_data[n.payload()], 0x30) == chunk.hash)
                })
        })
        .map(|(index, _)| u32::try_from(index).expect("a chunk index fits in u32"))
        .collect();
    assert_eq!(
        pad_chunks.len(),
        18,
        "the chunks on a pad material no node names"
    );
    for chunk in &pad_chunks {
        assert!(
            drawn.contains(chunk),
            "chunk {chunk} is counted but not drawn"
        );
    }
}
