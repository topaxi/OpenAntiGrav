//! `uvanim_diffuse_emissive`'s own `EmissiveTexture` (`0xb1f2a176`) sample,
//! read straight off the material's own sampler table and decoded - the one
//! part of "own baked colour/texture/glow term"
//! `hd_amphiseum_ceiling_vertex_light.rs` did not read
//! (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "`uvanim_diffuse_
//! emissive`'s own colour is read, and its largest populations do not
//! confirm the warm reading").
//!
//! `rcsmodel::Material::samplers` is `(name hash, path)` directly - no unit
//! lookup, no shader-variant resolution needed, the same shape
//! `hd_amphiseum_ceiling_vertex_light.rs` already reads `m.parameters`
//! through by hash rather than going via `mesh::rcs::skin`'s pick logic.

use oag_mesh::mesh;

const EMISSIVE_TEXTURE: u32 = 0xb1f2_a176;
/// `emissive.rs`'s `TINT`: the float3 the emissive sample is multiplied by.
const TINT: u32 = 0xe8bc_d7f5;

/// Mean of all four channels, `0..=1` - alpha included, since the alpha-gate
/// check needs it and a caller that only wanted RGB can ignore `[3]`.
fn mean_rgba(blob: &[u8]) -> anyhow::Result<([f32; 4], u32, u32)> {
    let parsed = oag_texture::gtf::Gtf::parse(blob)?;
    let texture = parsed
        .only()
        .ok_or_else(|| anyhow::anyhow!("not one texture"))?;
    let rgba = texture.to_rgba(blob)?;
    let (width, height) = texture.level_size(0);
    let mut sum = [0f64; 4];
    let mut n = 0usize;
    for row in &rgba {
        for px in row.as_chunks::<4>().0 {
            for k in 0..4 {
                sum[k] += f64::from(px[k]) / 255.0;
            }
            n += 1;
        }
    }
    let mean = [
        (sum[0] / n as f64) as f32,
        (sum[1] / n as f64) as f32,
        (sum[2] / n as f64) as f32,
        (sum[3] / n as f64) as f32,
    ];
    Ok((mean, width, height))
}

/// `(hue degrees, saturation)`. Saturation, not a fixed `delta` epsilon, is
/// what decides whether the hue means anything - a computed *mean* over many
/// texels lands within a few parts in `1e-4` of grey even on a genuinely
/// achromatic source, which a bare `delta <= 1e-6` guard does not catch (it
/// let one of this file's own five rows through as a spurious `180deg`
/// before this fix).
fn hue_degrees(r: f32, g: f32, b: f32) -> (f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let saturation = if max <= 1e-6 { 0.0 } else { delta / max };
    if saturation < 0.02 {
        return (0.0, saturation);
    }
    let h = if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (if h < 0.0 { h + 360.0 } else { h }, saturation)
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/amphiseum/track.vex".into());
    let slots: Vec<usize> = args
        .next()
        .unwrap_or_else(|| "354,357,382,178,432".into())
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let data = mesh::read_blob(&spec, &name)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("{name}: no sibling .rcsmodel"))?;
    let source = oag_rcs::rcsmodel::Model::parse(&geometry)?;
    let (model, _) = mesh::rcs::scene_from(&spec, &name, &data)?
        .ok_or_else(|| anyhow::anyhow!("{name}: not a PS3 model"))?;

    for &slot in &slots {
        let Some(material) = source.materials.get(slot) else {
            println!("slot {slot}: <no such slot>");
            continue;
        };
        println!("slot {slot}\t{}", material.name);
        // Necessary-not-sufficient check that this slot's glow reaches the
        // renderer at all: `Model::lightmaps` is where the *second* texture
        // lands once decoded, whatever its role (`hd_emissive_reach.rs`'s own
        // condition 2), and `Model::emissive` is deduplicated by `(tint,
        // rate)` value rather than indexed by material slot - so a match on
        // this material's own tint is evidence its entry exists in the built
        // table, not proof this exact slot's vertices carry the index that
        // selects it.
        println!(
            "  second texture reached Model::lightmaps[{slot}]: {}",
            model.lightmaps.get(slot).is_some_and(Option::is_some)
        );
        if let Some(p) = material.parameters.iter().find(|p| p.hash == TINT) {
            let (hue, sat) = hue_degrees(p.value[0], p.value[1], p.value[2]);
            println!(
                "  TINT (0xe8bcd7f5) = ({:.4}, {:.4}, {:.4}, {:.4}) quads={} hue={:.1}deg sat={:.3}{}",
                p.value[0],
                p.value[1],
                p.value[2],
                p.value[3],
                p.quads,
                hue,
                sat,
                if sat < 0.02 {
                    " (near-grey, hue not meaningful)"
                } else {
                    ""
                }
            );
            let matched = model.emissive.iter().any(|e| {
                e.tint
                    .iter()
                    .zip(p.value)
                    .all(|(a, b)| (a - b).abs() < 1e-4)
            });
            println!("  this tint appears in Model::emissive's built table: {matched}");
        } else {
            println!("  no TINT (0xe8bcd7f5) parameter on this material");
        }
        // The glow is added as `albedo + diffuseAlpha * tint * unit1Sample`
        // per emissive.rs - so a warm tint contributes nothing where the
        // *albedo's own alpha* is ~0.
        if !material.texture.is_empty() {
            let path = &material.texture;
            match mesh::read_blob(&spec, path)
                .ok()
                .and_then(|b| mean_rgba(&b).ok())
            {
                Some((mean, w, h)) => println!(
                    "  albedo (texture) -> {path} {w}x{h} alpha mean={:.4} (gates the glow's reach)",
                    mean[3]
                ),
                None => println!("  albedo (texture) -> {path}: could not read/decode"),
            }
        }
        let Some((_hash, Some(path))) = material
            .samplers
            .iter()
            .find(|(h, _)| *h == EMISSIVE_TEXTURE)
        else {
            println!("  no EmissiveTexture (0xb1f2a176) sampler entry on this material");
            continue;
        };
        println!("  EmissiveTexture -> {path}");
        match mesh::read_blob(&spec, path) {
            Err(e) => println!("  could not read {path}: {e}"),
            Ok(blob) => match mean_rgba(&blob) {
                Err(e) => println!("  could not decode {path}: {e}"),
                Ok((mean, width, height)) => {
                    let (hue, sat) = hue_degrees(mean[0], mean[1], mean[2]);
                    println!(
                        "  {width}x{height} mean rgba=({:.4}, {:.4}, {:.4}, {:.4}) hue={:.1}deg sat={:.3}{}",
                        mean[0],
                        mean[1],
                        mean[2],
                        mean[3],
                        hue,
                        sat,
                        if sat < 0.02 {
                            " (near-grey, hue not meaningful)"
                        } else {
                            ""
                        }
                    );
                }
            },
        }
    }
    Ok(())
}
