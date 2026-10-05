//! Scratch probe: which draw calls of a built HD circuit sit near a world
//! position, in which of the three lists, with what texture and blend.
//!
//! Built through `mesh::rcs::scene_from` - the real production path, not a
//! reimplementation - so what it reports is what the race actually submits.

use oag_core::math::Vec3;
use oag_mesh::mesh;

/// The distance from `p` to the nearest point of triangle `abc`.
///
/// Plain barycentric region test - Ericson, *Real-Time Collision Detection*,
/// 5.1.5. Nearest **vertex** is not this and reads very differently on a road:
/// a long sparsely tessellated ribbon can pass a metre under a point whose
/// closest vertex is twelve metres away.
fn point_to_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> f32 {
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return (p - a).length();
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return (p - b).length();
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return (p - (a + ab * (d1 / (d1 - d3)))).length();
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return (p - c).length();
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return (p - (a + ac * (d2 / (d2 - d6)))).length();
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (p - (b + (c - b) * w)).length();
    }
    let denom = 1.0 / (va + vb + vc);
    (p - (a + ab * (vb * denom) + ac * (vc * denom))).length()
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());
    let at: Vec<f32> = args
        .next()
        .unwrap_or_else(|| "327.2,-42.0,-158.2".into())
        .split(',')
        .map(|s| s.trim().parse().unwrap())
        .collect();
    let reach: f32 = args.next().unwrap_or_else(|| "40".into()).parse().unwrap();

    if std::env::var("OAG_LIST_MATERIALS").is_ok() {
        let data = mesh::read_blob(&spec, &name)?;
        let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
            .ok_or_else(|| anyhow::anyhow!("no sibling"))?;
        let m = oag_rcs::rcsmodel::Model::parse(&geometry)?;
        let mut count: std::collections::BTreeMap<u32, usize> = Default::default();
        for mesh in &m.meshes {
            *count.entry(mesh.material).or_default() += 1;
        }
        for (slot, mat) in m.materials.iter().enumerate() {
            println!(
                "{slot:4} {:4} chunk(s)  {}  tex {}",
                count.get(&(slot as u32)).copied().unwrap_or(0),
                mat.name,
                mat.texture
            );
        }
        return Ok(());
    }
    let data = mesh::read_blob(&spec, &name)?;
    let (model, report) = mesh::rcs::scene_from(&spec, &name, &data)?
        .ok_or_else(|| anyhow::anyhow!("{name}: not a PS3 model"))?;
    println!("{}", report.describe());

    if std::env::var("OAG_BIGGEST").is_ok() {
        // Draw calls by bounding radius. A circuit is hundreds of units
        // across; a draw whose sphere is thousands is geometry that reaches
        // somewhere it should not.
        let mut rows: Vec<(f32, String, u32, [f32; 3])> = Vec::new();
        let lists: [(&str, &Vec<mesh::DrawCall>); 3] = [
            ("opaque", &model.draws),
            ("cutout", &model.alpha_tested_draws),
            ("blended", &model.transparent_draws),
        ];
        for (list, draws) in lists {
            for d in draws {
                let label = d
                    .texture
                    .and_then(|t| model.textures.get(t))
                    .and_then(|t| t.as_ref())
                    .map_or("<none>", |t| t.label.as_str());
                rows.push((
                    d.bounds.radius,
                    format!("{list:8} {label}"),
                    (d.range.end - d.range.start) / 3,
                    d.bounds.centre,
                ));
            }
        }
        rows.sort_by(|a, b| b.0.total_cmp(&a.0));
        println!("draw calls by bounding radius, largest first:");
        for (r, label, tris, c) in rows.iter().take(20) {
            println!(
                "  radius {r:10.1}  {tris:6} tri(s)  centre [{:.0}, {:.0}, {:.0}]  {label}",
                c[0], c[1], c[2]
            );
        }
        return Ok(());
    }

    if std::env::var("OAG_NEAREST").is_ok() {
        let point = Vec3::new(at[0], at[1], at[2]);
        // Nearest *vertex* per draw call, not the bounding sphere: a long thin
        // road chunk's sphere can be centred hundreds of units away.
        let mut rows: Vec<(f32, String, u32)> = Vec::new();
        let lists: [(&str, &Vec<mesh::DrawCall>); 3] = [
            ("opaque", &model.draws),
            ("cutout", &model.alpha_tested_draws),
            ("blended", &model.transparent_draws),
        ];
        for (list, draws) in lists {
            for d in draws {
                let mut best = f32::MAX;
                let span = &model.indices[d.range.start as usize..d.range.end as usize];
                for tri in span.as_chunks::<3>().0 {
                    let v = |k: usize| {
                        let p = model.vertices[tri[k] as usize].position;
                        Vec3::new(p[0], p[1], p[2])
                    };
                    best = best.min(point_to_triangle(point, v(0), v(1), v(2)));
                }
                let label = d
                    .texture
                    .and_then(|t| model.textures.get(t))
                    .and_then(|t| t.as_ref())
                    .map_or("<none>", |t| t.label.as_str());
                rows.push((
                    best,
                    format!("{list:8} {label}"),
                    (d.range.end - d.range.start) / 3,
                ));
            }
        }
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        println!("nearest draw call(s) to {at:?}, by nearest point on a triangle:");
        for (dist, label, tris) in rows.iter().take(25) {
            println!("  {dist:8.2}  {tris:6} tri(s)  {label}");
        }
        return Ok(());
    }

    let lists: [(&str, &Vec<mesh::DrawCall>); 3] = [
        ("opaque", &model.draws),
        ("cutout", &model.alpha_tested_draws),
        ("blended", &model.transparent_draws),
    ];
    let mut by_texture: std::collections::BTreeMap<String, (usize, u32)> = Default::default();
    for (list, draws) in lists {
        for d in draws {
            let c = d.bounds.centre;
            let dist = ((c[0] - at[0]).powi(2) + (c[1] - at[1]).powi(2) + (c[2] - at[2]).powi(2))
                .sqrt()
                - d.bounds.radius;
            if dist > reach {
                continue;
            }
            let label = d
                .texture
                .and_then(|t| model.textures.get(t))
                .and_then(|t| t.as_ref())
                .map_or("<none>", |t| t.label.as_str());
            let entry = by_texture
                .entry(format!("{list:8} {label}"))
                .or_insert((0, 0));
            entry.0 += 1;
            entry.1 += (d.range.end - d.range.start) / 3;
        }
    }
    println!("within {reach} units of {at:?}:");
    for (key, (draws, tris)) in &by_texture {
        println!("  {draws:4} draw(s) {tris:6} tri(s)  {key}");
    }
    Ok(())
}
