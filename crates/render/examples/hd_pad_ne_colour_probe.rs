//! Scratch probe behind [`pads.md`](../../../docs/rendering/pads.md)'s "What
//! binds the `_ne` file": the alpha channel of a pad's `_ne` mask is already
//! established to cover exactly the light bars. This decodes the same file's
//! **colour** channels at those alpha-selected texels and reports it, as
//! actual numbers - the check that decides whether the maintainer's "light up
//! red" report is the `_ne` file's own paint or comes from somewhere else.

use oag_mesh::mesh;
use oag_texture::gtf;

fn report(spec: &str, path: &str) -> anyhow::Result<()> {
    let blob = mesh::read_blob(spec, path)?;
    let parsed = gtf::Gtf::parse(&blob)?;
    let texture = parsed
        .only()
        .ok_or_else(|| anyhow::anyhow!("not one texture"))?;
    let rgba = texture.to_rgba(&blob)?;
    let (width, height) = texture.level_size(0);
    println!(
        "== {path} ({width}x{height}, {:?}, {} texels) ==",
        texture.format,
        rgba.len()
    );

    // Whole-texture stats regardless of alpha, to tell "the RGB is deliberate
    // paint over the bars, incidentally varied" from "the RGB is a tangent-
    // space normal map that varies everywhere, and the alpha mask is the
    // only channel that means anything about the light bars".
    {
        let mut min = [255u8; 3];
        let mut max = [0u8; 3];
        let mut sum = [0u64; 3];
        let mut distinct = std::collections::HashSet::new();
        for p in &rgba {
            for k in 0..3 {
                min[k] = min[k].min(p[k]);
                max[k] = max[k].max(p[k]);
                sum[k] += p[k] as u64;
            }
            distinct.insert(*p);
        }
        let n = rgba.len() as u64;
        let mean = [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8];
        println!(
            "  whole-texture RGB: min={min:?} max={max:?} mean={mean:?} distinct_rgba={}",
            distinct.len()
        );
    }

    // Alpha-selected texels: the mask over the light bars, per pads.md.
    // Report the full distribution, not just one extremum, so a mixed bar
    // (anti-aliased edges, etc.) doesn't get summarised away.
    let mut selected: Vec<[u8; 4]> = rgba.iter().copied().filter(|p| p[3] > 0).collect();
    if selected.is_empty() {
        println!("  no texel has alpha > 0 - mask is empty, unexpected");
        return Ok(());
    }
    selected.sort_by_key(|p| std::cmp::Reverse(p[3]));

    let fully_opaque: Vec<[u8; 4]> = selected.iter().copied().filter(|p| p[3] == 255).collect();
    let sample_set = if fully_opaque.is_empty() {
        &selected
    } else {
        &fully_opaque
    };

    let mut min = [255u8; 3];
    let mut max = [0u8; 3];
    let mut sum = [0u64; 3];
    for p in sample_set {
        for k in 0..3 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
            sum[k] += p[k] as u64;
        }
    }
    let n = sample_set.len() as u64;
    let mean = [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8];

    println!(
        "  alpha>0 texels: {} total, {} at alpha==255",
        selected.len(),
        fully_opaque.len()
    );
    println!(
        "  sampled from: {} texels (alpha==255 if any, else all alpha>0)",
        n
    );
    println!("  RGB min={min:?} max={max:?} mean={mean:?}");
    println!(
        "  most saturated alpha-selected texel: {:?}",
        selected
            .iter()
            .filter(|p| p[3] > 0)
            .max_by_key(|p| {
                let (r, g, b) = (p[0] as i32, p[1] as i32, p[2] as i32);
                (r.max(g).max(b)) - (r.min(g).min(b))
            })
            .copied()
    );
    println!(
        "  highest-alpha texel(s), first 5: {:?}",
        &selected[..selected.len().min(5)]
    );

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let spec = format!("{image}:PS3_GAME/USRDIR/DATA02.PSARC");

    for path in [
        "/data/environments/12_sol_2/hd_textures/dds/ds_weaponup_ne.gtf",
        "/data/environments/12_sol_2/hd_textures/dds/ds_speedup_ne.gtf",
        // Also the diffuse files' own alpha: `mesh.wesl`'s documented
        // ADD_SECOND shape gates the glow by `first.a` (unit 0, the
        // diffuse), not by unit 1's own alpha - see the tint probe and
        // pads.md for why this matters.
        "/data/environments/12_sol_2/hd_textures/dds/ds_weaponup_cs.gtf",
        "/data/environments/12_sol_2/hd_textures/dds/ds_speedup_cs.gtf",
    ] {
        report(&spec, path)?;
    }

    Ok(())
}
