//! Scratch probe: does Omega's base package carry the front-end flyer assets
//! `Grid Selection` would draw, and in which form?
//!
//! ```sh
//! cargo run -p oag-game --example omega_flyer_probe -- \
//!     data/extracted/ps4/omega-eu/uroot/data00.psarc
//! ```

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("a .psarc path");
    let mut archive = oag_assets::psarc::Archive::open_file(std::path::Path::new(&path))?;
    let flyers: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().contains("/fe/flyers/"))
        .cloned()
        .collect();
    println!("{} flyer entries", flyers.len());
    for name in flyers
        .iter()
        .filter(|p| p.to_ascii_lowercase().contains("01_uplift"))
    {
        let size = archive.read_path(name).map(|b| b.len()).unwrap_or(0);
        println!("  {name} ({size} bytes)");
    }
    for name in flyers.iter().take(6) {
        println!("  e.g. {name}");
    }
    // The magic of the first model found.
    for name in flyers
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with("flyer.rcsmodel"))
        .take(1)
    {
        let bytes = archive.read_path(name)?;
        println!(
            "{name}: first 16 bytes {:02x?}",
            &bytes[..16.min(bytes.len())]
        );
    }
    for name in flyers
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with("flyer.vex"))
        .take(1)
    {
        let bytes = archive.read_path(name)?;
        println!(
            "{name}: first 16 bytes {:02x?}",
            &bytes[..16.min(bytes.len())]
        );
    }
    // And through the game's own preview path, which is what a flyer would
    // be drawn from.
    if let Some(source) = std::env::args().nth(2) {
        let mut opened = oag_game::title::open_source(&source, Vec::new(), Vec::new())?;
        for entry in [
            "Data/fe/flyers/01_uplift/flyer.vex",
            "Data/FE/Flyers/01_uplift/flyer.vex",
        ] {
            match oag_game::preview::model(&mut opened.archives, entry) {
                Ok(model) => println!(
                    "{entry}: built, {} vertices, {} triangles, {} anim nodes",
                    model.vertices.len(),
                    model.indices.len() / 3,
                    model.anim_nodes.len()
                ),
                Err(error) => println!("{entry}: {error:#}"),
            }
        }
    }
    Ok(())
}
