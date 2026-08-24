//! Scratch probe: the authored parameter block of one material slot - every
//! `vec4` its record carries, by name hash - so a value the renderer ignores
//! can be told from one the record never had.

use oag_render::mesh;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());
    let want = args.next().unwrap_or_default();

    let data = mesh::read_blob(&spec, &name)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("{name}: no sibling .rcsmodel"))?;
    let model = oag_formats::rcsmodel::Model::parse(&geometry)?;
    for (slot, material) in model.materials.iter().enumerate() {
        if !want.is_empty() && !material.name.contains(&want) && want != slot.to_string() {
            continue;
        }
        println!("slot {slot} {}", material.name);
        for (hash, path) in &material.samplers {
            println!("  sampler {hash:#010x} -> {path:?}");
        }
        for p in &material.parameters {
            println!(
                "  param   {:#010x} x{} = [{:.4}, {:.4}, {:.4}, {:.4}]",
                p.hash, p.quads, p.value[0], p.value[1], p.value[2], p.value[3]
            );
        }
    }
    Ok(())
}
