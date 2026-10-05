//! Scratch probe: what coverage a blended chunk actually asks for.
//!
//! For every draw call whose texture matches a needle, samples that texture at
//! each of the draw's own vertex texture coordinates, through the alpha
//! channel `mesh::rcs::skin::roles` chose, and reports the distribution. The
//! direct answer to "is this surface see-through because the disc says so, or
//! because this renderer reads it wrong".

use oag_mesh::mesh;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());
    let want = args
        .next()
        .unwrap_or_else(|| "dc_iridescent_gradient".into());

    let data = mesh::read_blob(&spec, &name)?;
    let (model, _) = mesh::rcs::scene_from(&spec, &name, &data)?
        .ok_or_else(|| anyhow::anyhow!("{name}: not a PS3 model"))?;

    let lists: [(&str, &Vec<mesh::DrawCall>); 3] = [
        ("opaque", &model.draws),
        ("cutout", &model.alpha_tested_draws),
        ("blended", &model.transparent_draws),
    ];
    for (list, draws) in lists {
        for (index, d) in draws.iter().enumerate() {
            let Some(texture) = d.texture.and_then(|t| model.textures.get(t)?.as_ref()) else {
                continue;
            };
            if !texture.label.contains(&want) {
                continue;
            }
            // Decoded once per draw call rather than per texel: Wipeout HD's
            // textures stay in their DXT blocks in memory now. See
            // `oag_mesh::mesh::Texels`.
            let Some(rgba) = texture.to_rgba() else {
                continue;
            };
            // The alpha lane the roles reading chose, per vertex - every vertex
            // of one draw call shares a material, so the first one answers.
            let first = model.indices[d.range.start as usize] as usize;
            let channel = ((model.vertices[first].slots >> 3) & 3) as usize;
            let (mut lo, mut hi, mut sum) = (255u32, 0u32, 0u64);
            let (mut zero, mut faint, mut n) = (0usize, 0usize, 0usize);
            let (mut umin, mut umax, mut vmin, mut vmax) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
            for &i in &model.indices[d.range.start as usize..d.range.end as usize] {
                let v = &model.vertices[i as usize];
                let [u, w] = v.texcoord;
                umin = umin.min(u);
                umax = umax.max(u);
                vmin = vmin.min(w);
                vmax = vmax.max(w);
                // Repeat addressing, which is what the sampler binds.
                let x = ((u.rem_euclid(1.0) * texture.width as f32) as u32).min(texture.width - 1)
                    as usize;
                let y = ((w.rem_euclid(1.0) * texture.height as f32) as u32).min(texture.height - 1)
                    as usize;
                let at = (y * texture.width as usize + x) * 4 + channel;
                let a = u32::from(rgba[at]);
                lo = lo.min(a);
                hi = hi.max(a);
                sum += u64::from(a);
                zero += usize::from(a < 8);
                faint += usize::from(a < 64);
                n += 1;
            }
            let c = d.bounds.centre;
            println!(
                "  centre [{:.1}, {:.1}, {:.1}] radius {:.1}",
                c[0], c[1], c[2], d.bounds.radius
            );
            println!(
                "{list} draw {index}: {} tri(s)  {}\n  \
                 alpha channel {channel}: {lo}..{hi} mean {:.1}, {:.1}% under 8, {:.1}% under 64\n  \
                 u {umin:.2}..{umax:.2}  v {vmin:.2}..{vmax:.2}  ({}x{} texture)",
                (d.range.end - d.range.start) / 3,
                texture.label,
                sum as f64 / n as f64,
                zero as f64 / n as f64 * 100.0,
                faint as f64 / n as f64 * 100.0,
                texture.width,
                texture.height,
            );
        }
    }
    Ok(())
}
