//! Scratch probe: the scene loader's own report for an HD circuit.

use oag_mesh::mesh;

fn main() -> anyhow::Result<()> {
    let spec = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());
    let data = mesh::read_blob(&spec, &name)?;
    let Some((model, report)) = mesh::rcs::scene_from(&spec, &name, &data)? else {
        anyhow::bail!("no PS3 sibling geometry");
    };
    println!("{}", report.describe());
    println!(
        "{} draw calls, {} vertices",
        model.draws.len(),
        model.vertices.len()
    );
    Ok(())
}
