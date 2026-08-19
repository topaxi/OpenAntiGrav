//! Scratch probe: across every HD circuit, the chunks a `.vex` node mentions
//! that the mesh pass does not draw - the set a narrowed `referenced()` would
//! newly hand to the world-space pass - with node class and material.

use oag_formats::{rcsmodel, vex};
use oag_render::mesh;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    for archive_index in 0..7 {
        let spec = format!("{image}:PS3_GAME/USRDIR/DATA{archive_index:02}.PSARC");
        let Ok(mut psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let vexes: Vec<String> = psarc
            .paths()
            .iter()
            .filter(|p| p.ends_with(".vex") && p.contains("/environments/"))
            .cloned()
            .collect();
        for path in vexes {
            let Ok(data) = psarc.read_path(&path) else {
                continue;
            };
            let Some(sibling) = mesh::rcs::sibling_name(&path) else {
                continue;
            };
            let Ok(model_blob) = psarc.read_path(&sibling) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&model_blob) else {
                continue;
            };
            let known: std::collections::HashMap<u32, usize> = model
                .meshes
                .iter()
                .enumerate()
                .map(|(i, m)| (m.hash, i))
                .collect();
            let Ok(classes) = vex::classes_of(&data) else {
                continue;
            };
            let Some(mesh_class) = classes.mesh else {
                continue;
            };
            let Ok(nodes) = vex::nodes(&data) else {
                continue;
            };
            let order = vex::byte_order(&data);
            let mut drawn = std::collections::BTreeSet::new();
            let mut mentioned: std::collections::BTreeMap<u32, Vec<(u32, usize)>> =
                Default::default();
            for node in &nodes {
                let payload = &data[node.payload()];
                for at in (0..payload.len().saturating_sub(3)).step_by(4) {
                    let word = order.u32(payload, at);
                    if !known.contains_key(&word) {
                        continue;
                    }
                    if node.class_id == mesh_class && at == 0x30 {
                        drawn.insert(word);
                    }
                    mentioned.entry(word).or_default().push((node.class_id, at));
                }
            }
            for (hash, sites) in &mentioned {
                if drawn.contains(hash) {
                    continue;
                }
                let chunk = &model.meshes[known[hash]];
                let material = model
                    .material_of(chunk)
                    .map(|m| {
                        m.name
                            .rsplit('/')
                            .next()
                            .unwrap_or(m.name.as_str())
                            .to_string()
                    })
                    .unwrap_or_else(|| "?".into());
                let classes: Vec<String> = sites
                    .iter()
                    .map(|(class, at)| format!("{class:#x}+{at:#x}"))
                    .collect();
                println!("{path}: {hash:#010x} {material} via {}", classes.join(" "));
            }
        }
    }
    Ok(())
}
