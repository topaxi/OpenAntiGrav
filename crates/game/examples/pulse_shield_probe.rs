//! Scratch probe: which shield shells does a Pulse disc carry per team, and
//! what do they hold? Reproduces the PSP half of the "Concept craft's shield"
//! question in `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
//!
//! ```sh
//! cargo run -q -p oag-game --example pulse_shield_probe -- data/images/pulse-psp-usa.chd
//! ```

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/pulse-psp-usa.chd".to_string());
    let mut archives = oag_pulse::open(&path)?;
    const TEAMS: &[&str] = &[
        "AG_Systems",
        "Assegai",
        "EGX",
        "Feisar",
        "Goteki",
        "Piranha",
        "Qirex",
        "Triakis",
    ];
    for team in TEAMS {
        print!("{team}:");
        for stem in ["shipshield", "extrashield"] {
            let name = format!(r"Data\Ships\{team}\{stem}.vex");
            match archives.read_name(&name) {
                Ok(blob) => {
                    let text = String::from_utf8_lossy(&blob);
                    let textures: Vec<&str> = text
                        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
                        .filter(|word| word.ends_with(".tga"))
                        .collect();
                    print!("  {stem}.vex {} bytes {textures:?}", blob.len());
                }
                Err(_) => print!("  {stem}.vex absent"),
            }
        }
        println!();
    }
    Ok(())
}
