fn main() -> anyhow::Result<()> {
    let mut archive =
        oag_assets::psarc::Archive::open("data/extracted/vita/PCSF00007/base/PSP2/data.psarc")?;
    for team in [
        "ag_systems2048",
        "auricom2048",
        "feisar2048",
        "piranha2048",
        "qirex2048",
    ] {
        for n in 1..=4 {
            let entry = format!("Data\\HandlingStats\\{team}\\{n}\\handlingstats.xml");
            let Ok(blob) = archive.read_path(&entry.replace('\\', "/")) else {
                println!("{team}/{n}: no handling stats");
                continue;
            };
            let text = String::from_utf8_lossy(&blob);
            let head: String = text.lines().take(4).collect::<Vec<_>>().join(" ");
            let stats = oag_formats::handling::from_blob(&blob);
            println!(
                "{team}/{n}: {} | {}",
                match &stats {
                    Ok(s) => format!("{} class(es)", s.classes.len()),
                    Err(e) => format!("ERROR {e}"),
                },
                head.trim().chars().take(120).collect::<String>()
            );
            let ship = format!("Data\\art\\published\\Ships\\{team}\\{n}\\Ship.vex");
            let rcs = format!("Data\\art\\published\\Ships\\{team}\\{n}\\ship.rcsmodel");
            println!(
                "    Ship.vex {} | ship.rcsmodel {}",
                archive.contains(&ship.replace('\\', "/")),
                archive.contains(&rcs.replace('\\', "/"))
            );
        }
    }
    Ok(())
}
