//! Scratch probe: every drawn material slot's own role reading, one row each -
//! the join key `scripts/hd-material-probe.py` groups its per-pixel tone
//! deltas by, rather than by slot ordinal alone.
//!
//! A new file rather than a change to `hd_slot_list.rs`: that file answers
//! "which slot paints this pixel" and other lanes read it tonight, while this
//! one answers "what does that slot's own fragment program compute" - `Model`
//! already carries the answer, off `mesh::rcs::skin::roles`, so this is a
//! dump of public fields rather than a second reading of the microcode.

use oag_mesh::mesh;
use oag_mesh::mesh::slots;

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

    // `Fog.Fog Color`, straight off the same `.envsettings` the race loads it
    // from (`crates/raceplay/src/load/environment.rs`'s `envsettings_fog`) -
    // the anchor `scripts/hd-material-probe.py` needs to classify a tinted
    // pixel that fog has pulled off its material's own flat colour, rather
    // than one this probe fitted from the picture.
    if let Some(envsettings_name) = name
        .strip_suffix(".vex")
        .map(|s| format!("{s}.envsettings"))
    {
        if let Ok(blob) = mesh::read_blob(&spec, &envsettings_name)
            && let Ok(text) = String::from_utf8(blob)
            && let Ok(env) = oag_tables::envsettings::EnvSettings::parse(&text)
            && let Some(colour) = env.vec3(oag_tables::envsettings::FOG_COLOUR)
        {
            println!(
                "# fog_colour_linear\t{}\t{}\t{}",
                colour[0], colour[1], colour[2]
            );
        } else {
            println!("# fog_colour_linear\tnone");
        }
    }

    // Which list each drawn slot actually issues its draw call(s) from -
    // `isolate::tint`'s exact-match segmentation is only trustworthy for
    // `opaque`: a cutout slot's holes close under a flat 1x1 texture and a
    // blended slot's coverage goes fully opaque, so either would claim
    // background pixels the real frame shows through. A slot mixing lists
    // keeps its first, which never happens on Talon's Junction (checked
    // below) - a slot mixing lists would need its pixels split by draw
    // rather than by slot, which this probe does not attempt.
    let mut list_of: std::collections::BTreeMap<usize, &'static str> = Default::default();
    let mut mixed = 0usize;
    for (list, draws) in [
        ("opaque", &model.draws),
        ("cutout", &model.alpha_tested_draws),
        ("blended", &model.transparent_draws),
    ] {
        for d in draws {
            let Some(slot) = d.texture else { continue };
            match list_of.get(&slot) {
                Some(&prior) if prior != list => mixed += 1,
                _ => {
                    list_of.entry(slot).or_insert(list);
                }
            }
        }
    }

    println!("slot\tlist\tmaterial\ttexture\troles\tspecular_exponent\tvariant_resolved");
    for (&slot, &list) in &list_of {
        let material = source.materials.get(slot);
        let packed = model
            .material_slots
            .get(slot)
            .copied()
            .unwrap_or(slots::DEFAULT);
        let mut roles = Vec::new();
        if packed & slots::SECOND_IS_LIGHTMAP != 0 {
            roles.push("lightmap");
        }
        if packed & slots::ALBEDO_FROM_SECOND != 0 {
            roles.push("albedo_from_second");
        }
        if packed & slots::ALPHA_FROM_SECOND != 0 {
            roles.push("alpha_from_second");
        }
        if packed & slots::FLIP_V != 0 {
            roles.push("flip_v");
        }
        if packed & slots::EMISSIVE == slots::EMISSIVE {
            roles.push("emissive");
        } else {
            if packed & slots::NO_AMBIENT != 0 {
                roles.push("no_ambient");
            }
            if packed & slots::NO_SUN != 0 {
                roles.push("no_sun");
            }
        }
        if packed & slots::ADD_SECOND != 0 {
            roles.push("add_second");
        }
        if roles.is_empty() {
            roles.push("lit_default");
        }
        let specular = model
            .material_specular_exponent
            .get(slot)
            .copied()
            .unwrap_or(mesh::DEFAULT_SPECULAR_EXPONENT);
        let resolved = model
            .material_variants
            .get(slot)
            .is_some_and(Option::is_some);
        println!(
            "{slot}\t{list}\t{}\t{}\t{}\t{specular:.1}\t{resolved}",
            material.map_or("<none>", |m| m.name.as_str()),
            material.map_or("", |m| m.texture.as_str()),
            roles.join("|"),
        );
    }
    if mixed > 0 {
        eprintln!(
            "warning: {mixed} draw(s) named a slot already seen under a different list - \
             that slot's row above is one list only and its pixels are not purely that list"
        );
    }
    Ok(())
}
