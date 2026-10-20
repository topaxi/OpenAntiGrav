//! Scratch probe: does a Pure circuit author `wopoint`-shaped nodes (a 0x20-byte
//! payload: colour, a one, inner radius, outer radius, two zeros), and how many
//! nodes of each class id does the file carry?
//!
//! ```sh
//! cargo run -q -p oag-pure --example pure_light_nodes -- data/images/pure-psp-usa.chd
//! ```
use oag_vex::vex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image = std::env::args().nth(1).ok_or("usage: IMAGE")?;
    let mut archives = oag_pure::open(&image)?;
    let wad = archives.data.as_wad_mut("data")?;
    for index in 0..wad.len() as usize {
        let Ok(head) = wad.peek(index, 16) else {
            continue;
        };
        if head.get(12..16) != Some(b"VEXX".as_slice())
            && head.get(12..16) != Some(b"XXEV".as_slice())
        {
            continue;
        }
        let Ok(blob) = wad.read(index) else { continue };
        let Ok(nodes) = vex::nodes(&blob) else {
            continue;
        };
        let order = vex::byte_order(&blob);
        let mut hist = std::collections::BTreeMap::<u32, usize>::new();
        let mut shaped = std::collections::BTreeMap::<u32, usize>::new();
        for n in &nodes {
            *hist.entry(n.class_id).or_default() += 1;
            let range = n.payload();
            let Some(p) = blob.get(range) else { continue };
            if p.len() == 0x20 {
                let f = |at: usize| f32::from_bits(order.u32(p, at));
                if f(0x10) > 0.0
                    && f(0x14) > f(0x10)
                    && f(0x14) < 500.0
                    && (0.0..=2.0).contains(&f(0))
                    && f(0xc) == 1.0
                {
                    *shaped.entry(n.class_id).or_default() += 1;
                }
            }
        }
        if !shaped.is_empty() {
            println!(
                "entry {index}: wopoint-shaped by class {shaped:?} ({} nodes)",
                nodes.len()
            );
        }
    }
    Ok(())
}
