//! Scratch probe: the connected pieces of every `emissive_bloom` chunk of a ship's `.rcsmodel`,
//! with each piece's bounding box and `Uv1` range - which piece is the nozzle's "jewel".
//!
//! ```sh
//! cargo run -p oag-render --example hd_hull_glow_parts -- data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA03.PSARC /data/ships/harimau/ship.vex
//! ```
use oag_mesh::mesh;
use oag_rcs::rcsmodel;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args.next().expect("archive spec");
    let name = args.next().expect("a .vex name");
    let blob = mesh::read_blob(&spec, &mesh::rcs::sibling_name(&name).unwrap())?;
    let model = rcsmodel::Model::parse(&blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    for (ci, chunk) in model.meshes.iter().enumerate() {
        let Some(material) = model.material_of(chunk) else {
            continue;
        };
        if !material.name.contains("emissive_bloom") {
            continue;
        }
        let stride = chunk.declared_stride().unwrap_or(0);
        for (si, sub) in chunk.submeshes.iter().enumerate() {
            let pos = chunk.positions(&blob, sub, stride)?;
            let uv = chunk.texcoords(&blob, sub, stride)?;
            let idx = chunk.indices(&blob, sub)?;
            println!(
                "chunk {ci} sub {si}: {} verts, {} indices ({} tri)",
                sub.vertex_count,
                sub.index_count,
                sub.index_count / 3
            );
            let mut parent: Vec<usize> = (0..pos.len()).collect();
            fn find(p: &mut Vec<usize>, mut x: usize) -> usize {
                while p[x] != x {
                    p[x] = p[p[x]];
                    x = p[x];
                }
                x
            }
            for t in idx.chunks(3) {
                let a = find(&mut parent, t[0] as usize);
                for &v in &t[1..] {
                    let b = find(&mut parent, v as usize);
                    parent[b] = a;
                }
            }
            let mut tris: std::collections::BTreeMap<usize, usize> = Default::default();
            for t in idx.chunks(3) {
                *tris.entry(find(&mut parent, t[0] as usize)).or_default() += 1;
            }
            let mut comps: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
            let used: std::collections::BTreeSet<usize> = idx.iter().map(|&i| i as usize).collect();
            for v in used {
                let r = find(&mut parent, v);
                comps.entry(r).or_default().push(v);
            }
            for (root, vs) in comps {
                let mut lo = [f32::MAX; 3];
                let mut hi = [f32::MIN; 3];
                let mut ulo = [f32::MAX; 2];
                let mut uhi = [f32::MIN; 2];
                for &v in &vs {
                    for k in 0..3 {
                        lo[k] = lo[k].min(pos[v][k]);
                        hi[k] = hi[k].max(pos[v][k]);
                    }
                    for k in 0..2 {
                        ulo[k] = ulo[k].min(uv[v][k]);
                        uhi[k] = uhi[k].max(uv[v][k]);
                    }
                }
                println!(
                    "  piece {:3} verts {:2} tri  x {:7.3}..{:7.3} y {:7.3}..{:7.3} z {:7.3}..{:7.3}  u {:.3}..{:.3} v {:.3}..{:.3}",
                    vs.len(),
                    tris[&root],
                    lo[0],
                    hi[0],
                    lo[1],
                    hi[1],
                    lo[2],
                    hi[2],
                    ulo[0],
                    uhi[0],
                    ulo[1],
                    uhi[1]
                );
            }
        }
    }
    Ok(())
}
