//! Scratch probe, the reproducer for `oag_pulse::race::HULL_VARIANTS`: does
//! Pulse's own `Definition.xml` declare `PI_TeamModel`/`PI_ModelSkin` for a
//! disc team, the way [`dlc-pack.md`](../../../docs/formats/dlc-pack.md)'s
//! schema (measured off a DLC pack's manifest) says it can - and does the
//! same axis exist on Wipeout Pure?
//!
//! **Deliberately prints no `Unlock` loyalty numbers** - shipped tuning data,
//! which `dlc-pack.md` already declines to reproduce for the same reason.
//! Only structure and file presence are reported.
//!
//! ```sh
//! cargo run -q -p oag-game --example pulse_variant_probe -- data/images/pulse-psp-usa.chd
//! cargo run -q -p oag-game --example pulse_variant_probe -- data/images/pure-psp-usa.chd
//! ```

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/pulse-psp-usa.chd".to_string());
    let mut archives = oag_pulse::open(&path).or_else(|_| oag_pure::open(&path))?;

    let blob = archives.read_name(r"Data\Plugins\PI001\Definition.xml")?;
    let xml = oag_formats::fexml::text(&blob)?;
    println!(
        "{path}: PI_TeamModel {}, PI_ModelSkin {}",
        xml.matches("PI_TeamModel").count(),
        xml.matches("PI_ModelSkin").count(),
    );

    // Every base team this build knows the id of, on either title - absent
    // ones are skipped rather than failing the probe.
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
        for stem in ["Ship", "extra", "zone01", "Zone"] {
            let name = format!(r"Data\Ships\{team}\{stem}.vex");
            if archives.read_name(&name).is_ok() {
                print!("  {team}/{stem}.vex");
            }
        }
        println!();
    }
    Ok(())
}
