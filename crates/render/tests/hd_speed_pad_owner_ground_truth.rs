//! Which chunk each speed-pad node of the four original HD circuits owns.
//!
//! Their `Speedup Pad` nodes name a hash no chunk carries (the engine's own
//! resolution of such a hash is unrecovered, `rcsmodel.md`). The data still
//! pairs them: the chunks on a pad material that no pad node of either class
//! names are exactly as many as the speed-pad nodes, and each node's volume
//! holds one chunk's vertex centroid and no other's. That is a **position**
//! correspondence, measured; it is not the engine's rule.
//!
//! `#[ignore]`d: needs the PS3 disc image.
//! `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all -E 'binary(hd_speed_pad_owner_ground_truth)'`

mod archive_cache;

use std::path::Path;

use oag_mesh::mesh;
use oag_rcs::rcsmodel;
use oag_vex::{pads, vex};

const IMAGE: &str = "data/images/hdfury-ps3-eu-dec.iso";

/// A node's volume holds its chunk's centroid to within this (the measured
/// worst is 0.46 on `amphiseum`); the runner-up chunk is never nearer than 7.
const OWN: f32 = 1.0;
const OTHER: f32 = 7.0;

fn owners(track: &str, nodes_expected: usize) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(IMAGE);
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        return;
    }
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", path.display());
    let name = format!("/data/environments/{track}/track.vex");
    let data = archive_cache::read(&spec, &name).expect("reading the track");
    let blob = archive_cache::read(&spec, &mesh::rcs::sibling_name(&name).unwrap())
        .expect("a .rcsmodel beside it");
    let model = rcsmodel::Model::parse(&blob).expect("the model parses");
    let classes = vex::classes_of(&data).expect("classes");
    let nodes = vex::nodes(&data).expect("nodes");
    let order = vex::byte_order(&data);
    let speed = classes.speedup_pad.expect("a speed pad class");
    let volumes = pads::volumes(&data, &nodes, speed);
    assert_eq!(volumes.len(), nodes_expected, "{track}: speed-pad nodes");
    let named: Vec<u32> = nodes
        .iter()
        .filter(|n| {
            Some(n.class_id) == classes.speedup_pad || Some(n.class_id) == classes.weapon_pad
        })
        .map(|n| order.u32(&data[n.payload()], 0x30))
        .collect();
    assert!(
        nodes
            .iter()
            .filter(|n| Some(n.class_id) == classes.speedup_pad)
            .all(|n| model
                .mesh_index(order.u32(&data[n.payload()], 0x30))
                .is_none()),
        "{track}: a speed-pad node resolves, this test is for the ones that do not"
    );
    let mut centroids = Vec::new();
    for chunk in &model.meshes {
        let on_pad = model
            .materials
            .get(chunk.material as usize)
            .is_some_and(|m| m.name.contains("weapon_pads") || m.name.contains("speedup_material"));
        if named.contains(&chunk.hash) || !on_pad {
            continue;
        }
        let stride = chunk
            .declared_stride()
            .or_else(|| chunk.solve_stride_without_a_box(&blob))
            .expect("a stride");
        let points: Vec<_> = chunk
            .submeshes
            .iter()
            .flat_map(|s| chunk.positions(&blob, s, stride).unwrap())
            .collect();
        let n = points.len() as f32;
        centroids.push(std::array::from_fn::<f32, 3, _>(|k| {
            points.iter().map(|p| p[k]).sum::<f32>() / n
        }));
    }
    assert_eq!(
        centroids.len(),
        nodes_expected,
        "{track}: unnamed pad chunks"
    );
    let mut taken = vec![0; centroids.len()];
    for volume in &volumes {
        let mut by_distance: Vec<_> = centroids
            .iter()
            .enumerate()
            .map(|(k, c)| (volume.distance(*c), k))
            .collect();
        by_distance.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        assert!(by_distance[0].0 < OWN, "{track}: no chunk inside a node");
        assert!(by_distance[1].0 > OTHER, "{track}: a node holds two chunks");
        taken[by_distance[0].1] += 1;
    }
    assert!(taken.iter().all(|n| *n == 1), "{track}: not one to one");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn talons_junction() {
    owners("talons_junction", 18);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn amphiseum() {
    owners("amphiseum", 16);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn modesto_heights() {
    owners("modesto_heights", 17);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn tech_de_ra() {
    owners("tech_de_ra", 15);
}
