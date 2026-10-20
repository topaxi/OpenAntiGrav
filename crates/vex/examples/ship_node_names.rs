//! Scratch probe: every node of a ship's `Ship.vex` with its class and name.
//!
//! ```sh
//! cargo run -q -p oag-vex --example ship_node_names -- data/images/pulse-ps2-eu.chd Feisar
//! ```
use oag_vex::vex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let image = args.next().ok_or("usage: IMAGE TEAM")?;
    let team = args.next().ok_or("team")?;
    let mut archives = oag_pulse::open(&image)?;
    let name = if team.ends_with(".vex") {
        team.clone()
    } else {
        format!(r"Data\Ships\{team}\Ship.vex")
    };
    let blob = archives.read_name(&name)?;
    let nodes = vex::nodes(&blob)?;
    for (i, n) in nodes.iter().enumerate() {
        println!(
            "{i:3} class {:#x} name {:?} payload {}",
            n.class_id,
            n.name,
            blob.get(n.payload()).map_or(0, <[u8]>::len)
        );
    }
    for (i, a) in vex::anim_transforms(&blob, &nodes).iter().enumerate() {
        let Some(a) = a else { continue };
        println!(
            "anim node {i} {:?}: loop {}s, {} s per key",
            nodes[i].name, a.loop_seconds, a.seconds_per_key
        );
        let mut t = 0.0f32;
        while t < a.loop_seconds.min(8.0) {
            let m = a.sample(t);
            println!(
                "  t {t:.2} translation [{:.3} {:.3} {:.3}] scale-ish [{:.3} {:.3} {:.3}]",
                m[12], m[13], m[14], m[0], m[5], m[10]
            );
            t += 0.25;
        }
    }
    Ok(())
}
