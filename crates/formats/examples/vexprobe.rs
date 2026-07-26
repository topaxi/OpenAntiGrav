//! Temporary probe: validates the .vex decoder against a real model.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).unwrap();
    let data = std::fs::read(&path)?;
    let tree = oag_formats::vex::tree_len(&data)?;
    let tex = oag_formats::vex::texture_len(&data)?;
    println!(
        "{} bytes: header 16 + tree {tree} + textures {tex} = {} (magic {})",
        data.len(),
        16 + tree + tex,
        oag_formats::vex::has_magic(&data)
    );

    let nodes = oag_formats::vex::nodes(&data)?;
    println!("{} nodes", nodes.len());
    for n in nodes.iter().take(6) {
        println!(
            "  {:indent$}class {:#05x} {:?} children={}",
            "", n.class_id, n.name, n.child_count, indent = n.depth * 2
        );
    }

    let mut worst: f32 = 0.0;
    let mut checked = 0;
    for m in nodes.iter().filter(|n| n.class_id == oag_formats::vex::CLASS_MESH) {
        let payload = &data[m.payload()];
        // Mesh header carries an f32 bbox in model units at +0x10 and +0x20.
        let f32_at = |o: usize| {
            f32::from_bits(u32::from_le_bytes(payload[o..o + 4].try_into().unwrap()))
        };
        let declared_min = [f32_at(0x10), f32_at(0x14), f32_at(0x18)];
        let declared_max = [f32_at(0x20), f32_at(0x24), f32_at(0x28)];

        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        let mut any = false;
        for list in 0..2u8 {
            for b in oag_formats::vex::mesh_batches(payload, list)? {
                for v in &b.vertices {
                    any = true;
                    for i in 0..3 {
                        lo[i] = lo[i].min(v.position[i]);
                        hi[i] = hi[i].max(v.position[i]);
                    }
                }
            }
        }
        if !any { continue; }
        checked += 1;
        let extent = (0..3).map(|i| declared_max[i] - declared_min[i]).fold(0.0f32, f32::max);
        for i in 0..3 {
            let e = ((lo[i] - declared_min[i]).abs()).max((hi[i] - declared_max[i]).abs());
            worst = worst.max(e / extent.max(1e-6));
        }
        if checked <= 2 {
            println!("  mesh bbox declared {declared_min:?}..{declared_max:?}");
            println!("            decoded  {lo:?}..{hi:?}");
        }
    }
    println!("\n{checked} meshes checked, worst bbox error {:.4}% of extent", worst * 100.0);
    Ok(())
}
