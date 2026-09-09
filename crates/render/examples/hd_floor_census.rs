//! Census: for every HD circuit, in both directions, how many chunks the
//! loader drops entirely (no recoverable stride) or partially (a submesh that
//! fails `submesh_fits`), broken down by whether the chunk's material is
//! see-through - the closest thing to a machine definition of "a glass floor
//! panel went missing" without a picture.
//!
//! Two passes, because `oag_render::mesh::rcs::build`/`build_scene` use two
//! different stride-recovery paths for the same reason: a node-addressed
//! chunk (a prop) has an authored box to check against and a per-submesh
//! `submesh_fits` gate; a world-space chunk (the road, most of a circuit) has
//! neither - see `docs/formats/rcsmodel.md`, "Wipeout HD's road is not in the
//! `.vex`".
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_floor_census -- \
//!     data/images/hdfury-ps3-eu-dec.iso
//! ```

use oag_formats::rcsmodel;

use oag_render::mesh;
use oag_vex::vex;

/// One `(archive, .vex path)` pair per circuit direction, read straight off
/// `scripts/psarc.py list`'s output on the decrypted EU disc image - the
/// disc names four `DATA0N.PSARC` archives and a circuit's forward and
/// reversed models do not always share one.
const CIRCUITS: &[(&str, &str)] = &[
    ("DATA00.PSARC", "/data/environments/amphiseum/track.vex"),
    (
        "DATA00.PSARC",
        "/data/environments/amphiseum/track_reversed.vex",
    ),
    (
        "DATA00.PSARC",
        "/data/environments/modesto_heights/track.vex",
    ),
    (
        "DATA00.PSARC",
        "/data/environments/modesto_heights/track_reversed.vex",
    ),
    (
        "DATA00.PSARC",
        "/data/environments/talons_junction/track.vex",
    ),
    (
        "DATA00.PSARC",
        "/data/environments/talons_junction/track_reversed.vex",
    ),
    ("DATA00.PSARC", "/data/environments/tech_de_ra/track.vex"),
    (
        "DATA00.PSARC",
        "/data/environments/tech_de_ra/track_reversed.vex",
    ),
    ("DATA00.PSARC", "/data/environments/zone_1/track.vex"),
    ("DATA00.PSARC", "/data/environments/zone_2/track.vex"),
    ("DATA00.PSARC", "/data/environments/zone_3/track.vex"),
    ("DATA00.PSARC", "/data/environments/zone_4/track.vex"),
    (
        "DATA02.PSARC",
        "/data/environments/04_chenghou_project/track.vex",
    ),
    ("DATA02.PSARC", "/data/environments/01_vineta_k/track.vex"),
    (
        "DATA02.PSARC",
        "/data/environments/15_anulpha_pass/track.vex",
    ),
    ("DATA02.PSARC", "/data/environments/03_track/track.vex"),
    ("DATA02.PSARC", "/data/environments/02_track/track.vex"),
    (
        "DATA02.PSARC",
        "/data/environments/10_sebenco_climb/track.vex",
    ),
    ("DATA02.PSARC", "/data/environments/05_ubermall/track.vex"),
    ("DATA02.PSARC", "/data/environments/12_sol_2/track.vex"),
    (
        "DATA03.PSARC",
        "/data/environments/01_vineta_k/track_reversed.vex",
    ),
    (
        "DATA03.PSARC",
        "/data/environments/02_track/track_reversed.vex",
    ),
    (
        "DATA03.PSARC",
        "/data/environments/03_track/track_reversed.vex",
    ),
    (
        "DATA03.PSARC",
        "/data/environments/04_chenghou_project/track_reversed.vex",
    ),
    (
        "DATA03.PSARC",
        "/data/environments/05_ubermall/track_reversed.vex",
    ),
    (
        "DATA03.PSARC",
        "/data/environments/10_sebenco_climb/track_reversed.vex",
    ),
    (
        "DATA03.PSARC",
        "/data/environments/12_sol_2/track_reversed.vex",
    ),
    (
        "DATA03.PSARC",
        "/data/environments/15_anulpha_pass/track_reversed.vex",
    ),
];

/// A chunk this census could not get the loader to draw at all, with enough
/// context to look it up by hand.
struct Dropped {
    circuit: String,
    hash: u32,
    material: String,
    see_through: bool,
    /// `true` for a chunk no `.vex` node addresses - the road, walls and
    /// scenery pass, where a per-submesh fits gate does not run at all.
    world_space: bool,
    /// The chunk's own bias, which for a world-space chunk *is* roughly its
    /// world position - close enough to say "this is near the ground" or
    /// not without a full transform.
    bias_y: f32,
    /// Distance from the chunk's own bias to its node's authored box centre,
    /// transformed into world space through the node's own transform chain -
    /// `None` for a world-space (unreferenced) drop, which has no node.
    ///
    /// The pad-precedent signature: a node-addressed chunk baked in world
    /// space anyway has this near zero, because the node's own placement and
    /// the chunk's own bias describe the same point twice.
    bias_matches_node_world: Option<f32>,
    node_name: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());

    let mut dropped: Vec<Dropped> = Vec::new();
    let mut totals = (0usize, 0usize, 0usize); // chunks, world-space, node-addressed

    for &(archive, name) in CIRCUITS {
        let spec = format!("{image}:PS3_GAME/USRDIR/{archive}");
        let data = match mesh::read_blob(&spec, name) {
            Ok(d) => d,
            Err(e) => {
                println!("{name}: could not read - {e}");
                continue;
            }
        };
        let sibling = mesh::rcs::sibling_name(name).unwrap();
        let model_blob = match mesh::read_blob(&spec, &sibling) {
            Ok(d) => d,
            Err(e) => {
                println!("{name}: no sibling .rcsmodel - {e}");
                continue;
            }
        };
        let model =
            rcsmodel::Model::parse(&model_blob).map_err(|e| anyhow::anyhow!("{name}: {e}"))?;
        let classes = vex::classes_of(&data)?;
        let Some(class_id) = classes.mesh else {
            println!("{name}: no mesh class id for this .vex version");
            continue;
        };
        let nodes = vex::nodes(&data)?;
        let order = vex::byte_order(&data);
        let world = vex::world_transforms(&data, &nodes);

        // Every `Mesh` node's `(hash, min, max, world-space box centre, name)`,
        // the box reading `oag_render::mesh::rcs::node_geometry` does, plus the
        // world transform this census needs and the real loader does not.
        type NodeBox = (u32, [f32; 3], [f32; 3], [f32; 3], Option<String>);
        let node_boxes: Vec<NodeBox> = nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.class_id == class_id)
            .filter_map(|(index, n)| {
                let payload = &data[n.payload()];
                if payload.len() < 0x34 {
                    return None;
                }
                let read3 = |at: usize| -> [f32; 3] {
                    std::array::from_fn(|i| order.f32(payload, at + i * 4))
                };
                let min = read3(0x10);
                let max = read3(0x20);
                let m = &world[index];
                let centre = [
                    (min[0] + max[0]) / 2.0,
                    (min[1] + max[1]) / 2.0,
                    (min[2] + max[2]) / 2.0,
                ];
                let world_centre = [
                    centre[0] * m[0] + centre[1] * m[4] + centre[2] * m[8] + m[12],
                    centre[0] * m[1] + centre[1] * m[5] + centre[2] * m[9] + m[13],
                    centre[0] * m[2] + centre[1] * m[6] + centre[2] * m[10] + m[14],
                ];
                Some((
                    order.u32(payload, 0x30),
                    min,
                    max,
                    world_centre,
                    n.name.clone(),
                ))
            })
            .collect();

        let mut circuit_dropped = 0usize;
        let mut circuit_world = 0usize;
        let mut circuit_nodes = 0usize;

        for mesh in &model.meshes {
            totals.0 += 1;
            let material = model
                .material_of(mesh)
                .map(|m| m.name.clone())
                .unwrap_or_else(|| "?".into());
            let see_through = model
                .material_of(mesh)
                .is_some_and(rcsmodel::Material::is_see_through);

            if let Some(&(_, min, max, world_centre, ref node_name)) =
                node_boxes.iter().find(|(h, ..)| *h == mesh.hash)
            {
                // Node-addressed pass, matching `rcs::build`.
                circuit_nodes += 1;
                totals.2 += 1;
                let bias = mesh.bias;
                let bias_matches_node_world = Some(
                    ((bias[0] - world_centre[0]).powi(2)
                        + (bias[1] - world_centre[1]).powi(2)
                        + (bias[2] - world_centre[2]).powi(2))
                    .sqrt(),
                );
                let tolerance = mesh.scale.iter().fold(0.0f32, |a, &b| a.max(b)) * 2.0;
                let stride = mesh
                    .declared_stride()
                    .or_else(|| mesh.solve_stride(&model_blob, (min, max), tolerance))
                    .or_else(|| mesh.solve_stride_by_layout())
                    .or_else(|| mesh.solve_stride_by_normals(&model_blob));
                let Some(stride) = stride else {
                    circuit_dropped += 1;
                    dropped.push(Dropped {
                        circuit: name.to_string(),
                        hash: mesh.hash,
                        material,
                        see_through,
                        world_space: false,
                        bias_y: mesh.bias[1],
                        bias_matches_node_world,
                        node_name: node_name.clone(),
                    });
                    continue;
                };
                let any_submesh_fits = mesh
                    .submeshes
                    .iter()
                    .filter(|s| s.vertex_count > 0 && s.index_count > 0)
                    .any(|s| mesh.submesh_fits(&model_blob, s, stride, (min, max)));
                if !any_submesh_fits && !mesh.submeshes.is_empty() {
                    circuit_dropped += 1;
                    dropped.push(Dropped {
                        circuit: name.to_string(),
                        hash: mesh.hash,
                        material,
                        see_through,
                        world_space: false,
                        bias_y: mesh.bias[1],
                        bias_matches_node_world,
                        node_name: node_name.clone(),
                    });
                }
            } else {
                // World-space pass, matching `rcs::build_scene` - no box, no
                // per-submesh fits gate at all today.
                circuit_world += 1;
                totals.1 += 1;
                let stride = mesh
                    .declared_stride()
                    .or_else(|| mesh.solve_stride_without_a_box(&model_blob));
                if stride.is_none() {
                    circuit_dropped += 1;
                    dropped.push(Dropped {
                        circuit: name.to_string(),
                        hash: mesh.hash,
                        material,
                        see_through,
                        world_space: true,
                        bias_y: mesh.bias[1],
                        bias_matches_node_world: None,
                        node_name: None,
                    });
                }
            }
        }

        let circuit_total = circuit_world + circuit_nodes;
        println!(
            "{name}: {circuit_total} chunk(s), {circuit_world} world-space, \
             {circuit_nodes} node-addressed, {circuit_dropped} dropped entirely"
        );
    }

    println!(
        "\n{} chunk(s) total, {} world-space, {} node-addressed",
        totals.0, totals.1, totals.2
    );
    println!("{} dropped entirely (no drawable stride)", dropped.len());

    let see_through_dropped: Vec<&Dropped> = dropped.iter().filter(|d| d.see_through).collect();
    println!(
        "{} of those are see-through materials - candidates for a missing glass floor/panel",
        see_through_dropped.len()
    );
    let world_baked = |d: &Dropped| d.bias_matches_node_world.is_some_and(|dist| dist < 1.0);
    for d in &dropped {
        let flag = if d.see_through {
            "SEE-THRU"
        } else {
            "opaque  "
        };
        let baked = if world_baked(d) { "WORLD-BAKED" } else { "" };
        println!(
            "  {flag}  {}  hash {:#010x}  material {:<32}  node {:<40}  \
             bias.y {:7.1}  node-vs-bias dist {:>9}  {baked}",
            d.circuit,
            d.hash,
            d.material,
            d.node_name.as_deref().unwrap_or(if d.world_space {
                "(world-space, no node)"
            } else {
                "?"
            }),
            d.bias_y,
            d.bias_matches_node_world
                .map(|v| format!("{v:.2}"))
                .unwrap_or_else(|| "n/a".into()),
        );
    }
    let baked_count = dropped.iter().filter(|d| world_baked(d)).count();
    println!(
        "\n{baked_count} of {} drops match the pad-precedent signature exactly: the node's own \
         authored box, transformed to world space, sits within 1 unit of the chunk's own bias - \
         i.e. the chunk is baked in world space and the node-transform path can never fit it.",
        dropped.len()
    );

    Ok(())
}
