//! Census of every `.rcsmodel` in one or more loose `.psarc` archives through
//! [`oag_rcs::rcsmodel::psp2`]: how many decode, how many relocation sites go
//! unpaired, and what header word `+0x04` holds (0 on the Vita, 0x100 on PS4).
//!
//! `cargo run -p oag-rcs --example omega_rcsmodel_census -- <a.psarc>...`

use std::collections::BTreeMap;

use oag_rcs::rcsmodel::psp2;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in std::env::args().skip(1) {
        let mut archive = oag_assets::psarc::Archive::open(&path)?;
        let names: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
            .cloned()
            .collect();
        let (mut files, mut magic, mut ok, mut unpaired, mut submeshes) = (0, 0, 0, 0, 0);
        let (mut with_geometry, mut with_materials, mut refused) = (0, 0, 0);
        let (mut bound, mut unbound, mut textured_materials, mut materials_total) = (0, 0, 0, 0);
        let (mut scenes, mut nodes_total, mut meshes_total) = (0, 0, 0);
        let (mut linked, mut unlinked, mut node_bound, mut unwritten_binds) = (0, 0, 0, 0);
        let mut word4: BTreeMap<u32, usize> = BTreeMap::new();
        let mut failures = Vec::new();
        for name in names {
            let Ok(blob) = archive.read_path(&name) else {
                refused += 1;
                continue;
            };
            files += 1;
            if blob.len() < 8 || blob[..4] != psp2::MAGIC.to_le_bytes() {
                continue;
            }
            magic += 1;
            *word4
                .entry(u32::from_le_bytes(blob[4..8].try_into()?))
                .or_default() += 1;
            match psp2::parse(&blob) {
                Ok(model) => {
                    ok += 1;
                    unpaired += model.unpaired_pointers;
                    submeshes += model.submeshes.len();
                    with_geometry += usize::from(model.has_geometry());
                    with_materials += usize::from(!model.materials.is_empty());
                    bound += model
                        .submeshes
                        .iter()
                        .filter(|s| s.material.is_some())
                        .count();
                    unbound += model
                        .submeshes
                        .iter()
                        .filter(|s| s.material.is_none())
                        .count();
                    materials_total += model.materials.len();
                    scenes += usize::from(!model.scene.nodes.is_empty());
                    nodes_total += model.scene.nodes.len();
                    meshes_total += model.scene.meshes.len();
                    unwritten_binds += model
                        .scene
                        .nodes
                        .iter()
                        .filter(|n| n.bind.is_none())
                        .count();
                    linked += model.submeshes.iter().filter(|s| s.mesh.is_some()).count();
                    unlinked += model.submeshes.iter().filter(|s| s.mesh.is_none()).count();
                    node_bound += model.submeshes.iter().filter(|s| s.node.is_some()).count();
                    textured_materials += model
                        .materials
                        .iter()
                        .filter(|m| !m.textures.is_empty())
                        .count();
                    if model.unpaired_pointers > 0 && failures.len() < 5 {
                        failures.push(format!("{name}: {} unpaired", model.unpaired_pointers));
                    }
                }
                Err(e) => failures.push(format!("{name}: {e}")),
            }
        }
        println!(
            "{path}: {files} .rcsmodel read ({refused} unreadable), {magic} with the magic, \
             {ok} decode, {with_geometry} with geometry, {with_materials} with a material \
             table, {submeshes} submeshes, {unpaired} unpaired pointers, header word +4 {word4:x?}"
        );
        println!(
            "  material index: {bound} submeshes name a material, {unbound} name none; \
             {textured_materials} of {materials_total} materials resolve a texture path"
        );
        println!(
            "  node table: {scenes} files read one ({nodes_total} nodes, {unwritten_binds} without a \
             written bind matrix, {meshes_total} mesh objects); {linked} submeshes reach a mesh \
             object, {unlinked} reach none, {node_bound} are node-bound"
        );
        for f in failures {
            println!("  {f}");
        }
    }
    Ok(())
}
