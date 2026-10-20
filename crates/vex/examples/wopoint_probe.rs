//! Scratch probe: `wopoint` (`0x3cf`) and `wospot` (`0x3ce`) nodes of a circuit,
//! world position and raw payload. `cargo run -q -p oag-vex --example wopoint_probe -- IMAGE TRACK [class]`
use oag_vex::vex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut archives = oag_pulse::open(&a[0])?;
    let blob = archives.read_name(&format!(r"Data\Environments\{}\track.vex", a[1]))?;
    let nodes = vex::nodes(&blob)?;
    let chain = vex::world_transforms(&blob, &nodes);
    let want: u32 = a.get(2).map_or(0x3cf, |s| {
        u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
    });
    for (i, n) in nodes.iter().enumerate() {
        if n.class_id != want {
            continue;
        }
        let m = chain[i];
        let p = &blob[n.payload()];
        println!(
            "node {i} pos [{:.3} {:.3} {:.3}] len {} payload {:02x?}",
            m[12],
            m[13],
            m[14],
            p.len(),
            p
        );
    }
    Ok(())
}
