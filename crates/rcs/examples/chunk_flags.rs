//! Scratch probe: per chunk of an HD `.rcsmodel`, its material's first texture
//! and its render-block flags halfword, for correlating a render-state split
//! (which fog pair a draw used) against the data.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("rcsmodel path")?;
    let blob = std::fs::read(&path)?;
    let model = oag_rcs::rcsmodel::Model::parse(&blob).map_err(|e| format!("{e}"))?;
    for (index, mesh) in model.meshes.iter().enumerate() {
        let material = model.materials.get(mesh.material as usize);
        let tex = material.map_or("-", |m| m.texture.rsplit('/').next().unwrap_or(&m.texture));
        println!(
            "{index}\t{tex}\t{:#06x}\t{}",
            mesh.render_flags,
            material.map_or("-", |m| m.name.rsplit('/').next().unwrap_or(&m.name))
        );
    }
    Ok(())
}
