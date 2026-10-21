//! Scratch probe: the `paraboloidReflectionTex` map `oag_mesh::mesh::sky_paraboloid` builds for a circuit's
//! `sky.gtf`, written as raw `RGBA8` (512x256).
use oag_assets::Container;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args.next().ok_or_else(|| anyhow::anyhow!("no spec"))?;
    let entry = args.next().ok_or_else(|| anyhow::anyhow!("no entry"))?;
    let out = args.next().ok_or_else(|| anyhow::anyhow!("no output"))?;
    let blob = Container::open(&spec)?.read_entry(&entry)?;
    let sky = oag_mesh::mesh::sky_cube::build("sky", &blob, 0.0, [1.0; 3], 0.0)?;
    let map = oag_mesh::mesh::sky_paraboloid::from_sky(&sky)
        .ok_or_else(|| anyhow::anyhow!("not a cube"))?;
    std::fs::write(out, map)?;
    Ok(())
}
