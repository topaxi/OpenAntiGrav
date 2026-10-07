//! Census: per circuit model, how many material slots are
//! `diffuse_normal_specular_emmissive` and how many `_ne` masks the scene
//! binds, the reach of binding that family (`rcsmaterial.md`, "Vineta K
//! against a draw capture").
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_light_bar_census -- <image> /data/environments/01_vineta_k/track.vex ...
//! ```

use oag_mesh::mesh;
use oag_rcs::rcsmodel;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args.next().ok_or_else(|| anyhow::anyhow!("image path"))?;
    for name in args {
        let Some((spec, data)) = (0..4).find_map(|n| {
            let spec = format!("{image}:PS3_GAME/USRDIR/DATA0{n}.PSARC");
            mesh::read_blob(&spec, &name).ok().map(|d| (spec, d))
        }) else {
            println!("{name}\tnot found");
            continue;
        };
        let Some(geometry) = mesh::rcs::sibling_geometry(&spec, &name, &data) else {
            continue;
        };
        let model = rcsmodel::Model::parse(&geometry)?;
        let family = model
            .materials
            .iter()
            .filter(|m| {
                m.name
                    .ends_with("diffuse_normal_specular_emmissive.rcsmaterial")
            })
            .count();
        let mut read = |p: &str| {
            (0..4).find_map(|n| {
                mesh::read_blob(&format!("{image}:PS3_GAME/USRDIR/DATA0{n}.PSARC"), p).ok()
            })
        };
        let (_, report) = mesh::rcs::build_scene(&name, &data, &geometry, &mut read)?;
        println!(
            "{name}\tfamily slots {family}\tpad_ne bound {}\tunread {}",
            report.pad_ne_bound, report.pad_ne_unread
        );
    }
    Ok(())
}
