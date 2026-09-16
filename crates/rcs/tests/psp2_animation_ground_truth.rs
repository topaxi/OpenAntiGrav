//! Validates the `.rcsskeleton`/`.rcsanimclip` readers and the model's node
//! table against every skeleton-bearing model Wipeout 2048 ships.
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
//! The three files were read together and the reading closes arithmetically
//! across all of them, which is what these tests assert - see
//! `docs/formats/2048-animation.md`:
//!
//! - **every clip id is a skeleton id** and no clip has more tracks than ids;
//! - **every channel's key count is its duration over its key spacing**, and
//!   its key spacing is the reciprocal of its rate, to within one key;
//! - **every mesh object's node is a skeleton node**, whichever order the
//!   model's table lists them in;
//! - **every submesh record is reachable from exactly one mesh object**, so
//!   the record search and the object walk agree on the file's contents;
//! - **the model's bind matrix of every node that has one written is the
//!   skeleton's own scale, rotation and translation composed through the
//!   hierarchy without the pivots** - exact, on every such node of every
//!   file, which is what pins the parent array, the root marker, the root's
//!   static parent matrix, and the composition order all at once.

use std::path::{Path, PathBuf};

use oag_rcs::rcsanimclip;
use oag_rcs::rcsmodel::psp2;
use oag_rcs::rcsskeleton::{self, Kind, local_matrix, multiply};

const PACKAGES: [&str; 3] = [
    "vita/PCSF00007/base/PSP2/data.psarc",
    "vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

fn package(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted")
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

/// What one sweep found.
#[derive(Default, Debug)]
struct Survey {
    files: usize,
    nodes: usize,
    tracks: usize,
    channels: usize,
    keys: usize,
    bind_matches: usize,
    binds_written: usize,
    node_bound_meshes: usize,
    rates: std::collections::BTreeMap<u32, usize>,
    kinds: std::collections::BTreeMap<(usize, &'static str), usize>,
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Scalar => "scalar",
        Kind::Vec3 => "vec3",
        Kind::Quat => "quat",
        Kind::Bool => "bool",
    }
}

fn sweep(package: &Path, survey: &mut Survey) {
    let mut archive = oag_assets::psarc::Archive::open(package.to_str().expect("utf-8 path"))
        .expect("the package opens");
    let skeletons: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.ends_with(".rcsskeleton"))
        .cloned()
        .collect();
    for path in skeletons {
        let stem = path.strip_suffix(".rcsskeleton").expect("suffix");
        let skeleton = rcsskeleton::parse(&archive.read_path(&path).expect("skeleton reads"))
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        // A DLC package can ship a circuit's skeleton and model without its
        // clip (`dlc1`'s Moa Therma) - the base package has that one.
        let clip_path = format!("{stem}.rcsanimclip");
        let clip = archive
            .read_path(&clip_path)
            .ok()
            .map(|blob| rcsanimclip::parse(&blob).unwrap_or_else(|e| panic!("{clip_path}: {e}")));
        let model_path = format!("{stem}.rcsmodel");
        let model = psp2::parse(&archive.read_path(&model_path).expect("model reads"))
            .unwrap_or_else(|e| panic!("{model_path}: {e}"));
        survey.files += 1;
        survey.nodes += skeleton.nodes.len();

        // The clip binds skeleton nodes and nothing else.
        let tracks = clip
            .as_ref()
            .map(|c| c.tracks.as_slice())
            .unwrap_or_default();
        if let Some(clip) = &clip {
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
        }
        for track in tracks {
            survey.tracks += 1;
            for (slot, channel) in track.channels.iter().enumerate() {
                let Some(channel) = channel else { continue };
                survey.channels += 1;
                survey.keys += channel.count;
                *survey
                    .kinds
                    .entry((slot, kind_name(channel.kind)))
                    .or_default() += 1;
                let rate = 1.0 / channel.seconds_per_key;
                *survey.rates.entry(rate.round() as u32).or_default() += 1;
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
            }
        }

        // The model's own table agrees with the skeleton.
        let scene = &model.scene;
        // A Zone circuit has no nodes at all - an empty skeleton beside an
        // empty table - and still lists its mesh objects.
        assert!(!scene.meshes.is_empty(), "{model_path}: no mesh objects");
        let reachable: usize = scene.meshes.iter().map(|m| m.submesh_records.len()).sum();
        assert_eq!(
            reachable,
            model.submeshes.len(),
            "{model_path}: every record is reachable from one mesh object"
        );
        assert!(
            model.submeshes.iter().all(|s| s.mesh.is_some()),
            "{model_path}: every submesh links back"
        );
        for mesh in &scene.meshes {
            if let Some(n) = mesh.node {
                survey.node_bound_meshes += 1;
                assert!(
                    skeleton.node_by_id(scene.nodes[n].id).is_some(),
                    "{model_path}: {} is bound to a node the skeleton lacks",
                    mesh.name
                );
            }
        }
        // The bind matrices: the skeleton's own TRS through the hierarchy,
        // pivots left out.
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
            survey.binds_written += usize::from(skeleton.node_by_id(model_node.id).is_some());
            let Some(i) = skeleton.node_by_id(model_node.id) else {
                continue;
            };
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
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn every_skeleton_clip_and_model_close_on_each_other() {
    let mut survey = Survey::default();
    let mut found = 0;
    for name in PACKAGES {
        let Some(path) = package(name) else { continue };
        found += 1;
        sweep(&path, &mut survey);
    }
    if found == 0 {
        return;
    }
    println!("{survey:#?}");
    // The base package alone carries 49 skeletons; the figures below are
    // the three packages together, and each is a floor rather than an
    // equality so a re-extracted package with one more file does not fail
    // this for the wrong reason.
    assert!(survey.files >= 49, "{} files", survey.files);
    assert!(survey.tracks >= 4305, "{} tracks", survey.tracks);
    assert!(survey.keys >= 1_850_000, "{} keys", survey.keys);
    // Every written bind matrix the skeleton can name composed to itself -
    // the loop above panics on the first that does not, so this is the
    // count that says the loop ran over something.
    assert_eq!(survey.bind_matches, survey.binds_written);
    assert!(
        survey.bind_matches >= 11_775,
        "{} bind matrices",
        survey.bind_matches
    );
    // Five keys a second on the race tracks, thirty on the Zone ones.
    assert!(
        survey.rates.contains_key(&5) && survey.rates.contains_key(&30),
        "{:?}",
        survey.rates
    );
    for (slot, kind) in survey.kinds.keys() {
        let expected = match slot {
            0 | 2 | 4 | 5 => "vec3",
            1 => "quat",
            3 => "bool",
            _ => "scalar",
        };
        assert_eq!(*kind, expected, "slot {slot} carries a {kind}");
    }
}

/// `altima`'s wind turbines, read off its own files with no HD twin: each
/// rotor's geometry sits 1,300 units from its node origin with the pivot at
/// the geometry's centre, and the rotor spins about the x axis every two
/// seconds. With the pivot in the composition the rotor's centroid lands on
/// its hub's; with the bare `S * R * T` it lands 2,500 units away and sweeps
/// a 1,300-unit circle - the measurement that settled the pivot on a circuit
/// Wipeout HD does not ship.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn altimas_rotors_sit_on_their_hubs_because_of_the_pivot() {
    let Some(package) = package(PACKAGES[0]) else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open(package.to_str().expect("utf-8 path"))
        .expect("the package opens");
    let base = "data/art/published/environments/altima/track";
    let model = psp2::parse(
        &archive
            .read_path(&format!("{base}.rcsmodel"))
            .expect("model"),
    )
    .expect("model parses");
    let skeleton = rcsskeleton::parse(
        &archive
            .read_path(&format!("{base}.rcsskeleton"))
            .expect("skeleton"),
    )
    .expect("skeleton parses");
    let clip = rcsanimclip::parse(
        &archive
            .read_path(&format!("{base}.rcsanimclip"))
            .expect("clip"),
    )
    .expect("clip parses");
    let motions: Vec<oag_rcs::rig::NodeMotion> = skeleton
        .nodes
        .iter()
        .map(|n| oag_rcs::rig::NodeMotion {
            node: n.clone(),
            track: clip.track(n.id).cloned(),
        })
        .collect();
    // World matrices at time zero, with the pivot pair in the composition
    // or with the *bind* pose's pivot translation taken back out of it -
    // an approximation of the pivotless form, since the sampled rotation is
    // the clip's first key rather than the bind's; the margin asserted
    // below is a thousand units against a two-unit fit, so the
    // approximation cannot flip it.
    let world_at_zero = |pivot: bool| -> Vec<[f32; 16]> {
        let mut world = vec![rcsskeleton::IDENTITY; skeleton.nodes.len()];
        for &i in &skeleton.order() {
            let node = &skeleton.nodes[i];
            let mut local = motions[i].sample(0.0);
            if !pivot {
                let bare = local_matrix(
                    node.scale,
                    node.rotation,
                    node.translation,
                    [0.0; 3],
                    [0.0; 3],
                );
                let pivoted = node.local();
                for k in 0..3 {
                    local[12 + k] += bare[12 + k] - pivoted[12 + k];
                }
            }
            let parent = node.parent.map_or(node.above, |p| world[p]);
            world[i] = multiply(&local, &parent);
        }
        world
    };
    let skeleton_node_of = |mesh: usize| -> usize {
        let node = model.scene.meshes[mesh].node.expect("node-bound");
        skeleton
            .node_by_id(model.scene.nodes[node].id)
            .expect("in the skeleton")
    };
    let centroid = |mesh: usize, world: &[[f32; 16]]| -> [f32; 3] {
        let w = world[skeleton_node_of(mesh)];
        let mut sum = [0.0f32; 3];
        let mut n = 0usize;
        for submesh in model.submeshes.iter().filter(|s| s.mesh == Some(mesh)) {
            for p in &submesh.positions {
                for (k, acc) in sum.iter_mut().enumerate() {
                    *acc += p[0] * w[k] + p[1] * w[4 + k] + p[2] * w[8 + k] + w[12 + k];
                }
                n += 1;
            }
        }
        std::array::from_fn(|k| sum[k] / n as f32)
    };
    let distance = |a: [f32; 3], b: [f32; 3]| -> f32 {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    };
    let rotors: Vec<usize> = model
        .scene
        .meshes
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            m.name
                .rsplit(':')
                .next()
                .is_some_and(|n| n.starts_with("rotorShape"))
        })
        .map(|(i, _)| i)
        .collect();
    assert_eq!(rotors.len(), 6, "altima has six rotors");
    let with = world_at_zero(true);
    let without = world_at_zero(false);
    for &rotor in &rotors {
        // The hub is the mesh on the rotor node's parent.
        let parent = skeleton.nodes[skeleton_node_of(rotor)]
            .parent
            .expect("a rotor hangs under its hub");
        let hub = (0..model.scene.meshes.len())
            .find(|&m| model.scene.meshes[m].node.is_some() && skeleton_node_of(m) == parent)
            .expect("the hub carries a mesh");
        let on_hub = distance(centroid(rotor, &with), centroid(hub, &with));
        let off_hub = distance(centroid(rotor, &without), centroid(hub, &without));
        assert!(
            on_hub < 5.0,
            "{}: rotor centroid {on_hub} units from its hub's with the pivot",
            model.scene.meshes[rotor].name
        );
        assert!(
            off_hub > 1000.0,
            "{}: without the pivot the rotor would still sit {off_hub} units from its hub - the pivot check has lost its power",
            model.scene.meshes[rotor].name
        );
    }
}
