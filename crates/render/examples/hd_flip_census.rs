//! Scratch probe: which material variants flip the diffuse texture coordinate.
//!
//! `oag_formats::rcsmaterial::vertex::Program::flips` reads the `v = 1 - v`
//! out of the resolved vertex block. This runs it over every drawn slot of a
//! circuit so the reading can be checked against surfaces whose picture is
//! already known to be right or wrong.

use oag_render::mesh;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());

    let data = mesh::read_blob(&spec, &name)?;
    let (model, _) = mesh::rcs::scene_from(&spec, &name, &data)?
        .ok_or_else(|| anyhow::anyhow!("{name}: not a PS3 model"))?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("no sibling"))?;
    let source = oag_formats::rcsmodel::Model::parse(&geometry)?;

    let mut flipped = 0;
    let mut upright = 0;
    let mut unread = 0;
    for (slot, variant) in model.material_variants.iter().enumerate() {
        let Some(material) = source.materials.get(slot) else {
            continue;
        };
        let Some(variant) = variant else { continue };
        let Ok(blob) = mesh::read_blob(&spec, &format!("/{}", material.name)) else {
            unread += 1;
            continue;
        };
        let Some(program) = oag_formats::rcsmaterial::vertex::Program::of(&blob, variant.vertex)
        else {
            unread += 1;
            continue;
        };
        let hash = oag_formats::rcsmaterial::name_hash("Uv1");
        let flips = program.flips(hash);
        if flips {
            flipped += 1;
        } else {
            upright += 1;
        }
        if std::env::var_os("OAG_VERBOSE").is_some() {
            println!(
                "  slot {slot:>4} {} {}",
                if flips { "FLIP  " } else { "upright" },
                material.name.rsplit('/').next().unwrap_or_default()
            );
        }
    }
    println!("{flipped} variant(s) flip the diffuse v, {upright} do not, {unread} unread");
    Ok(())
}
