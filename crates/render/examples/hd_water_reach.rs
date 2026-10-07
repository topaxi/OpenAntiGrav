//! Census: the water-family counts the loader report prints for each circuit.
//!
//! ```sh
//! cargo run -p oag-render --example hd_water_reach -- <image> /data/environments/01_vineta_k/track.vex ...
//! ```

use oag_mesh::mesh;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args.next().unwrap();
    for name in args {
        let Some((spec, data)) = (0..4).find_map(|n| {
            let spec = format!("{image}:PS3_GAME/USRDIR/DATA0{n}.PSARC");
            mesh::read_blob(&spec, &name).ok().map(|d| (spec, d))
        }) else {
            continue;
        };
        let Some(geometry) = mesh::rcs::sibling_geometry(&spec, &name, &data) else {
            continue;
        };
        let _ = spec;
        let (_, report) = mesh::rcs::build_scene(&name, &data, &geometry, &mut |n| {
            (0..4).find_map(|k| {
                mesh::read_blob(&format!("{image}:PS3_GAME/USRDIR/DATA0{k}.PSARC"), n).ok()
            })
        })?;
        let text = report.describe();
        let water: Vec<&str> = text
            .split(", ")
            .filter(|part| part.contains("water material") || part.contains("ice material"))
            .collect();
        println!(
            "{name}: {}",
            if water.is_empty() {
                "-".to_string()
            } else {
                water.join("; ")
            }
        );
    }
    Ok(())
}
