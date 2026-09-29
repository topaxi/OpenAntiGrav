//! Scratch probe for the Omega race lane: how much of 2048's `.rcsmodel`
//! reader ([`oag_rcs::rcsmodel::psp2`]) does a PS4 `.rcsmodel` satisfy?
//!
//! `cargo run -p oag-rcs --example omega_rcsmodel_probe -- <file.rcsmodel>...`

use oag_rcs::rcsmodel::psp2;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path)?;
        print!("{path}: {} bytes, ", data.len());
        match psp2::parse(&data) {
            Ok(m) => {
                let tris: usize = m.submeshes.iter().map(psp2::SubMesh::triangle_count).sum();
                let verts: usize = m.submeshes.iter().map(|s| s.positions.len()).sum();
                let normals: usize = m.submeshes.iter().map(|s| s.normals.len()).sum();
                let with_uv = m
                    .submeshes
                    .iter()
                    .filter(|s| !s.texcoords.is_empty())
                    .count();
                let with_tangent = m
                    .submeshes
                    .iter()
                    .filter(|s| !s.tangents.is_empty())
                    .count();
                let strides: std::collections::BTreeMap<usize, usize> =
                    m.submeshes.iter().fold(Default::default(), |mut acc, s| {
                        *acc.entry(s.stride).or_default() += 1;
                        acc
                    });
                println!();
                println!(
                    "  strides {strides:?}, {with_uv} submesh(es) with a uv set, {with_tangent} with a tangent"
                );
                println!(
                    "sections {:?}, {} submesh(es), {verts} vertices ({normals} normals), {tris} triangles, {} materials, {} unpaired pointers",
                    m.sections.iter().map(|s| s.len).collect::<Vec<_>>(),
                    m.submeshes.len(),
                    m.materials.len(),
                    m.unpaired_pointers
                );
            }
            Err(e) => println!("parse error: {e}"),
        }
    }
    Ok(())
}
