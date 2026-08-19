//! Scratch probe: which `.rcsmodel` on the disc carries the chunks Talon's
//! Junction's prop nodes address and its own model does not.

use oag_formats::{rcsmodel, vex};
use oag_render::mesh;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let name = "/data/environments/talons_junction/track.vex";
    let spec = format!("{image}:PS3_GAME/USRDIR/DATA00.PSARC");
    let data = mesh::read_blob(&spec, name)?;
    let model_blob = mesh::read_blob(&spec, &mesh::rcs::sibling_name(name).unwrap())?;
    let model = rcsmodel::Model::parse(&model_blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    let classes = vex::classes_of(&data)?;
    let class_id = classes.mesh.unwrap();
    let nodes = vex::nodes(&data)?;
    let order = vex::byte_order(&data);
    let mut wanted = std::collections::BTreeMap::new();
    for node in &nodes {
        if node.class_id != class_id {
            continue;
        }
        let payload = &data[node.payload()];
        if payload.len() < 0x34 {
            continue;
        }
        let hash = order.u32(payload, 0x30);
        if model.mesh(hash).is_none() {
            wanted.insert(hash, node.name.clone().unwrap_or_default());
        }
    }
    println!("{} unresolved hashes", wanted.len());

    for archive_index in 0..7 {
        let spec = format!("{image}:PS3_GAME/USRDIR/DATA{archive_index:02}.PSARC");
        let Ok(mut psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = psarc
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = psarc.read_path(&path) else {
                continue;
            };
            let Ok(candidate) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            let hits: Vec<_> = wanted
                .iter()
                .filter(|(hash, _)| candidate.mesh(**hash).is_some())
                .map(|(hash, name)| format!("{hash:#010x} {name}"))
                .collect();
            if !hits.is_empty() {
                println!("DATA{archive_index:02} {path}: {} hit(s)", hits.len());
                for hit in hits {
                    println!("    {hit}");
                }
            }
        }
    }
    Ok(())
}
