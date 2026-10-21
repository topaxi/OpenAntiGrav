//! Scratch probe: for chosen material ordinals, every chunk's declared
//! attributes and the bounding box of each texture coordinate set it carries.
//!
//! ```sh
//! cargo run -p oag-render --example hd_slot_uvs -- <image> /data/environments/amphiseum/track.vex 501
//! ```

use oag_assets::Container;
use oag_mesh::mesh;
use oag_rcs::rcsmodel;

fn bbox(uv: &[[f32; 2]]) -> String {
    let lo = uv
        .iter()
        .fold([f32::MAX; 2], |a, p| [a[0].min(p[0]), a[1].min(p[1])]);
    let hi = uv
        .iter()
        .fold([f32::MIN; 2], |a, p| [a[0].max(p[0]), a[1].max(p[1])]);
    format!("u {:.3}..{:.3} v {:.3}..{:.3}", lo[0], hi[0], lo[1], hi[1])
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args.next().unwrap();
    let name = args.next().unwrap();
    let slots: Vec<u32> = args
        .next()
        .unwrap()
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();
    let spec = format!("{image}:PS3_GAME/USRDIR/DATA00.PSARC");
    let data = mesh::read_blob(&spec, &name)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("no sibling"))?;
    let _ = Container::open(&spec)?;
    let model = rcsmodel::Model::parse(&geometry)?;
    for (i, m) in model.meshes.iter().enumerate() {
        if !slots.contains(&m.material) {
            continue;
        }
        println!(
            "chunk {i} hash {:08x} material {} decl {:?}",
            m.hash,
            m.material,
            m.declared_stride()
        );
        if let Some(decl) = &m.decl {
            for a in &decl.attributes {
                println!(
                    "    attr {:08x} {:?} comps {} type {} off {}",
                    a.name_hash,
                    a.name(),
                    a.components,
                    a.rsx_type,
                    a.offset
                );
            }
        }
        for sub in &m.submeshes {
            let stride = m.declared_stride().unwrap_or(0);
            let d = m.texcoords(&geometry, sub, stride).ok();
            let l = m.lightmap_texcoords(&geometry, sub, stride).ok();
            let s = m.second_texcoords(&geometry, sub, stride).ok();
            let idx = m.indices(&geometry, sub).ok();
            let pos = m.positions(&geometry, sub, stride).ok();
            let (refd, wild) = match (&idx, &d) {
                (Some(ix), Some(d)) => {
                    let mut seen = vec![false; d.len()];
                    for &i in ix {
                        if let Some(s) = seen.get_mut(usize::from(i)) {
                            *s = true;
                        }
                    }
                    let n = seen.iter().filter(|s| **s).count();
                    let w = seen
                        .iter()
                        .zip(d)
                        .filter(|(s, uv)| **s && (uv[0].abs() > 8.0 || uv[1].abs() > 8.0))
                        .count();
                    (n, w)
                }
                _ => (0, 0),
            };
            let centre = pos.as_ref().map(|p| {
                let n = p.len() as f32;
                let c = p
                    .iter()
                    .fold([0.0f32; 3], |a, v| [a[0] + v[0], a[1] + v[1], a[2] + v[2]]);
                [c[0] / n, c[1] / n, c[2] / n]
            });
            println!(
                "    sub fmt {:02x?} referenced {refd} wild-referenced {wild} local centre {centre:?}",
                sub.format
            );
            println!(
                "    sub verts {} diffuse[{}] lightmap[{}] second[{}]",
                sub.vertex_count,
                d.as_deref().map_or("none".into(), bbox),
                l.as_deref().map_or("none".into(), bbox),
                s.as_deref().map_or("none".into(), bbox),
            );
        }
    }
    Ok(())
}
