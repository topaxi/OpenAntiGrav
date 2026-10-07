//! Scratch probe: every material slot that actually issues a draw, with its
//! name, its blend, and its screen-independent size - the key an
//! `OAG_TINT_MATERIALS` frame is read against.
//!
//! Matching a tinted pixel against all 400 ordinals invents answers, because
//! most ordinals never draw; matching against this list does not.

use oag_mesh::mesh;

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

    let mut rows: std::collections::BTreeMap<usize, (usize, &'static str, f32)> =
        std::collections::BTreeMap::new();
    for (list, draws) in [
        ("opaque", &model.draws),
        ("cutout", &model.alpha_tested_draws),
        ("blended", &model.transparent_draws),
    ] {
        for d in draws {
            let Some(slot) = d.texture else {
                if std::env::var("OAG_UNTEXTURED").is_ok() {
                    let c = d.bounds.centre;
                    println!(
                        "untextured {list:<8} centre ({:.1}, {:.1}, {:.1}) radius {:.0} range {}..{}",
                        c[0], c[1], c[2], d.bounds.radius, d.range.start, d.range.end
                    );
                }
                continue;
            };
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
            let entry = rows.entry(slot).or_insert((0, list, 0.0));
            entry.0 += 1;
            entry.2 = entry.2.max(d.bounds.radius);
        }
    }
    for (slot, (count, list, radius)) in rows {
        let material = source.materials.get(slot);
        let hue = (slot as f32 * 0.618_034).fract() * 6.0;
        println!(
            "slot {slot:>4} hue {hue:5.3} {count:>3} {list:<8} radius {radius:>6.0}  {}  {}",
            material.map_or("<none>", |m| m.name.as_str()),
            material.map_or("", |m| m.texture.as_str()),
        );
    }
    Ok(())
}
