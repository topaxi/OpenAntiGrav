//! A track model's `mag*` material records: every sampler binding and every
//! authored parameter value, for reading what a magstrip surface is fed.
//!
//! ```sh
//! cargo run -p oag-render --example hd_mag_model [<image>:<archive> <model> [<filter>]]
//! ```

use oag_rcs::rcsmodel;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/talons_junction/track.rcsmodel".into());
    let want = args.next().unwrap_or_else(|| "mag".into());
    let blob = oag_mesh::mesh::read_blob(&spec, &name)?;
    let model = rcsmodel::Model::parse(&blob)?;
    for (slot, m) in model.materials.iter().enumerate() {
        if !m.name.contains(&want) {
            continue;
        }
        println!("slot {slot} {}", m.name);
        for (h, p) in &m.samplers {
            println!("  sampler {h:08x} {p:?}");
        }
        for p in &m.parameters {
            println!("  param {p:?}");
        }
    }
    Ok(())
}
