//! Scratch probe: print one material row (or all rows matching a needle) of an
//! HD `.rcsmodel`'s material table.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("rcsmodel path")?;
    let needle = std::env::args().nth(2).unwrap_or_default();
    let blob = std::fs::read(&path)?;
    let model = oag_rcs::rcsmodel::Model::parse(&blob).map_err(|e| format!("{e}"))?;
    for (index, material) in model.materials.iter().enumerate() {
        let hit = index.to_string() == needle
            || (!needle.is_empty() && material.texture.contains(&needle));
        if needle.is_empty() || hit {
            println!(
                "{index:4}  {}  first {:?}  second {:?}  state {:#06x?}",
                material.name, material.texture, material.second_texture, material.state,
            );
        }
    }
    Ok(())
}
