//! Scratch probe: for every hash `referenced()`'s world-bake test now
//! excludes, check whether the OLD node-transform path (`declared_stride`/
//! `solve_stride`/`solve_stride_by_layout`/`solve_stride_by_normals` +
//! `submesh_fits`) would still have drawn it - which would mean the fix
//! makes the world-space pass draw a *second* copy on top of one the node
//! pass already drew, rather than replacing an absence.

use oag_rcs::rcsmodel;

use oag_render::mesh;
use oag_vex::vex;

const CIRCUITS: &[(&str, &str)] = &[
    ("DATA02.PSARC", "/data/environments/12_sol_2/track.vex"),
    (
        "DATA02.PSARC",
        "/data/environments/15_anulpha_pass/track.vex",
    ),
    (
        "DATA02.PSARC",
        "/data/environments/10_sebenco_climb/track.vex",
    ),
    ("DATA02.PSARC", "/data/environments/05_ubermall/track.vex"),
    ("DATA02.PSARC", "/data/environments/01_vineta_k/track.vex"),
    ("DATA02.PSARC", "/data/environments/02_track/track.vex"),
    ("DATA02.PSARC", "/data/environments/03_track/track.vex"),
];

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());

    let mut total_excluded = 0usize;
    let mut total_also_node_drawable = 0usize;

    for &(archive, name) in CIRCUITS {
        let spec = format!("{image}:PS3_GAME/USRDIR/{archive}");
        let data = mesh::read_blob(&spec, name)?;
        let sibling = mesh::rcs::sibling_name(name).unwrap();
        let model_blob = mesh::read_blob(&spec, &sibling)?;
        let model = rcsmodel::Model::parse(&model_blob).map_err(|e| anyhow::anyhow!("{e}"))?;
        let classes = vex::classes_of(&data)?;
        let Some(class_id) = classes.mesh else {
            continue;
        };
        let nodes = vex::nodes(&data)?;
        let order = vex::byte_order(&data);
        let world = vex::world_transforms(&data, &nodes);

        let mut excluded = 0usize;
        let mut also_drawable = 0usize;

        for (index, node) in nodes.iter().enumerate() {
            if node.class_id != class_id {
                continue;
            }
            let payload = &data[node.payload()];
            if payload.len() < 0x34 {
                continue;
            }
            let read3 =
                |at: usize| -> [f32; 3] { std::array::from_fn(|i| order.f32(payload, at + i * 4)) };
            let min = read3(0x10);
            let max = read3(0x20);
            let hash = order.u32(payload, 0x30);
            let Some(mesh) = model.mesh(hash) else {
                continue;
            };
            let centre = [
                (min[0] + max[0]) / 2.0,
                (min[1] + max[1]) / 2.0,
                (min[2] + max[2]) / 2.0,
            ];
            let m = &world[index];
            let world_centre = [
                centre[0] * m[0] + centre[1] * m[4] + centre[2] * m[8] + m[12],
                centre[0] * m[1] + centre[1] * m[5] + centre[2] * m[9] + m[13],
                centre[0] * m[2] + centre[1] * m[6] + centre[2] * m[10] + m[14],
            ];
            let dist = ((world_centre[0] - mesh.bias[0]).powi(2)
                + (world_centre[1] - mesh.bias[1]).powi(2)
                + (world_centre[2] - mesh.bias[2]).powi(2))
            .sqrt();
            if dist >= 1.0 {
                continue; // not excluded by the fix
            }
            excluded += 1;

            // The OLD node path, exactly as `rcs::build` runs it.
            let tolerance = mesh.scale.iter().fold(0.0f32, |a, &b| a.max(b)) * 2.0;
            let stride = mesh
                .declared_stride()
                .or_else(|| mesh.solve_stride(&model_blob, (min, max), tolerance))
                .or_else(|| mesh.solve_stride_by_layout())
                .or_else(|| mesh.solve_stride_by_normals(&model_blob));
            if let Some(stride) = stride {
                let any_fits = mesh
                    .submeshes
                    .iter()
                    .filter(|s| s.vertex_count > 0 && s.index_count > 0)
                    .any(|s| mesh.submesh_fits(&model_blob, s, stride, (min, max)));
                if any_fits {
                    also_drawable += 1;
                    println!(
                        "{name}  hash {hash:#010x}  node {:?}  WOULD ALSO DRAW via the node path \
                         - double submission risk",
                        node.name
                    );
                }
            }
        }

        println!("{name}: {excluded} excluded by the fix, {also_drawable} also node-drawable");
        total_excluded += excluded;
        total_also_node_drawable += also_drawable;
    }

    println!(
        "\n{total_excluded} total exclusions, {total_also_node_drawable} would have also drawn \
         via the node path"
    );
    Ok(())
}
