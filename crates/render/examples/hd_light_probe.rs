//! Scratch probe: compute `mesh.wgsl`'s whole light equation on the CPU, for
//! the draw calls of one texture, **weighted by world-space triangle area**.
//!
//! `hd_shade_probe.rs` reports the shading inputs a vertex carries. This
//! reports what the shader makes of them: the prelit term off the circuit's
//! own lightmap atlas, the sun term through its baked shadow mask, and the
//! product with the albedo - so a surface that arrives on screen darker than
//! its own art can be attributed to the term that darkens it.
//!
//! Area weighting rather than a vertex count, because screen coverage follows
//! area and a vertex histogram does not: a handful of large triangles on a
//! dark patch of an atlas outvote a dense strip on a light one.

use oag_mesh::mesh::{self, ModelTexture, slots};
use oag_tables::envsettings::{self, EnvSettings};

/// Bilinear-free point sample, the way `hd_shade_probe` reads a texel.
fn sample(texture: &ModelTexture, rgba: &[u8], [u, v]: [f32; 2]) -> [f32; 4] {
    let x = ((u.rem_euclid(1.0) * texture.width as f32) as u32).min(texture.width - 1) as usize;
    let y = ((v.rem_euclid(1.0) * texture.height as f32) as u32).min(texture.height - 1) as usize;
    let at = (y * texture.width as usize + x) * 4;
    std::array::from_fn(|c| rgba.get(at + c).map_or(0.0, |&b| f32::from(b) / 255.0))
}

/// One decode per texture, keyed on the shared texture's own address.
///
/// Wipeout HD's `.gtf` textures stay in their DXT blocks in memory now (see
/// `oag_mesh::mesh::Texels`), so decoding at the call site would decode a
/// 2048x2048 atlas once per vertex. This is what the probes got for free back
/// when every texture was expanded to RGBA8 at load.
fn rgba_of<'a>(
    cache: &'a mut std::collections::HashMap<usize, Vec<u8>>,
    texture: &std::sync::Arc<ModelTexture>,
) -> &'a [u8] {
    cache
        .entry(std::sync::Arc::as_ptr(texture) as usize)
        .or_insert_with(|| {
            texture
                .to_rgba()
                .map(std::borrow::Cow::into_owned)
                .unwrap_or_default()
        })
}

fn luma([r, g, b]: [f32; 3]) -> f32 {
    0.299 * r + 0.587 * g + 0.114 * b
}

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

    let settings = mesh::read_blob(&spec, &format!("{}.envsettings", &name[..name.len() - 4]))
        .ok()
        .and_then(|blob| String::from_utf8(blob).ok())
        .and_then(|text| EnvSettings::parse(&text).ok())
        .ok_or_else(|| anyhow::anyhow!("no .envsettings beside {name}"))?;
    let triple = |key| settings.vec3(key).unwrap_or([0.0; 3]);
    let direction = settings
        .direction(envsettings::SUN_DIRECTION)
        .unwrap_or([0.0, 1.0, 0.0]);
    let sun = triple(envsettings::SUN_COLOUR);
    let ambient = triple(envsettings::AMBIENT_COLOUR);
    let prelit_scale = settings.vec3(envsettings::PRELIT_SCALE).unwrap_or([1.0; 3]);
    let prelit_power = settings
        .vec3(envsettings::PRELIT_POWER)
        .unwrap_or([1.0; 3])
        .map(|p| p.max(1e-3));
    println!("rig: direction {direction:.3?} sun {sun:.3?} ambient {ambient:.3?}");
    println!("     prelit scale {prelit_scale:.3?} power {prelit_power:.3?}");

    let mut decoded: std::collections::HashMap<usize, Vec<u8>> = Default::default();
    let mut area_total = 0.0f64;
    let mut sums = [0.0f64; 5];
    let mut albedo_sum = [0.0f64; 3];
    let mut out_sum = [0.0f64; 3];
    let mut seen: Vec<usize> = Vec::new();
    let mut anims = 0usize;

    for draws in [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ] {
        for d in draws {
            let Some(slot) = d.texture else { continue };
            let Some(texture) = model.textures.get(slot).and_then(Option::as_ref) else {
                continue;
            };
            if want != "*" && !texture.label.contains(&want) {
                continue;
            }
            if let Ok(only) = std::env::var("OAG_SLOT")
                && only.parse::<usize>() != Ok(slot)
            {
                continue;
            }
            if let Ok(near) = std::env::var("OAG_NEAR") {
                let at: Vec<f32> = near
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if at.len() == 4 {
                    let c = d.bounds.centre;
                    let dist =
                        ((c[0] - at[0]).powi(2) + (c[1] - at[1]).powi(2) + (c[2] - at[2]).powi(2))
                            .sqrt();
                    if dist - d.bounds.radius > at[3] {
                        continue;
                    }
                }
            }
            if !seen.contains(&slot) {
                seen.push(slot);
                let bits = model
                    .material_slots
                    .get(slot)
                    .copied()
                    .unwrap_or(slots::DEFAULT);
                let second = model
                    .lightmaps
                    .get(slot)
                    .and_then(Option::as_ref)
                    .map_or("<none>", |t| t.label.as_str());
                println!(
                    "slot {slot}: bits {bits:#07b}  lightmap {}  albedo_from_second {}  \
                     alpha_from_second {}  alpha channel {}  second {second}",
                    bits & slots::SECOND_IS_LIGHTMAP != 0,
                    bits & slots::ALBEDO_FROM_SECOND != 0,
                    bits & slots::ALPHA_FROM_SECOND != 0,
                    (bits >> 3) & 3,
                );
            }
            let bits = model
                .material_slots
                .get(slot)
                .copied()
                .unwrap_or(slots::DEFAULT);
            let lightmapped = bits & slots::SECOND_IS_LIGHTMAP != 0;
            let second = model.lightmaps.get(slot).and_then(Option::as_ref);
            let mut draw_area = 0.0f64;
            let mut draw_albedo = 0.0f64;
            let mut uv = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
            let mut vsum = 0.0f64;
            let mut seen_v: std::collections::BTreeSet<i32> = std::collections::BTreeSet::new();
            let indices = &model.indices[d.range.start as usize..d.range.end as usize];
            for tri in indices.as_chunks::<3>().0 {
                let vs: [_; 3] = std::array::from_fn(|k| &model.vertices[tri[k] as usize]);
                let e1: [f32; 3] = std::array::from_fn(|c| vs[1].position[c] - vs[0].position[c]);
                let e2: [f32; 3] = std::array::from_fn(|c| vs[2].position[c] - vs[0].position[c]);
                let cross = [
                    e1[1] * e2[2] - e1[2] * e2[1],
                    e1[2] * e2[0] - e1[0] * e2[2],
                    e1[0] * e2[1] - e1[1] * e2[0],
                ];
                let area = f64::from(
                    0.5 * (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt(),
                );
                if area <= 0.0 {
                    continue;
                }
                for v in vs {
                    let w = area / 3.0;
                    let baked = match (lightmapped, second) {
                        (true, Some(t)) => sample(t, rgba_of(&mut decoded, t), v.lightmap_texcoord),
                        _ => [0.0, 0.0, 0.0, 1.0],
                    };
                    let baked_linear: [f32; 3] = std::array::from_fn(|c| baked[c].powf(2.2));
                    let prelit: [f32; 3] = std::array::from_fn(|c| {
                        prelit_scale[c] * baked_linear[c].powf(prelit_power[c])
                    });
                    let n = v.normal;
                    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
                    let ndl = ((n[0] * direction[0] + n[1] * direction[1] + n[2] * direction[2])
                        / len)
                        .clamp(0.0, 1.0);
                    let mask = baked[3] * v.sun_mask;
                    let sun_diffuse: [f32; 3] = std::array::from_fn(|c| sun[c] * ndl * mask);
                    let vertex_light: [f32; 3] = std::array::from_fn(|c| v.colour[c]);
                    let authored: [f32; 3] = std::array::from_fn(|c| {
                        ambient[c] + prelit[c] + vertex_light[c] + sun_diffuse[c]
                    });
                    let picture = if bits & slots::ALBEDO_FROM_SECOND != 0 {
                        match second {
                            Some(t) => sample(t, rgba_of(&mut decoded, t), v.texcoord),
                            None => [0.0; 4],
                        }
                    } else {
                        sample(texture, rgba_of(&mut decoded, texture), v.texcoord)
                    };
                    let lit: [f32; 3] = std::array::from_fn(|c| picture[c].powf(2.2) * authored[c]);
                    let encoded: [f32; 3] =
                        std::array::from_fn(|c| lit[c].clamp(0.0, 1.0).powf(1.0 / 2.2));
                    if v.anim != 0 || v.xform != 0 {
                        anims += 1;
                    }
                    uv.0 = uv.0.min(v.texcoord[0]);
                    uv.1 = uv.1.max(v.texcoord[0]);
                    uv.2 = uv.2.min(v.texcoord[1]);
                    uv.3 = uv.3.max(v.texcoord[1]);
                    seen_v.insert((v.texcoord[1] * 1000.0).round() as i32);
                    vsum += w * f64::from(v.texcoord[1].rem_euclid(1.0));
                    draw_area += w;
                    draw_albedo += w * f64::from(luma([picture[0], picture[1], picture[2]]));
                    area_total += w;
                    sums[0] += w * f64::from(luma(prelit));
                    sums[1] += w * f64::from(luma(vertex_light));
                    sums[2] += w * f64::from(luma(sun_diffuse));
                    sums[3] += w * f64::from(luma(authored));
                    sums[4] += w * f64::from(mask);
                    for c in 0..3 {
                        albedo_sum[c] += w * f64::from(picture[c]);
                        out_sum[c] += w * f64::from(encoded[c]);
                    }
                }
            }
            if std::env::var_os("OAG_PER_DRAW").is_some() && draw_area > 0.0 {
                println!(
                    "  slot {slot} {}draw at {:?} radius {:.0}: {:.0} sq units, albedo luma {:.1}/255, \
                     u {:.2}..{:.2} v {:.2}..{:.2} mean v {:.3}",
                    if d.moving { "moving " } else { "" },
                    d.bounds.centre,
                    d.bounds.radius,
                    draw_area,
                    255.0 * draw_albedo / draw_area,
                    uv.0,
                    uv.1,
                    uv.2,
                    uv.3,
                    vsum / draw_area
                );
                if std::env::var_os("OAG_RAW_V").is_some() {
                    let all: Vec<f32> = seen_v.iter().map(|&t| t as f32 / 1000.0).collect();
                    println!("      raw v values ({}): {all:.3?}", all.len());
                }
            }
        }
    }
    if area_total <= 0.0 {
        println!("no draw matched {want}");
        return Ok(());
    }
    let a = area_total;
    println!("{want}: {:.0} square unit(s) of surface", area_total);
    println!("  {anims} vertex reference(s) carry a UV or node animation");
    println!("  area-weighted, as luma:");
    println!("    prelit        {:.4}", sums[0] / a);
    println!("    vertex light  {:.4}", sums[1] / a);
    println!("    sun diffuse   {:.4}", sums[2] / a);
    println!("    ambient       {:.4}", luma(ambient));
    println!("    authored      {:.4}  (sum of the four)", sums[3] / a);
    println!("    shadow mask   {:.4}", sums[4] / a);
    println!(
        "    albedo (srgb) [{:.3}, {:.3}, {:.3}] = {:.1}/255",
        albedo_sum[0] / a,
        albedo_sum[1] / a,
        albedo_sum[2] / a,
        255.0
            * luma([
                (albedo_sum[0] / a) as f32,
                (albedo_sum[1] / a) as f32,
                (albedo_sum[2] / a) as f32
            ])
    );
    println!(
        "    out    (srgb) [{:.3}, {:.3}, {:.3}] = {:.1}/255",
        out_sum[0] / a,
        out_sum[1] / a,
        out_sum[2] / a,
        255.0
            * luma([
                (out_sum[0] / a) as f32,
                (out_sum[1] / a) as f32,
                (out_sum[2] / a) as f32
            ])
    );
    Ok(())
}
