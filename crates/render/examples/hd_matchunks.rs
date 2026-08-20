//! Scratch probe: chunk-per-material census of one or more `.rcsmodel` files
//! read straight off disk, as TSV.
//!
//! `model<TAB>chunk<TAB>hash<TAB>material<TAB>tris<TAB>verts<TAB>colourset<TAB>lightmapuv<TAB>tangent<TAB>transparency`
use oag_formats::rcsmodel;

const TANGENT: u32 = 0xdbe5_f417;

fn main() -> anyhow::Result<()> {
    println!("model\tchunk\thash\tmaterial\ttris\tverts\tcolour\tlmapuv\ttangent\ttransp");
    for path in std::env::args().skip(1) {
        let blob = std::fs::read(&path)?;
        let model = match rcsmodel::Model::parse(&blob) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{path}: {e}");
                continue;
            }
        };
        for (i, chunk) in model.meshes.iter().enumerate() {
            let mat = model
                .material_of(chunk)
                .map_or("<none>", |m| m.name.as_str());
            let tris: usize = chunk.submeshes.iter().map(|s| s.index_count / 3).sum();
            let verts: usize = chunk.submeshes.iter().map(|s| s.vertex_count).sum();
            let (colour, lmap, tangent) = chunk.decl.as_ref().map_or((0, 0, 0), |d| {
                (
                    i32::from(d.vertex_colour().is_some()),
                    i32::from(d.lightmap_texcoord().is_some()),
                    i32::from(d.attributes.iter().any(|a| a.name_hash == TANGENT)),
                )
            });
            let transp = model
                .material_of(chunk)
                .and_then(oag_formats::rcsmodel::Material::transparency)
                .map_or("?".into(), |t| format!("{t:?}"));
            println!(
                "{path}\t{i}\t{:#010x}\t{mat}\t{tris}\t{verts}\t{colour}\t{lmap}\t{tangent}\t{transp}",
                chunk.hash
            );
        }
    }
    Ok(())
}
