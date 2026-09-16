//! Scratch probe for the PS4 Omega Collection lane: confirm `oag_vex::vex`
//! decodes an Omega `.vex` file unmodified (little-endian, auto-detected
//! from the file's own `VEXX`/`XXEV` magic) - see `docs/formats/vex.md`'s
//! "The Omega Collection's PS4 build" section.
//!
//! `cargo run -p oag-vex --example ps4_vex_probe -- <path.vex>`

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: ps4_vex_probe <path>")?;
    let data = std::fs::read(&path)?;
    let nodes = oag_vex::vex::nodes(&data).map_err(|e| format!("{e}"))?;
    println!("{path}: {} bytes, {} nodes", data.len(), nodes.len());
    for n in nodes.iter().take(10) {
        println!("  {:?}", n);
    }
    Ok(())
}
