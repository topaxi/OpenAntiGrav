//! Scratch probe: every vertex of the draws whose texture label contains a
//! needle, as `x y z nx ny nz r g b a sun_mask u v` lines on stdout.
//!
//! Used to compare this renderer's vertex inputs for Talon's Junction's
//! `etched_glass_tech` floor against the same draw read out of RPCS3.

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
    for (index, d) in model.transparent_draws.iter().enumerate() {
        let Some(texture) = d.texture.and_then(|t| model.textures.get(t)?.as_ref()) else {
            continue;
        };
        if !texture.label.contains(&want) {
            continue;
        }
        println!(
            "# draw {index} {} {} tri",
            texture.label,
            (d.range.end - d.range.start) / 3
        );
        for &i in &model.indices[d.range.start as usize..d.range.end as usize] {
            let v = &model.vertices[i as usize];
            println!(
                "{} {} {} {} {} {} {} {} {} {} {} {} {}",
                v.position[0],
                v.position[1],
                v.position[2],
                v.normal[0],
                v.normal[1],
                v.normal[2],
                v.colour[0],
                v.colour[1],
                v.colour[2],
                v.colour[3],
                v.sun_mask,
                v.texcoord[0],
                v.texcoord[1],
            );
        }
    }
    Ok(())
}
