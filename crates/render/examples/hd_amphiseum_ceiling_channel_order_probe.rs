//! Tests the channel-order/byte-order hypothesis
//! `hds-frame-was-too-bright-and-too-bloomy.md`'s Next Steps names for
//! `base_diffusespecular`/`cf_diff_spec`: "Check ... specifically for a
//! channel-order or byte-order defect in their own baked colour-set
//! decode" - `oag_rcs::rcsmodel::Mesh::vertex_light` reads the four bytes at
//! the colour-set attribute's offset as `[r, g, b, mask]`, in file order,
//! with no swap. If that order is wrong - reading a hardware or export
//! convention's `[b, g, r, mask]` as `[r, g, b, mask]` - red and blue trade
//! places, and a warm (red-dominant) surface reads cool (blue-dominant)
//! exactly the way these two materials do at every stage of
//! `hd_amphiseum_ceiling_vertex_light.rs`'s own reconstruction.
//!
//! Reuses that file's exact method (top-decile-by-luma, then Amphiseum's
//! own authored curve) on the same slots, with an R/B swap applied to the
//! raw colour before either step - swapping first, since luma is not
//! symmetric in R and B (`0.2126 r + 0.7152 g + 0.0722 b`), so which
//! vertices land in the top decile can itself change under the swap.
//! `animhexlights` (325) is included as a **negative control**: its own
//! raw colour-set data already reconstructs to the reference's warm hue
//! with no decode change (docs/ghidra/functions/ps3-hdfury-eu/renderer.md,
//! "The per-material microcode sweep"), so if swapping R/B turns *it*
//! cool, that is evidence the swap breaks a working reading rather than
//! fixing a broken one - the control the hypothesis has to survive, not
//! just the two materials it's meant to explain.

use oag_mesh::mesh;

const PRELIT_SCALE: f32 = 6.0;
const PRELIT_POWER: f32 = 3.5;

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

fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn report(label: &str, colours: &[[f32; 3]]) {
    if colours.is_empty() {
        println!("    {label}: no vertices");
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
    let (hue, sat) = hue_degrees(mean[0], mean[1], mean[2]);
    println!(
        "    {label} mean rgb=({:.4}, {:.4}, {:.4}) hue={:.1}deg sat={:.3}",
        mean[0], mean[1], mean[2], hue, sat
    );
}

/// Top-decile-by-luma, then Amphiseum's own curve, reported - the two
/// stages `hd_amphiseum_ceiling_vertex_light.rs` already established are
/// what a rendered pixel is actually dominated by.
fn reconstruct(label: &str, raw: &[[f32; 3]]) {
    let mut by_luma = raw.to_vec();
    by_luma.sort_by(|a, b| luma(*b).partial_cmp(&luma(*a)).unwrap());
    let top = (by_luma.len() / 10).max(1).min(by_luma.len());
    report(&format!("{label}, top decile by luma"), &by_luma[..top]);
    let curved: Vec<[f32; 3]> = by_luma[..top]
        .iter()
        .map(|c| {
            [
                c[0].max(0.0).powf(PRELIT_POWER) * PRELIT_SCALE,
                c[1].max(0.0).powf(PRELIT_POWER) * PRELIT_SCALE,
                c[2].max(0.0).powf(PRELIT_POWER) * PRELIT_SCALE,
            ]
        })
        .collect();
    report(&format!("{label}, curved top decile"), &curved);
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
        .unwrap_or_else(|| "353,360,325".into())
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
        let mut raw: Vec<[f32; 3]> = Vec::new();
        for mesh in source
            .meshes
            .iter()
            .flat_map(oag_rcs::rcsmodel::Mesh::surfaces)
        {
            if mesh.material != slot {
                continue;
            }
            let has_cs = mesh
                .decl
                .as_ref()
                .is_some_and(oag_rcs::rcsmodel::VertexDecl::light_colour_set);
            if !has_cs {
                continue;
            }
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
            }
        }
        println!("  vertices: {}", raw.len());
        println!("  as decoded (r, g, b), no swap:");
        reconstruct("  as-decoded", &raw);
        let swapped: Vec<[f32; 3]> = raw.iter().map(|c| [c[2], c[1], c[0]]).collect();
        println!("  r/b swapped:");
        reconstruct("  r/b swapped", &swapped);
    }
    Ok(())
}
