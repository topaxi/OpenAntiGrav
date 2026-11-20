use std::path::Path;
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut ar = oag_assets::psarc::Archive::open_file(Path::new(&args[0])).unwrap();
    let blob = ar.read_path(&args[1]).unwrap();
    let m = oag_rcs::rcsmodel::psp2::parse(&blob).unwrap();
    let mut shown = 0; let mut names: Vec<String> = Vec::new();
    let mut with_node = 0;
    let mut total = 0;
    for (mi, mo) in m.scene.meshes.iter().enumerate() {
        if !mo.is_crowd_entity() { continue; }
        total += 1; names.push(mo.name.clone());
        with_node += usize::from(mo.node.is_some());
        if shown < 0 {
            shown += 1;
            for r in &mo.submesh_records {
                if let Some(s) = m.submeshes.iter().find(|s| s.record == *r) {
                    let mut lo=[f32::MAX;3]; let mut hi=[f32::MIN;3];
                    for p in &s.positions { for k in 0..3 { lo[k]=lo[k].min(p[k]); hi[k]=hi[k].max(p[k]); } }
                    println!("mesh {mi} {} node={:?} flags={:#x} verts={} tris={} lo={:?} hi={:?}", mo.name, mo.node, mo.flags, s.positions.len(), s.indices.len()/3, lo, hi);
                }
            }
            if let Some(n) = mo.node { let nd=&m.scene.nodes[n]; println!("   node {n} bind={:?}", nd.bind); }
        }
    }
    for n in &names { println!("NAME {n}"); }
    println!("entities {total} with node {with_node}");
}
