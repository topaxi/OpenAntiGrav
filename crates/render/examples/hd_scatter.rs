//! Scratch probe: how a draw's vertices are spread about their own centroid.
//!
//! A bounding sphere of thousands of units around a square unit of surface is
//! either a moving object's honest whole-path extent or scattered geometry,
//! and the distance histogram tells them apart: a real object's vertices are
//! spread evenly through its radius, while a mis-decoded one is a tight
//! cluster with a handful of outliers thrown across the world.

use oag_mesh::mesh;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/15_anulpha_pass/track.vex".into());
    let want: usize = args.next().unwrap_or_else(|| "362".into()).parse()?;

    let data = mesh::read_blob(&spec, &name)?;
    let (model, _) = mesh::rcs::scene_from(&spec, &name, &data)?
        .ok_or_else(|| anyhow::anyhow!("{name}: not a PS3 model"))?;

    let mut shown = 0;
    for draws in [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ] {
        for d in draws {
            if d.texture != Some(want) || d.bounds.radius < 500.0 {
                continue;
            }
            let centre = d.bounds.centre;
            let mut distances: Vec<f32> = model.indices
                [d.range.start as usize..d.range.end as usize]
                .iter()
                .map(|&i| {
                    let p = model.vertices[i as usize].position;
                    ((p[0] - centre[0]).powi(2)
                        + (p[1] - centre[1]).powi(2)
                        + (p[2] - centre[2]).powi(2))
                    .sqrt()
                })
                .collect();
            distances.sort_by(f32::total_cmp);
            let n = distances.len();
            let at = |q: f64| distances[((n as f64 - 1.0) * q) as usize];
            println!(
                "slot {want} draw at {:?} radius {:.0}, {n} vertex reference(s)",
                centre, d.bounds.radius
            );
            println!(
                "  distance from centroid: p50 {:.1}  p90 {:.1}  p99 {:.1}  max {:.1}",
                at(0.50),
                at(0.90),
                at(0.99),
                distances[n - 1]
            );
            let far = distances.iter().filter(|&&v| v > at(0.50) * 10.0).count();
            println!(
                "  {far} of {n} sit beyond ten times the median - {:.2}%",
                far as f64 / n as f64 * 100.0
            );
            shown += 1;
            if shown >= 3 {
                return Ok(());
            }
        }
    }
    Ok(())
}
