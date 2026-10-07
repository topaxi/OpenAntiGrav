//! Dump: every drawn material slot's bound picture label, per circuit model,
//! one line each - run before and after a change to `skin::picks` and diff the
//! two files for the disc-wide reach.
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_pick_dump -- <image> /data/environments/01_vineta_k/track.vex ...
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
        let rcs = oag_rcs::rcsmodel::Model::parse(&geometry)?;
        let (model, _) = mesh::rcs::build_scene(&name, &data, &geometry, &mut |n| {
            mesh::read_blob(&spec, n).ok()
        })?;
        for (slot, material) in rcs.materials.iter().enumerate() {
            let label = model
                .textures
                .get(slot)
                .and_then(Option::as_ref)
                .map_or("-", |t| t.label.as_str());
            println!(
                "{name} {slot} {} {label}",
                material.name.rsplit('/').next().unwrap_or("")
            );
        }
    }
    Ok(())
}
