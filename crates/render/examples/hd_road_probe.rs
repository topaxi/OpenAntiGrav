//! Scratch: the specular exponent of every `track_surface` slot of a circuit.
use oag_mesh::mesh;
use oag_rcs::{rcsmaterial, rcsmodel};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let archive = args.next().unwrap();
    let name = args.next().unwrap();
    let needle = args.next().unwrap_or("track_surface".into());
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let spec = format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC");
    let data = mesh::read_blob(&spec, &name)?;
    let (model, _) = mesh::rcs::scene_from(&spec, &name, &data)?.unwrap();
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data).unwrap();
    let source = rcsmodel::Model::parse(&geometry)?;
    for (slot, variant) in model.material_variants.iter().enumerate() {
        let Some(variant) = variant else { continue };
        let Some(material) = source.materials.get(slot) else { continue };
        if !material.name.contains(&needle) { continue }
        let blob = mesh::read_blob(&spec, &format!("/{}", material.name))?;
        let Some(program) = rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset) else { continue };
        let exp = program.specular_exponent();
        let hash = program.specular_exponent_parameter();
        let val = hash.and_then(|h| material.parameters.iter().find(|p| p.hash == h)).map(|p| p.value[0]);
        println!("slot {slot} fragoff {:#x} {} exp {:?} patch {:?} value {:?} tex {:?}", variant.fragment.offset, material.name, exp, hash.map(|h| format!("{h:#010x}")), val, material.samplers.iter().filter_map(|(_,p)| p.clone()).collect::<Vec<_>>());
        for p in &material.parameters { println!("    param {:#010x} = {:?}", p.hash, p.value); }
    }
    Ok(())
}
