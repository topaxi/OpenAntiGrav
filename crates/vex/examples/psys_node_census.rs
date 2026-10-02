//! Scratch probe: every `.vex` entry on a Pulse source that places a
//! `ParticleSystem` (`0x3c4`) or `weatherPos` (`0x3da`) node, with the node's
//! name, attributes and payload head.
//!
//! ```sh
//! cargo run -q -p oag-vex --example psys_node_census -- data/images/pulse-psp-usa.chd
//! ```

use oag_vex::vex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image = std::env::args()
        .nth(1)
        .ok_or("usage: psys_node_census IMAGE")?;
    let mut archives = oag_pulse::open(&image)?;
    let wad = archives.data.as_wad_mut("data")?;
    let mut names = std::collections::BTreeMap::new();
    for n in 1..=32 {
        for file in [
            "track.vex",
            "track_reversed.vex",
            "zone_track.vex",
            "zone_track_reversed.vex",
        ] {
            let name = format!(r"Data\Environments\{n:02}_Track\{file}");
            if let Some(i) = wad.index_of_name(&name) {
                names.insert(i, name);
            }
        }
    }
    let mut hits = 0usize;
    let mut systems = Vec::new();
    for index in 0..wad.len() as usize {
        if let Ok(head) = wad.peek(index, 4)
            && oag_vex::pob::looks_like_particle_system(&head)
            && let Ok(blob) = wad.read(index)
            && let Ok(system) = oag_vex::pob::ParticleSystem::parse(&blob)
        {
            systems.push(system.name.clone());
        }
    }
    systems.sort();
    println!("{} systems: {}", systems.len(), systems.join(" "));
    for index in 0..wad.len() as usize {
        let Ok(blob) = wad.read(index) else { continue };
        let Ok(nodes) = vex::nodes(&blob) else {
            continue;
        };
        let world = vex::world_transforms(&blob, &nodes);
        for node in &nodes {
            if node.class_id != vex::CLASS_PARTICLE_SYSTEM && node.class_id != 0x3da {
                continue;
            }
            hits += 1;
            let payload = &blob[node.payload()];
            let parent = node.parent.and_then(|p| nodes[p].name.clone());
            let header = &blob[node.offset..node.offset + node.header_size];
            let ascii: String = header
                .iter()
                .map(|&b| {
                    if (0x20..0x7f).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            println!("  {:?} header {ascii}", names.get(&index));
            let mut chain = Vec::new();
            let mut at = node.parent;
            while let Some(p) = at {
                chain.push(format!(
                    "{:#x}:{}",
                    nodes[p].class_id,
                    nodes[p].name.as_deref().unwrap_or("?")
                ));
                at = nodes[p].parent;
            }
            println!("  chain {}", chain.join(" < "));
            let floats: Vec<f32> = payload
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| f32::from_le_bytes(*c))
                .collect();
            println!("  payload f32 {floats:?}");
            if floats.len() == 16 {
                let local: [f32; 16] = floats.clone().try_into().expect("16");
                let parent = node.parent.map_or(vex::IDENTITY, |p| world[p]);
                let w = vex::multiply(&local, &parent);
                println!(
                    "  world pos [{:.2}, {:.2}, {:.2}] up [{:.3}, {:.3}, {:.3}]",
                    w[12], w[13], w[14], w[4], w[5], w[6]
                );
            }
            println!(
                "entry {index} class {:#x} {:?} parent {:?} attrs {:?} payload {} bytes head {:02x?}",
                node.class_id,
                node.name,
                parent,
                vex::node_attributes(&blob, node),
                payload.len(),
                &payload[..payload.len().min(48)]
            );
        }
    }
    println!("{hits} node(s)");
    for name in [
        "WO_BLUE_WELDER",
        "WO_MODESTO_STEAM_A",
        "WO_RAIN",
        "WO_RAIN_LENS",
        "WO_SNOW",
    ] {
        let blob = archives.read_name(&format!(r"Data\Psys\{name}.POB"))?;
        let system = oag_vex::pob::ParticleSystem::parse(&blob)?;
        for e in system.emitters(&blob)? {
            println!(
                "{name}: emitter {:?} flags {:#x} looping {} duration {} interval {:?} per {:?} life {:?}",
                e.name,
                e.flags,
                e.flags & oag_vex::pob::flags::LOOPING != 0,
                e.duration_ticks,
                e.interval_ticks,
                e.per_emission,
                e.lifetime_ticks
            );
        }
    }
    Ok(())
}
