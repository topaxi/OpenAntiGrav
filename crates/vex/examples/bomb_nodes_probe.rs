//! Scratch probe: list every node of a weapon `.vex` with its class, parent and
//! mesh triangle count.
//!
//! ```sh
//! cargo run -q -p oag-vex --example bomb_nodes_probe -- data/images/pulse-psp-usa.chd 'Data\Weapons\Pulse_Bomb.vex'
//! ```

use oag_vex::vex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let image = args.next().ok_or("usage: IMAGE ENTRY")?;
    let name = args.next().ok_or("usage: IMAGE ENTRY")?;
    let mut archives = oag_pulse::open(&image)?;
    let blob = archives.read_name(&name)?;
    let nodes = vex::nodes(&blob)?;
    for (index, node) in nodes.iter().enumerate() {
        let tris: usize = if node.class_id == vex::CLASS_MESH {
            vex::mesh_batches(&blob[node.payload()], 0)
                .map(|b| b.iter().map(|b| b.triangles().len()).sum())
                .unwrap_or(0)
        } else {
            0
        };
        println!(
            "#{index} class {:#x} {:?} parent {:?} payload {} bytes tris {tris}",
            node.class_id,
            node.name,
            node.parent,
            blob[node.payload()].len()
        );
        if node.class_id == 0x3c0 {
            let anim = vex::anim_transform_of(&blob, node).expect("decodes");
            println!("   {anim:?}");
            for k in 0..8 {
                let t = k as f32 * 0.25;
                println!("   t={t:.2} {:?}", anim.sample(t));
            }
        }
    }
    Ok(())
}
