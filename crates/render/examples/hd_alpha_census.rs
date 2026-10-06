//! Scratch probe: the alpha every blended surface actually resolves to, per
//! material slot, weighted by world-space triangle area.
//!
//! A blended draw whose texel alpha comes out at zero is drawn and invisible,
//! which on a frame reads exactly like geometry that was never built. This
//! reproduces `mesh.wesl`'s own coverage choice - which of the two bound
//! textures the alpha is read from (`slots::ALPHA_FROM_SECOND`) and which of
//! its four channels (`slots::alpha_channel`) - so the answer is the shader's
//! and not a guess at it.

use oag_mesh::mesh::{self, ModelTexture, slots};

fn texel(texture: &ModelTexture, rgba: &[u8], [u, v]: [f32; 2]) -> [f32; 4] {
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

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());

    let data = mesh::read_blob(&spec, &name)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("{name}: no sibling .rcsmodel"))?;
    let source = oag_rcs::rcsmodel::Model::parse(&geometry)?;
    let (model, _) = mesh::rcs::scene_from(&spec, &name, &data)?
        .ok_or_else(|| anyhow::anyhow!("{name}: not a PS3 model"))?;

    let mut decoded: std::collections::HashMap<usize, Vec<u8>> = Default::default();
    let mut rows: std::collections::BTreeMap<usize, (f64, f64, usize)> =
        std::collections::BTreeMap::new();
    for draws in [&model.alpha_tested_draws, &model.transparent_draws] {
        for d in draws {
            let Some(slot) = d.texture else { continue };
            let bits = model
                .material_slots
                .get(slot)
                .copied()
                .unwrap_or(slots::DEFAULT);
            let first = model.textures.get(slot).and_then(Option::as_ref);
            let second = model.lightmaps.get(slot).and_then(Option::as_ref);
            let coverage = if bits & slots::ALPHA_FROM_SECOND != 0 {
                second
            } else {
                first
            };
            let Some(coverage) = coverage else { continue };
            let channel = ((bits >> 3) & 3) as usize;
            let row = rows.entry(slot).or_insert((0.0, 0.0, 0));
            row.2 += 1;
            for tri in model.indices[d.range.start as usize..d.range.end as usize]
                .as_chunks::<3>()
                .0
            {
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
                for v in vs {
                    row.0 += area / 3.0;
                    let texels = rgba_of(&mut decoded, coverage);
                    row.1 += (area / 3.0) * f64::from(texel(coverage, texels, v.texcoord)[channel]);
                }
            }
        }
    }

    let mut ranked: Vec<_> = rows
        .into_iter()
        .filter(|(_, (area, _, _))| *area > 0.0)
        .map(|(slot, (area, alpha, draws))| (alpha / area, area, slot, draws))
        .collect();
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0));
    println!("blended slots by the alpha they resolve to, dimmest first:");
    for (mean, area, slot, draws) in ranked {
        let material = source.materials.get(slot);
        let bits = model
            .material_slots
            .get(slot)
            .copied()
            .unwrap_or(slots::DEFAULT);
        println!(
            "  slot {slot:>4} alpha {mean:.4}  {area:>8.0} sq units  {draws:>3} draw(s)  \
             chan {}{}  {}",
            (bits >> 3) & 3,
            if bits & slots::ALPHA_FROM_SECOND != 0 {
                " of the second"
            } else {
                ""
            },
            material.map_or("<none>", |m| m.name.as_str()),
        );
    }
    Ok(())
}
