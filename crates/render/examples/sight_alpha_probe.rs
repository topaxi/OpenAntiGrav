//! Scratch probe: what the three lock-on sight models actually declare.
//!
//! Prints each batch's `pass_mask` and its decoded blend class, then an alpha
//! census of the embedded texture, for `missile_sight_outer.vex`,
//! `missile_sight_inner.vex` and `leachbeam_sight.vex`. Run by hand while
//! working out why the reticle draws on a black square. See
//! `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`.
//!
//! ```sh
//! cargo run -p oag-render --example sight_alpha_probe
//! ```

use oag_formats::vex;

const ENTRIES: &[&str] = &[
    "Data\\HUD\\missile_sight_outer.vex",
    "Data\\HUD\\missile_sight_inner.vex",
    "Data\\HUD\\leachbeam_sight.vex",
    "Data\\HUD\\Pulse_Ready_Go.vex",
    "Data\\HUD\\Cockpit_321GO.vex",
];

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/pulse-psp-usa.chd".into());
    let spec = format!("{image}:PSP_GAME/USRDIR/Data.wad");
    let mut archive = oag_assets::Archive::open(&spec)?;

    for entry in ENTRIES {
        println!("== {entry} ==");
        let blob = match archive.read_name(entry) {
            Ok(blob) => blob,
            Err(why) => {
                println!("  unavailable: {why}");
                continue;
            }
        };

        let nodes = vex::nodes(&blob)?;
        for node in vex::nodes_by_class(&nodes, vex::CLASS_MESH) {
            let payload = &blob[node.payload()];
            for batch_list in [0u8, 1u8] {
                let batches = match vex::mesh_batches(payload, batch_list) {
                    Ok(b) => b,
                    Err(why) => {
                        println!("  batch list {batch_list}: {why}");
                        continue;
                    }
                };
                for (i, batch) in batches.iter().enumerate() {
                    println!(
                        "  mesh {:?} list {batch_list} batch {i}: pass_mask 0x{:04x} \
                         transparent {} blend {:?} alpha_tested {} culled {} \
                         header_flags 0x{:02x} additive_hdr {} verts {} prim {}",
                        node.name,
                        batch.pass_mask,
                        batch.is_transparent(),
                        batch.blend_class(),
                        batch.is_alpha_tested(),
                        batch.is_culled(),
                        batch.header_flags,
                        batch.is_additive_blend(),
                        batch.vertices.len(),
                        batch.primitive_type,
                    );
                    let colours: Vec<String> = batch
                        .vertices
                        .iter()
                        .map(|v| match v.colour {
                            Some(c) => {
                                format!("{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3])
                            }
                            None => "-".into(),
                        })
                        .collect();
                    println!("    vertex colours: {}", colours.join(" "));
                }
            }
        }

        for texture in vex::textures(&blob)?.into_iter().flatten() {
            let rgba = texture.to_rgba();
            let mut histogram = std::collections::BTreeMap::new();
            for texel in rgba.as_chunks::<4>().0 {
                *histogram.entry(texel[3]).or_insert(0usize) += 1;
            }
            println!(
                "  texture {:?} {}x{} bpp {} mips {}",
                texture.asset_path,
                texture.width,
                texture.height,
                texture.bits_per_pixel,
                texture.mip_count
            );
            println!("    alpha histogram: {histogram:?}");
            let mut rgb = std::collections::BTreeMap::new();
            for texel in rgba.as_chunks::<4>().0 {
                *rgb.entry((texel[0], texel[1], texel[2])).or_insert(0usize) += 1;
            }
            println!("    rgb histogram: {rgb:?}");
        }
    }
    Ok(())
}
