//! Scratch probe for the Omega race lane: does an Omega `track.vex` carry a
//! `WO Track` node this crate's own reader accepts, and does the arithmetic
//! close on the payload's own length?
//!
//! `cargo run -p oag-vex --example omega_track_probe -- <track.vex>...`

use oag_vex::{track, vex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path)?;
        let version = vex::version(&data).map_err(|e| format!("{e}"))?;
        let nodes = vex::nodes(&data).map_err(|e| format!("{e}"))?;
        let classes = vex::classes_of(&data).map_err(|e| format!("{e}"))?;
        println!(
            "{path}: {} bytes, vex version {version}, {} nodes, wo_track class {:?}",
            data.len(),
            nodes.len(),
            classes.wo_track
        );
        let Some(node) = track::find_node(&data, &nodes) else {
            println!("  no WO Track node");
            continue;
        };
        let payload = &data[node.payload()];
        println!(
            "  WO Track node at {}, payload {} bytes, magic order {:?}",
            node.offset,
            payload.len(),
            track::byte_order(payload)
        );
        match track::parse(payload) {
            Ok(ai) => println!(
                "  parsed: {} paths, {} junctions, {} points, encoded_len {} (payload {})",
                ai.paths.len(),
                ai.junctions.len(),
                ai.point_count(),
                ai.encoded_len(),
                payload.len()
            ),
            Err(e) => println!("  parse error: {e}"),
        }
    }
    Ok(())
}
