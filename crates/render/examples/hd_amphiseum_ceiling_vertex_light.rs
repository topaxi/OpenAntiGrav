//! Raw baked per-vertex colour (`Mesh::vertex_light`) for Amphiseum's
//! ceiling materials' non-lightmap, colour-set-carrying chunks - the disc's
//! own `f[TC0]` input the reconstructed fragment/vertex-program equation in
//! `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s `lane-hd-ceiling`
//! section reads before its `pow(_, prelitBias) * prelitScaleSpecular`
//! curve, and which `mesh.wgsl` currently adds raw with no curve at all.
//!
//! Per material slot named on the command line: the population's raw mean
//! (which a `pow(_, 3.5)` curve makes a poor summary of the *rendered*
//! result - the curve amplifies the brightest vertices far more than it
//! amplifies the mean), the top-decile-by-luma mean and hue (the vertices
//! that actually dominate a `pow(x, 3.5)` sum), and the same top decile
//! after applying Amphiseum's own authored curve
//! (`Lighting.Prelit ambient colour scale`=6.0,
//! `Lighting.Prelit ambient colour power`=3.5, read off
//! `track.envsettings` directly, not fitted) - to see whether the curve
//! itself could turn a borderline population warm.

use oag_mesh::mesh;

/// Amphiseum's own `track.envsettings`: `Lighting.Prelit ambient colour
/// scale`/`power`, both uniform `(6.0, 6.0, 6.0)`/`(3.5, 3.5, 3.5)` - read by
/// `python3 scripts/psarc.py cat ... amphiseum/track.envsettings`, not
/// fitted.
const PRELIT_SCALE: f32 = 6.0;
const PRELIT_POWER: f32 = 3.5;

/// Hue in degrees, 0 for a grey/black pixel (matching
/// `scripts/hd-frame-compare.py`'s own convention for an unsaturated pixel).
fn hue_degrees(r: f32, g: f32, b: f32) -> f32 {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    if delta <= 1e-6 {
        return 0.0;
    }
    let h = if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    if h < 0.0 { h + 360.0 } else { h }
}

fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn report(label: &str, colours: &[[f32; 3]]) {
    if colours.is_empty() {
        println!("  {label}: no vertices");
        return;
    }
    let n = colours.len() as f64;
    let mean = colours.iter().fold([0f64; 3], |mut acc, c| {
        for k in 0..3 {
            acc[k] += f64::from(c[k]);
        }
        acc
    });
    let mean = [
        (mean[0] / n) as f32,
        (mean[1] / n) as f32,
        (mean[2] / n) as f32,
    ];
    println!(
        "  {label} mean rgb=({:.4}, {:.4}, {:.4}) hue={:.1}deg",
        mean[0],
        mean[1],
        mean[2],
        hue_degrees(mean[0], mean[1], mean[2])
    );
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/amphiseum/track.vex".into());
    let slots: Vec<u32> = args
        .next()
        .unwrap_or_else(|| "353,360".into())
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let data = mesh::read_blob(&spec, &name)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("{name}: no sibling .rcsmodel"))?;
    let source = oag_rcs::rcsmodel::Model::parse(&geometry)?;

    for &slot in &slots {
        let material = source.materials.get(slot as usize);
        println!(
            "slot {slot}\t{}",
            material.map_or("<none>", |m| m.name.as_str())
        );
        // The material's own declared parameters, resolved by hash - the
        // authored *value* behind an unnamed patched hash, per the advisor's
        // own precedent (`pads.md`'s `0x7611a2d8`: "reading the value across
        // every circuit is what settled it").
        if let Some(m) = material {
            for p in &m.parameters {
                let name = match p.hash {
                    0x7611_a2d8 => "0x7611a2d8 (unnamed, ex-\"Speedup Pad\")",
                    0xef18_f362 => "0xef18f362 (unnamed)",
                    0x370a_63cb => "SpecularColour",
                    0x4c3c_ae3a => "SpecularColor",
                    0x81db_67ea => "constantAmbientColour",
                    _ => continue,
                };
                println!(
                    "    parameter {name} = ({:.4}, {:.4}, {:.4}, {:.4})",
                    p.value[0], p.value[1], p.value[2], p.value[3]
                );
            }
        }

        let mut raw: Vec<[f32; 3]> = Vec::new();
        let mut chunks_with_colour_set = 0usize;
        let mut chunks_total = 0usize;
        // Chunk-local Y range (bias + quantised position, no node placement
        // applied) - a coarse, capture-independent check that a slot's own
        // geometry actually sits overhead rather than at track level, since
        // the capture that measured the dome's screen position by eye
        // (`amphiseum-grid`) is gone from disk.
        let (mut y_min, mut y_max) = (f32::MAX, f32::MIN);
        for mesh in source
            .meshes
            .iter()
            .flat_map(oag_rcs::rcsmodel::Mesh::surfaces)
        {
            if mesh.material != slot {
                continue;
            }
            chunks_total += 1;
            let has_cs = mesh
                .decl
                .as_ref()
                .is_some_and(oag_rcs::rcsmodel::VertexDecl::light_colour_set);
            if !has_cs {
                continue;
            }
            chunks_with_colour_set += 1;
            let Some(stride) = mesh
                .declared_stride()
                .or_else(|| mesh.solve_stride_by_layout())
            else {
                continue;
            };
            for submesh in &mesh.submeshes {
                if submesh.vertex_count == 0 {
                    continue;
                }
                let Ok(colours) = mesh.vertex_light(&geometry, submesh, stride) else {
                    continue;
                };
                raw.extend(colours.into_iter().map(|c| [c[0], c[1], c[2]]));
                if let Ok(positions) = mesh.positions(&geometry, submesh, stride) {
                    for p in positions {
                        y_min = y_min.min(p[1]);
                        y_max = y_max.max(p[1]);
                    }
                }
            }
        }
        println!("  chunks: {chunks_with_colour_set} of {chunks_total} carry a colour set");
        println!("  vertices: {}", raw.len());
        if y_min <= y_max {
            println!("  chunk-local y range: [{y_min:.1}, {y_max:.1}]");
        }
        report("raw, whole population", &raw);

        // The top decile by luma: the population a `pow(x, 3.5)` sum is
        // actually dominated by, not the population the arithmetic mean
        // describes.
        let mut by_luma = raw.clone();
        by_luma.sort_by(|a, b| luma(*b).partial_cmp(&luma(*a)).unwrap());
        let top = by_luma.len() / 10;
        report(
            "raw, top decile by luma",
            &by_luma[..top.max(1).min(by_luma.len())],
        );

        // The same top decile through Amphiseum's own authored curve -
        // applied to the raw normalised byte, with **no** sRGB decode: the
        // vertex program's own `LG2 -> MUL -> EX2 -> MUL` chain reads
        // `v[3]` directly, unlike `mesh.wgsl`'s lightmap path, which decodes
        // `baked.rgb` through `pow(_, 2.2)` first. Applying that decode here
        // would be inventing a step the vertex microcode does not have.
        let curved: Vec<[f32; 3]> = by_luma[..top.max(1).min(by_luma.len())]
            .iter()
            .map(|c| {
                [
                    c[0].max(0.0).powf(PRELIT_POWER) * PRELIT_SCALE,
                    c[1].max(0.0).powf(PRELIT_POWER) * PRELIT_SCALE,
                    c[2].max(0.0).powf(PRELIT_POWER) * PRELIT_SCALE,
                ]
            })
            .collect();
        report(
            "curved (pow(x,3.5)*6), top decile by pre-curve luma",
            &curved,
        );
    }
    Ok(())
}
