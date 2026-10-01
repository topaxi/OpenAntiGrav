//! Scratch probe: the nodes of a circuit's `start_grid.vex` (the grid-camera scene), with each
//! `Anim Transform`'s key counts and times, and the node's world matrix sampled over time.
//!
//! ```sh
//! cargo run -q -p oag-vex --example start_grid_probe -- data/images/pulse-psp-usa.chd 16 [CSV]
//! ```

use oag_vex::vex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let image = args.next().ok_or("usage: IMAGE CIRCUIT")?;
    let circuit: u32 = args.next().ok_or("usage: IMAGE CIRCUIT")?.parse()?;
    let mut archives = oag_pulse::open(&image)?;
    let name = std::env::var("START_GRID_NAME")
        .unwrap_or_else(|_| format!(r"Data\Environments\{circuit:02}_Track\start_grid.vex"));
    let blob = archives.read_name(&name)?;
    let nodes = vex::nodes(&blob)?;
    for (index, node) in nodes.iter().enumerate() {
        println!(
            "#{index} class {:#x} {:?} parent {:?} payload {} bytes",
            node.class_id,
            node.name,
            node.parent,
            blob[node.payload()].len()
        );
        println!("   attributes {:?}", vex::node_attributes(&blob, node));
        if node.class_id == 0x3c0 {
            let anim = vex::anim_transform_of(&blob, node).expect("decodes");
            println!(
                "   keys: t {:?} r {:?} s {:?} loop {}",
                anim.translation.times.len(),
                anim.rotation.times.len(),
                anim.scale.times.len(),
                anim.loop_seconds
            );
        }
        if node.class_id == 0x3dd || node.class_id == 0xf7 {
            let p = &blob[node.payload()];
            for chunk in p.chunks(16) {
                println!(
                    "   {}",
                    chunk
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
        }
    }
    if let Some(path) = args.next() {
        let camera = nodes
            .iter()
            .position(|n| n.class_id == 0x3dd)
            .ok_or("no gridCamera")?;
        let mut out = String::new();
        for frame in 0..=1500u32 {
            let world = vex::world_transforms_at(&blob, &nodes, frame as f32 / 60.0);
            let row: Vec<String> = world[camera].iter().map(|v| format!("{v:.6}")).collect();
            out.push_str(&format!("{frame},{}\n", row.join(",")));
        }
        std::fs::write(path, out)?;
    }
    Ok(())
}
