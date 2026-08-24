//! Scratch probe: every input `mesh.wgsl` shades a surface with, for the draw
//! calls of one texture, so a surface that renders black can be told from the
//! term that makes it black.
//!
//! Reports the vertex light HD bakes into the colour set, the sun-occlusion
//! mask in its fourth byte, the world normal, and the albedo actually sampled
//! at each vertex's own texture coordinate.

use oag_render::mesh;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());
    let want = args.next().unwrap_or_else(|| "ds_wall_cs".into());

    let data = mesh::read_blob(&spec, &name)?;
    let (model, _) = mesh::rcs::scene_from(&spec, &name, &data)?
        .ok_or_else(|| anyhow::anyhow!("{name}: not a PS3 model"))?;

    let mut n = 0usize;
    let mut light = [0.0f64; 3];
    let mut light_max = [0.0f32; 3];
    let mut mask = (1.0f32, 0.0f32, 0.0f64);
    let mut albedo = [0.0f64; 3];
    let mut albedo_max = [0u8; 3];
    let mut normal_y = (1.0f32, -1.0f32, 0.0f64);
    let mut lit_sum = 0.0f64;
    let (mut umin, mut umax, mut vmin, mut vmax) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);

    for (list, draws) in [
        ("opaque", &model.draws),
        ("cutout", &model.alpha_tested_draws),
        ("blended", &model.transparent_draws),
    ] {
        for d in draws {
            let Some(texture) = d.texture.and_then(|t| model.textures.get(t)?.as_ref()) else {
                continue;
            };
            if !texture.label.contains(&want) {
                continue;
            }
            let _ = list;
            for &i in &model.indices[d.range.start as usize..d.range.end as usize] {
                let v = &model.vertices[i as usize];
                n += 1;
                for c in 0..3 {
                    light[c] += f64::from(v.colour[c]);
                    light_max[c] = light_max[c].max(v.colour[c]);
                }
                mask.0 = mask.0.min(v.sun_mask);
                mask.1 = mask.1.max(v.sun_mask);
                mask.2 += f64::from(v.sun_mask);
                normal_y.0 = normal_y.0.min(v.normal[1]);
                normal_y.1 = normal_y.1.max(v.normal[1]);
                normal_y.2 += f64::from(v.normal[1]);
                lit_sum += f64::from(v.lit);
                let [u, w] = v.texcoord;
                umin = umin.min(u);
                umax = umax.max(u);
                vmin = vmin.min(w);
                vmax = vmax.max(w);
                let x = ((u.rem_euclid(1.0) * texture.width as f32) as u32).min(texture.width - 1)
                    as usize;
                let y = ((w.rem_euclid(1.0) * texture.height as f32) as u32).min(texture.height - 1)
                    as usize;
                let at = (y * texture.width as usize + x) * 4;
                for c in 0..3 {
                    let b = texture.rgba[at + c];
                    albedo[c] += f64::from(b);
                    albedo_max[c] = albedo_max[c].max(b);
                }
            }
        }
    }
    if n == 0 {
        println!("no draw matched {want}");
        return Ok(());
    }
    let f = n as f64;
    println!("{want}: {n} vertex reference(s)");
    println!(
        "  vertex light  mean [{:.3}, {:.3}, {:.3}]  max [{:.3}, {:.3}, {:.3}]",
        light[0] / f,
        light[1] / f,
        light[2] / f,
        light_max[0],
        light_max[1],
        light_max[2]
    );
    println!(
        "  sun_mask      {:.3}..{:.3} mean {:.3}",
        mask.0,
        mask.1,
        mask.2 / f
    );
    println!(
        "  normal.y      {:.3}..{:.3} mean {:.3}",
        normal_y.0,
        normal_y.1,
        normal_y.2 / f
    );
    println!("  lit           mean {:.3}", lit_sum / f);
    println!("  u {umin:.2}..{umax:.2}  v {vmin:.2}..{vmax:.2}");
    println!(
        "  albedo at its own UV  mean [{:.1}, {:.1}, {:.1}]  max [{}, {}, {}]",
        albedo[0] / f,
        albedo[1] / f,
        albedo[2] / f,
        albedo_max[0],
        albedo_max[1],
        albedo_max[2]
    );
    Ok(())
}
