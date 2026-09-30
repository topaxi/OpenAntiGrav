//! Validates the PS4 `.rcsskeleton`/`.rcsanimclip` readers - the Vita's files
//! with 64-bit offsets - against every skeleton-bearing model in Wipeout:
//! Omega Collection's five base archives.
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
//! The same closures `psp2_animation_ground_truth.rs` asserts of the Vita's
//! files, over the PS4's, so a wrong pointer width or a wrong field offset
//! cannot survive:
//!
//! - **every clip id is a skeleton id** and no clip has more tracks than ids;
//! - **every channel's key count is its duration over its key spacing**, to
//!   within one key, and the clip's duration is its longest track's;
//! - **every node without a bind matrix of its own is a skeleton node**: the
//!   PS4's model lists far more nodes than its skeleton (`tech_de_ra`: 1,702
//!   against 168, the rest placed by the model's own written matrix), so
//!   "every node is named" is the Vita's closure and not this one - but the
//!   nodes with *no* matrix are exactly what the skeleton exists to place, and
//!   on all 12,310 of them across the five archives it does;
//! - **the model's bind matrix of every node that has one written is the
//!   skeleton's own scale, rotation and translation composed through the
//!   hierarchy without the pivots**, exact on every such node of every file,
//!   which pins the parent array, the root marker, the root's static parent
//!   matrix and the composition order at once - a wrong offset lands on floats
//!   that are not a matrix.
//!
//! One archive per test, so the sweep parallelises across tests.

use std::path::{Path, PathBuf};

use oag_rcs::rcsanimclip;
use oag_rcs::rcsmodel::psp2;
use oag_rcs::rcsskeleton::{self, Kind, local_matrix, multiply};

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
#[derive(Default, Debug, PartialEq, Eq)]
struct Survey {
    skeletons: usize,
    clips: usize,
    nodes: usize,
    tracks: usize,
    channels: usize,
    keys: usize,
    binds_written: usize,
    bind_matches: usize,
    node_bound_meshes: usize,
    /// Mesh objects on a node with no written bind matrix that the skeleton
    /// names, and that it does not.
    unwritten_named: usize,
    unwritten_absent: usize,
}

fn sweep(name: &str) -> Option<Survey> {
    let mut archive = oag_assets::psarc::Archive::open_file(&package(name)?).expect("opens");
    let skeletons: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".rcsskeleton"))
        .cloned()
        .collect();
    let mut survey = Survey::default();
    for path in skeletons {
        let stem = &path[..path.len() - ".rcsskeleton".len()];
        let skeleton = rcsskeleton::parse(&archive.read_path(&path).expect("skeleton reads"))
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        survey.skeletons += 1;
        survey.nodes += skeleton.nodes.len();

        let clip_path = format!("{stem}.rcsanimclip");
        let clip = archive
            .read_path(&clip_path)
            .ok()
            .map(|blob| rcsanimclip::parse(&blob).unwrap_or_else(|e| panic!("{clip_path}: {e}")));
        if let Some(clip) = &clip {
            survey.clips += 1;
            assert!(
                clip.tracks.len() <= clip.bound.len(),
                "{clip_path}: more tracks than ids"
            );
            for &id in &clip.bound {
                assert!(
                    skeleton.node_by_id(id).is_some(),
                    "{clip_path}: id {id:#x} is no skeleton node"
                );
            }
            let longest = clip
                .tracks
                .iter()
                .map(|t| t.duration)
                .fold(0.0f32, f32::max);
            assert!(
                (clip.duration - longest).abs() < 0.01,
                "{clip_path}: duration {} vs longest track {longest}",
                clip.duration
            );
            for track in &clip.tracks {
                survey.tracks += 1;
                for (slot, channel) in track.channels.iter().enumerate() {
                    let Some(channel) = channel else { continue };
                    survey.channels += 1;
                    survey.keys += channel.count;
                    let expected = (track.duration / channel.seconds_per_key).round() as usize;
                    assert!(
                        channel.count.abs_diff(expected) <= 1,
                        "{clip_path}: slot {slot} has {} keys for {} s at {} s/key",
                        channel.count,
                        track.duration,
                        channel.seconds_per_key
                    );
                    assert_eq!(
                        channel.keys.len(),
                        channel.count * channel.width(),
                        "{clip_path}: keys tile"
                    );
                    let expected_kind = match slot {
                        0 | 2 | 4 | 5 => Kind::Vec3,
                        1 => Kind::Quat,
                        3 => Kind::Bool,
                        _ => Kind::Scalar,
                    };
                    assert_eq!(channel.kind, expected_kind, "{clip_path}: slot {slot}");
                }
            }
        }

        let model_path = format!("{stem}.rcsmodel");
        let model = psp2::parse(&archive.read_path(&model_path).expect("model reads"))
            .unwrap_or_else(|e| panic!("{model_path}: {e}"));
        let scene = &model.scene;
        for mesh in &scene.meshes {
            let Some(n) = mesh.node else { continue };
            survey.node_bound_meshes += 1;
            let named = skeleton.node_by_id(scene.nodes[n].id).is_some();
            if scene.nodes[n].bind.is_none() {
                survey.unwritten_named += usize::from(named);
                survey.unwritten_absent += usize::from(!named);
            }
        }

        let order = skeleton.order();
        assert_eq!(
            order.len(),
            skeleton.nodes.len(),
            "{path}: the hierarchy has no cycle"
        );
        let mut world = vec![rcsskeleton::IDENTITY; skeleton.nodes.len()];
        for &i in &order {
            let node = &skeleton.nodes[i];
            let local = local_matrix(
                node.scale,
                node.rotation,
                node.translation,
                [0.0; 3],
                [0.0; 3],
            );
            let parent = node.parent.map_or(node.above, |p| world[p]);
            world[i] = multiply(&local, &parent);
        }
        for model_node in &scene.nodes {
            let Some(bind) = model_node.bind else {
                continue;
            };
            let Some(i) = skeleton.node_by_id(model_node.id) else {
                continue;
            };
            survey.binds_written += 1;
            let scale = world[i].iter().fold(1.0f32, |a, &b| a.max(b.abs()));
            let err = world[i]
                .iter()
                .zip(&bind)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0f32, f32::max);
            assert!(
                err <= 0.01 * scale,
                "{model_path}: node {i} (id {:#x}) composes to a bind matrix {err} off the model's own",
                model_node.id
            );
            survey.bind_matches += 1;
        }
    }
    Some(survey)
}

fn closes(name: &str, expected: (usize, usize, usize, usize, usize, usize, usize, usize)) {
    let Some(survey) = sweep(name) else { return };
    assert_eq!(survey.bind_matches, survey.binds_written, "{name}");
    assert_eq!(
        survey.unwritten_absent, 0,
        "{name}: a node with no matrix of its own that the skeleton does not name"
    );
    assert_eq!(
        (
            survey.skeletons,
            survey.clips,
            survey.nodes,
            survey.tracks,
            survey.channels,
            survey.keys,
            survey.bind_matches,
            survey.unwritten_named
        ),
        expected,
        "{name}"
    );
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data00_skeletons_and_clips_close_on_their_models() {
    closes("data00.psarc", (50, 9, 3915, 822, 1439, 430172, 2522, 1908));
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data01_skeletons_and_clips_close_on_their_models() {
    closes("data01.psarc", (8, 8, 2624, 918, 1308, 463962, 2622, 0));
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data02_skeletons_and_clips_close_on_their_models() {
    closes("data02.psarc", (20, 20, 4054, 2054, 3324, 1243348, 4054, 0));
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data03_skeletons_and_clips_close_on_their_models() {
    closes("data03.psarc", (0, 0, 0, 0, 0, 0, 0, 0));
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data04_skeletons_and_clips_close_on_their_models() {
    closes(
        "data04.psarc",
        (32, 32, 12030, 2309, 3958, 1170320, 5625, 10402),
    );
}
