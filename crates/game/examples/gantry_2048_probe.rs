//! Scratch probe for the start-gantry feature's 2048 pass: dumps the node
//! tree of 2048's own `321Go_*.vex` files, and what their `Mesh` payloads and
//! sibling `.rcsmodel` files look like.
//!
//! ```sh
//! cargo run -q -p oag-game --example gantry_2048_probe -- <base/PSP2/data.psarc>
//! ```

use oag_rcs::rcsmodel::psp2;

use oag_vex::vex;

const CANDIDATES: &[&str] = &[
    "data/billboards/hd_adverts/321go/321go_2048.vex",
    "data/billboards/hd_adverts/321go/321go_2048_combat.vex",
    "data/billboards/hd_adverts/321go/321fight_2048.vex",
    "data/billboards/hd_adverts/321go/321go_startfinish.vex",
    "data/billboards/hd_adverts/321go/321go_zone.vex",
    "data/billboards/hd_adverts/321go/321go_hd_zone_battle.vex",
    "data/billboards/hd_adverts/321go/321go_hd_detonator.vex",
    "data/billboards/hd_adverts/321go/fx350.vex",
    "data/billboards/hd_adverts/321go/321go_hd.vex",
];

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/extracted/vita/PCSF00007/base/PSP2/data.psarc".to_string());
    let mut archive = oag_assets::psarc::Archive::open(&path)?;

    for candidate in CANDIDATES {
        println!("=== {candidate} ===");
        let Ok(blob) = archive.read_path(candidate) else {
            println!("  read: ERROR (not in this archive)");
            continue;
        };
        println!("  bytes: {}", blob.len());
        let external = oag_mesh::mesh::geometry_is_external(&blob);
        println!("  geometry_is_external: {external}");
        let nodes = match vex::nodes(&blob) {
            Ok(n) => n,
            Err(e) => {
                println!("  nodes: ERROR {e}");
                continue;
            }
        };
        let classes = vex::classes_of(&blob).ok();
        let mesh_class = classes.and_then(|c| c.mesh);
        let anim_class = classes.and_then(|c| c.anim_transform);
        println!("  {} node(s), mesh class {mesh_class:?}", nodes.len());
        for node in &nodes {
            let indent = "  ".repeat(node.depth + 1);
            let name = node.name.as_deref().unwrap_or("<unnamed>");
            let is_mesh = mesh_class == Some(node.class_id);
            let payload = &blob[node.payload()];
            let mut extra = String::new();
            if is_mesh {
                extra.push_str(&format!(" payload={} byte(s)", payload.len()));
                if payload.len() >= 0x34 {
                    let hash = u32::from_be_bytes(payload[0x30..0x34].try_into().expect("4 bytes"));
                    extra.push_str(&format!(" hash_be={hash:#010x}"));
                    let hash_le =
                        u32::from_le_bytes(payload[0x30..0x34].try_into().expect("4 bytes"));
                    extra.push_str(&format!(" hash_le={hash_le:#010x}"));
                }
            }
            if Some(node.class_id) == anim_class
                && let Some(anim) = vex::anim_transform_of(&blob, node)
            {
                extra.push_str(&format!(
                    " anim_translation_times={:?}",
                    anim.translation.times
                ));
            }
            println!(
                "{indent}{name} (class {:#x}, depth {}, children {}){extra}",
                node.class_id, node.depth, node.child_count
            );
        }

        // The sibling .rcsmodel, if the archive has it.
        if let Some(sibling) = oag_mesh::mesh::rcs::sibling_name(candidate) {
            match archive.read_path(&sibling) {
                Ok(rcs_blob) => {
                    println!("  sibling {sibling}: {} byte(s)", rcs_blob.len());
                    let is_psp2 = rcs_blob.len() >= 4
                        && u32::from_le_bytes(rcs_blob[0..4].try_into().expect("4 bytes"))
                            == psp2::MAGIC;
                    println!("  psp2 container: {is_psp2}");
                    if is_psp2 {
                        match psp2::parse(&rcs_blob) {
                            Ok(model) => {
                                println!(
                                    "  psp2::parse OK: {} submesh(es), {} material(s), {} unpaired pointer(s)",
                                    model.submeshes.len(),
                                    model.materials.len(),
                                    model.unpaired_pointers
                                );
                                for (i, sm) in model.submeshes.iter().enumerate() {
                                    println!(
                                        "    submesh {i}: {} tri, {} vert, stride {}, material {:?}, {} texcoord(s)",
                                        sm.triangle_count(),
                                        sm.positions.len(),
                                        sm.stride,
                                        sm.material,
                                        sm.texcoords.len()
                                    );
                                    let mut xs: Vec<f32> =
                                        sm.positions.iter().map(|p| p[0]).collect();
                                    xs.sort_by(f32::total_cmp);
                                    if let (Some(&min), Some(&max)) = (xs.first(), xs.last()) {
                                        println!("      x span: {min:.3}..{max:.3}");
                                    }
                                    let mut uvs: Vec<(String, usize)> = Vec::new();
                                    for uv in &sm.texcoords {
                                        let key = format!("{:.3},{:.3}", uv[0], uv[1]);
                                        match uvs.iter_mut().find(|(k, _)| *k == key) {
                                            Some((_, c)) => *c += 1,
                                            None => uvs.push((key, 1)),
                                        }
                                    }
                                    println!("      distinct uv count: {}", uvs.len());
                                    if uvs.len() <= 10 {
                                        println!("      uvs: {uvs:?}");
                                    }
                                }
                                for (i, m) in model.materials.iter().enumerate() {
                                    println!("    material {i}: {m:?}");
                                }
                            }
                            Err(e) => println!("  psp2::parse: ERROR {e}"),
                        }
                    } else {
                        println!(
                            "  first 32 bytes: {:02x?}",
                            &rcs_blob[..32.min(rcs_blob.len())]
                        );
                    }
                }
                Err(e) => println!("  sibling {sibling}: ERROR {e}"),
            }
        }
    }

    Ok(())
}
