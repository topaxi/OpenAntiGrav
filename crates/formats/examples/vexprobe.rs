//! Development probe for the `.vex` decoder. Takes a path to a `.vex` file.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read(std::env::args().nth(1).unwrap())?;
    let nodes = oag_formats::vex::nodes(&data)?;

    let mut prims = std::collections::BTreeMap::new();
    let mut vtypes = std::collections::BTreeMap::new();
    let (mut batches, mut verts) = (0usize, 0usize);

    for m in nodes
        .iter()
        .filter(|n| n.class_id == oag_formats::vex::CLASS_MESH)
    {
        for list in 0..2u8 {
            for b in oag_formats::vex::mesh_batches(&data[m.payload()], list)? {
                *prims.entry(b.primitive_type).or_insert(0usize) += 1;
                *vtypes.entry(b.vertex_type).or_insert(0usize) += 1;
                batches += 1;
                verts += b.vertices.len();
            }
        }
    }
    println!("{batches} batches, {verts} vertices");
    println!("primitive types: {prims:?}");
    println!(
        "vertex types: {:?}",
        vtypes
            .iter()
            .map(|(k, v)| format!("{k:#06x}:{v}"))
            .collect::<Vec<_>>()
    );
    Ok(())
}
