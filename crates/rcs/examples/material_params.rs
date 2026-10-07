//! Scratch probe: print the authored parameters of every material of an HD
//! `.rcsmodel` whose material file name contains the needle.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("rcsmodel path")?;
    let needle = std::env::args().nth(2).unwrap_or_default();
    let blob = std::fs::read(&path)?;
    let model = oag_rcs::rcsmodel::Model::parse(&blob).map_err(|e| format!("{e}"))?;
    for (index, material) in model.materials.iter().enumerate() {
        if !material.name.contains(&needle) {
            continue;
        }
        let params: Vec<String> = material
            .parameters
            .iter()
            .map(|p| format!("{:#010x}={:?}", p.hash, p.value))
            .collect();
        println!(
            "{index:4} {} first {:?} | {}",
            material.name.rsplit('/').next().unwrap_or(&material.name),
            material
                .texture
                .rsplit('/')
                .next()
                .unwrap_or(&material.texture),
            params.join(" ")
        );
    }
    Ok(())
}
