//! Scratch probe: `index x y z` for every chunk of an HD `.rcsmodel`, in file
//! order - the join key for anything indexed by chunk, `track.pvs` included.

use oag_mesh::mesh;
use oag_rcs::rcsmodel;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());
    let model_blob = mesh::read_blob(&spec, &mesh::rcs::sibling_name(&name).unwrap())?;
    let model = rcsmodel::Model::parse(&model_blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    for (index, chunk) in model.meshes.iter().enumerate() {
        let Some(stride) = chunk
            .declared_stride()
            .or_else(|| chunk.solve_stride_without_a_box(&model_blob))
        else {
            println!("{index} nan nan nan 0 ?");
            continue;
        };
        let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
        for submesh in &chunk.submeshes {
            if let Ok(positions) = chunk.positions(&model_blob, submesh, stride) {
                for p in positions {
                    for i in 0..3 {
                        min[i] = min[i].min(p[i]);
                        max[i] = max[i].max(p[i]);
                    }
                }
            }
        }
        if min[0] > max[0] {
            println!("{index} nan nan nan 0 ?");
            continue;
        }
        let radius =
            ((max[0] - min[0]).powi(2) + (max[1] - min[1]).powi(2) + (max[2] - min[2]).powi(2))
                .sqrt()
                / 2.0;
        let material = model
            .material_of(chunk)
            .map(|m| {
                m.name
                    .rsplit('/')
                    .next()
                    .unwrap_or("?")
                    .trim_end_matches(".rcsmaterial")
                    .to_string()
            })
            .unwrap_or_else(|| "?".into());
        println!(
            "{index} {} {} {} {radius} {material}",
            (min[0] + max[0]) / 2.0,
            (min[1] + max[1]) / 2.0,
            (min[2] + max[2]) / 2.0,
        );
    }
    Ok(())
}
