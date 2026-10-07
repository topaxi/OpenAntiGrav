//! Scratch probe: the vertex declaration of every chunk drawn with a given
//! material slot, and what `diffuse_texcoord` answers for it.
//!
//! ```sh
//! cargo run -p oag-render --example hd_water_decl -- <image> <track.vex> <slot>...
//! ```

use oag_mesh::mesh;
use oag_rcs::rcsmodel;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args.next().unwrap();
    let name = args.next().unwrap();
    let wanted: Vec<u32> = args.filter_map(|a| a.parse().ok()).collect();
    let (spec, data) = (0..4)
        .find_map(|n| {
            let spec = format!("{image}:PS3_GAME/USRDIR/DATA0{n}.PSARC");
            mesh::read_blob(&spec, &name).ok().map(|d| (spec, d))
        })
        .ok_or_else(|| anyhow::anyhow!("no such track"))?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("no geometry"))?;
    let model = rcsmodel::Model::parse(&geometry)?;
    for (i, chunk) in model.meshes.iter().enumerate() {
        for surface in chunk.surfaces() {
            if !wanted.contains(&surface.material) {
                continue;
            }
            let Some(decl) = surface.decl.as_ref() else {
                println!("chunk {i} material {} inline layout", surface.material);
                continue;
            };
            let attrs: Vec<String> = decl
                .attributes
                .iter()
                .map(|a| {
                    format!(
                        "{:08x}{}:{}x{}@{}",
                        a.name_hash,
                        a.name().map(|n| format!("({n})")).unwrap_or_default(),
                        a.rsx_type,
                        a.components,
                        a.offset
                    )
                })
                .collect();
            println!(
                "chunk {i} material {} stride {} diffuse_texcoord {:?} [{}]",
                surface.material,
                decl.stride,
                decl.diffuse_texcoord().map(|a| a.name_hash),
                attrs.join(" ")
            );
        }
    }
    Ok(())
}
