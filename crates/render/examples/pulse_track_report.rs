//! Scratch probe: what a Pulse `.vex` holds against what the renderer emits
//! from it - node classes, batches, triangles and draw calls - so "meshes are
//! missing" can be answered with a count before it is answered with a picture.

use oag_mesh::mesh;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/pulse-psp-eu.chd".into());
    let name = args
        .next()
        .unwrap_or_else(|| r"Data\Environments\16_Track\track.vex".into());

    let data = oag_assets::Container::open(&spec)
        .and_then(|mut c| c.read_entry(&name))
        .or_else(|_| {
            oag_assets::Container::open(&format!("{spec}:PSP_GAME/USRDIR/Data.wad"))
                .and_then(|mut c| c.read_entry(&name))
        })?;
    let nodes = oag_vex::vex::nodes(&data)?;

    let mut by_class: std::collections::BTreeMap<u32, usize> = Default::default();
    for node in &nodes {
        *by_class.entry(node.class_id).or_default() += 1;
    }
    println!("{name}: {} node(s)", nodes.len());
    for (class, count) in &by_class {
        println!("  class {class:#06x}  {count:>5}");
    }

    let model = mesh::build(&name, &data)?;
    let triangles: usize = model.indices.len() / 3;
    println!(
        "built: {} vertices, {triangles} triangle(s), {} opaque + {} cutout + {} blended draw(s)",
        model.vertices.len(),
        model.draws.len(),
        model.alpha_tested_draws.len(),
        model.transparent_draws.len()
    );
    println!(
        "textures: {} slot(s), {} decoded",
        model.textures.len(),
        model.textures.iter().filter(|t| t.is_some()).count()
    );
    Ok(())
}
