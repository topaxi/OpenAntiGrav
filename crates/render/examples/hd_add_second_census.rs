//! Census: per circuit, how many vertices carry [`slots::ADD_SECOND`] and how
//! many distinct emissive-table entries the build made - the reach of a change
//! to `Program::accumulates`, run before and after it.
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_add_second_census -- \
//!     data/images/hdfury-ps3-eu-dec.iso /data/environments/01_vineta_k/track.vex ...
//! ```

use oag_mesh::mesh::{self, slots};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args.next().unwrap();
    let (mut vertices_total, mut materials_total) = (0usize, 0usize);
    for name in args {
        let Some((spec, data)) = (0..4).find_map(|n| {
            let spec = format!("{image}:PS3_GAME/USRDIR/DATA0{n}.PSARC");
            mesh::read_blob(&spec, &name).ok().map(|d| (spec, d))
        }) else {
            continue;
        };
        let Some((model, _)) = mesh::rcs::scene_from(&spec, &name, &data)? else {
            continue;
        };
        let added: std::collections::BTreeSet<u32> = model
            .vertices
            .iter()
            .filter(|v| v.slots & slots::ADD_SECOND != 0)
            .map(|v| v.slots)
            .collect();
        let vertices = model
            .vertices
            .iter()
            .filter(|v| v.slots & slots::ADD_SECOND != 0)
            .count();
        println!(
            "{name}: {vertices} vertices, {} distinct role words",
            added.len()
        );
        vertices_total += vertices;
        materials_total += added.len();
    }
    println!("total: {vertices_total} vertices, {materials_total} distinct role words");
    Ok(())
}
