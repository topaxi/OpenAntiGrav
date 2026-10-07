//! Prints the model-space extents, node scales over time, materials, authored
//! blends and texture statistics of HD's weapon detonation models, as built
//! by `mesh::rcs::build` - the path `oag_game`'s weapon loader takes. Set
//! `OAG_EXTENTS_PNG_DIR` to also write each texture out as a PNG. See
//! `docs/ghidra/functions/ps3-hdfury-eu/plasma.md`, 2026-09-23.
//!
//! ```sh
//! cargo run -p oag-render --example hd_weapon_extents
//! ```

use oag_mesh::mesh;

const SPEC: &str = "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC";

const MODELS: &[&str] = &[
    "/data/weapons/hd_rocket.vex",
    "/data/weapons/hd_plasma_ball.vex",
    "/data/weapons/hd_plasma_ring.vex",
    "/data/weapons/hd_plasma_sphere.vex",
    "/data/weapons/hd_plasma_halo.vex",
    "/data/weapons/hd_missile_ball_bloomring.vex",
    "/data/weapons/hd_missile_explosion.vex",
    "/data/weapons/hd_bomb_halo.vex",
    "/data/weapons/hd_bomb.vex",
    "/data/weapons/hd_bomb_sphere.vex",
    "/data/weapons/hd_bomb_sphere_white.vex",
];

fn main() -> anyhow::Result<()> {
    for name in MODELS {
        let data = mesh::read_blob(SPEC, name)?;
        let Some(geometry) = mesh::rcs::sibling_geometry(SPEC, name, &data) else {
            println!("{name}: no .rcsmodel");
            continue;
        };
        let (model, report) = mesh::rcs::build(
            name,
            &data,
            &geometry,
            &mut |path| mesh::read_blob(SPEC, path).ok(),
            |c| c.mesh,
        )?;
        if let Ok(parsed) = oag_rcs::rcsmodel::Model::parse(&geometry) {
            for m in &parsed.materials {
                println!(
                    "  material {:?}: state {:#x} blend {:?} texture {:?} samplers {:?}",
                    m.name,
                    m.state,
                    m.blend(),
                    m.texture,
                    m.samplers
                );
            }
        }
        let nodes = oag_vex::vex::nodes(&data)?;
        for t in [0.0f32, 0.1, 0.25, 0.5, 1.0, 1.3, 2.0, 3.0] {
            let world = oag_vex::vex::world_transforms_at(&data, &nodes, t);
            let s: Vec<String> = nodes
                .iter()
                .zip(&world)
                .filter(|(n, _)| n.class_id == 0x125)
                .map(|(_, m)| {
                    let sx = (m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt();
                    let sz = (m[8] * m[8] + m[9] * m[9] + m[10] * m[10]).sqrt();
                    format!("({sx:.3},{sz:.3} t {:.2},{:.2},{:.2})", m[12], m[13], m[14])
                })
                .collect();
            println!("  t={t}: mesh-node world scale {}", s.join(" "));
        }
        let world = oag_vex::vex::world_transforms(&data, &nodes);
        for (node, m) in nodes.iter().zip(&world) {
            let sx = (m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt();
            let sy = (m[4] * m[4] + m[5] * m[5] + m[6] * m[6]).sqrt();
            let sz = (m[8] * m[8] + m[9] * m[9] + m[10] * m[10]).sqrt();
            println!(
                "  node class {:#x} children {} world scale ({sx:.4}, {sy:.4}, {sz:.4}) t ({:.3}, {:.3}, {:.3})",
                node.class_id, node.child_count, m[12], m[13], m[14]
            );
        }
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        let mut rmax = 0f32;
        for v in &model.vertices {
            let r = (v.position[0] * v.position[0]
                + v.position[1] * v.position[1]
                + v.position[2] * v.position[2])
                .sqrt();
            rmax = rmax.max(r);
            for a in 0..3 {
                lo[a] = lo[a].min(v.position[a]);
                hi[a] = hi[a].max(v.position[a]);
            }
        }
        // Winding against the authored normals: `agree` counts triangles whose
        // `(b - a) x (c - a)` points the same way as their vertices' own
        // normals, and `outward` those whose normal points away from the
        // model's origin. `mean_normal` is the authored normals' average.
        let (mut agree, mut outward, mut total) = (0usize, 0usize, 0usize);
        let mut mean_normal = [0f32; 3];
        for v in &model.vertices {
            for (sum, n) in mean_normal.iter_mut().zip(v.normal) {
                *sum += n / model.vertices.len() as f32;
            }
        }
        for tri in model.indices.as_chunks::<3>().0 {
            let [a, b, c] = tri.map(|i| model.vertices[i as usize]);
            let e1 = [0, 1, 2].map(|k| b.position[k] - a.position[k]);
            let e2 = [0, 1, 2].map(|k| c.position[k] - a.position[k]);
            let n = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let authored = [0, 1, 2].map(|k| a.normal[k] + b.normal[k] + c.normal[k]);
            let centre = [0, 1, 2].map(|k| a.position[k] + b.position[k] + c.position[k]);
            let dot = |p: [f32; 3], q: [f32; 3]| p[0] * q[0] + p[1] * q[1] + p[2] * q[2];
            total += 1;
            agree += usize::from(dot(n, authored) > 0.0);
            outward += usize::from(dot(n, centre) > 0.0);
        }
        println!(
            "  winding: {agree} of {total} triangles agree with authored normals, \
             {outward} face away from the origin; mean authored normal {mean_normal:?}"
        );
        for (list, draws) in [
            ("opaque", &model.draws),
            ("cutout", &model.alpha_tested_draws),
            ("transparent", &model.transparent_draws),
        ] {
            for d in draws {
                println!(
                    "  {list} draw: {} tris, texture {:?}, blend {:?}, authored {:?}, culled {}",
                    d.range.len() / 3,
                    d.texture,
                    d.blend,
                    d.blend_state
                        .map(|s| (s.color.src_factor, s.color.dst_factor)),
                    d.culled
                );
            }
        }
        if let Some(dir) = std::env::var_os("OAG_EXTENTS_PNG_DIR") {
            let stem = name
                .rsplit('/')
                .next()
                .unwrap_or(name)
                .trim_end_matches(".vex");
            for (slot, texture) in model.textures.iter().enumerate() {
                let Some(texture) = texture else { continue };
                let Some(rgba) = texture.to_rgba() else {
                    continue;
                };
                let mut alpha = [0u64; 4];
                let mut rgb_sum = 0u64;
                for px in rgba.as_chunks::<4>().0 {
                    alpha[usize::from(px[3] / 64)] += 1;
                    rgb_sum += u64::from(px[0]) + u64::from(px[1]) + u64::from(px[2]);
                }
                println!(
                    "  texture {slot} {} {}x{}: alpha quartiles {alpha:?}, mean rgb {:.1}",
                    texture.label,
                    texture.width,
                    texture.height,
                    rgb_sum as f64 / (rgba.len() as f64 / 4.0 * 3.0)
                );
                let path = std::path::Path::new(&dir).join(format!("{stem}-{slot}.png"));
                std::fs::write(
                    path,
                    oag_texture::png::encode_rgba(texture.width, texture.height, &rgba),
                )?;
            }
        }
        println!(
            "{name}: {} verts, min {lo:?} max {hi:?} max|p| {rmax:.4} radius {:.4}\n  {}",
            model.vertices.len(),
            model.radius,
            report.describe()
        );
    }
    Ok(())
}
