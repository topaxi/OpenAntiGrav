//! Scratch probe: which batches of a ship's hull carry the `0x2000` extra-pass
//! bit, what their materials' second texture is, and how each is blended.
//!
//! ```sh
//! cargo run -q -p oag-vex --example ship_shine_probe -- data/images/pulse-psp-usa.chd Assegai
//! cargo run -q -p oag-vex --example ship_shine_probe -- data/images/pulse-psp-usa.chd 'Data\Environments\03_Track\track.vex'
//! ```

use oag_vex::vex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let image = args.next().ok_or("usage: ship_shine_probe IMAGE TEAM")?;
    let team = args.next().unwrap_or_else(|| "Assegai".to_string());
    let mut archives = oag_pulse::open(&image)?;
    let blob = if team.ends_with(".vex") {
        archives.read_name(&team)?
    } else {
        archives.read_name(&format!(r"Data\Ships\{team}\Ship.vex"))?
    };
    let nodes = vex::nodes(&blob)?;
    let textures = vex::textures(&blob)?;
    for (i, t) in textures.iter().enumerate() {
        match t {
            Some(t) => println!(
                "texture {i}: {}x{} {}bpp mips {} {:?}",
                t.width, t.height, t.bits_per_pixel, t.mip_count, t.asset_path
            ),
            None => println!("texture {i}: undecoded"),
        }
    }
    let texture_filter = std::env::var("PROBE_TEXTURE").unwrap_or_else(|_| "envtest".to_string());
    if let Some(out) = args.next() {
        let second = textures
            .iter()
            .flatten()
            .find(|t| {
                t.asset_path
                    .as_deref()
                    .is_some_and(|p| p.contains(texture_filter.as_str()))
            })
            .ok_or("no texture matches PROBE_TEXTURE (default envtest)")?;
        let rgba = second.to_rgba();
        let mut ppm = format!("P6 {} {} 255\n", second.width, second.height).into_bytes();
        for px in rgba.chunks(4) {
            ppm.extend_from_slice(&px[..3]);
        }
        std::fs::write(out, ppm)?;
    }
    let classes = vex::classes_of(&blob)?;
    for (index, node) in nodes.iter().enumerate() {
        if Some(node.class_id) != classes.mesh {
            continue;
        }
        let payload = &blob[node.payload()];
        let materials = vex::mesh_materials(payload);
        println!(
            "mesh node {index} {:?} flags {:#06x}",
            node.name,
            u16::from_le_bytes([payload[0], payload[1]])
        );
        for (m, material) in materials.iter().enumerate() {
            println!("  material {m}: {material:?}");
        }
        for list in [0u8, 1u8] {
            for batch in vex::mesh_batches(payload, list)? {
                println!(
                    "  list {list} batch: pass_mask {:#06x} hdr {:#04x} material {} verts {} prim {} vtype {:#x}",
                    batch.pass_mask,
                    batch.header_flags,
                    batch.material_index,
                    batch.vertices.len(),
                    batch.primitive_type,
                    batch.vertex_type
                );
            }
        }
    }
    Ok(())
}
