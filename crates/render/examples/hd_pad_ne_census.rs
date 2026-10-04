//! Which pad vertices carry `slots::PAD_NE`, with which glow colour, on each
//! HD circuit that authors pads: the check that every drawn pad chunk got its
//! `_ne` mask bound and a colour, rather than some circuits' pads drawing
//! without. `cargo run -p oag-render --example hd_pad_ne_census`

use oag_render::mesh::{self, slots};

const CIRCUITS: &[(&str, &str)] = &[
    ("DATA00", "talons_junction"),
    ("DATA00", "amphiseum"),
    ("DATA00", "modesto_heights"),
    ("DATA00", "tech_de_ra"),
    ("DATA02", "12_sol_2"),
    ("DATA02", "15_anulpha_pass"),
    ("DATA02", "01_vineta_k"),
];

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    for (archive, circuit) in CIRCUITS {
        let spec = format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC");
        let name = format!("/data/environments/{circuit}/track.vex");
        let Ok(data) = mesh::read_blob(&spec, &name) else {
            println!("{circuit}: not in {archive}");
            continue;
        };
        let model_blob = mesh::read_blob(&spec, &mesh::rcs::sibling_name(&name).unwrap())?;
        let mut read = |path: &str| mesh::read_blob(&spec, path).ok();
        for label in ["Speedup Pad", "Weapon Pad"] {
            let (model, report) = if label == "Speedup Pad" {
                mesh::rcs::build_pads(&name, &data, &model_blob, &mut read)?
            } else {
                mesh::rcs::build_weapon_pads(&name, &data, &model_blob, &mut read)?
            };
            let drawn = model.vertices.len();
            let flagged = model
                .vertices
                .iter()
                .filter(|v| v.slots & slots::PAD_NE != 0)
                .count();
            let mut colours: Vec<[f32; 3]> = model
                .vertices
                .iter()
                .filter(|v| v.slots & slots::PAD_NE != 0)
                .filter_map(|v| {
                    let index = slots::material_index(v.slots) as usize;
                    model.emissive.get(index.checked_sub(1)?).map(|e| e.tint)
                })
                .collect();
            colours.dedup();
            colours.sort_by(|a, b| a.partial_cmp(b).unwrap());
            colours.dedup();
            println!(
                "{circuit} {label}: {flagged} of {drawn} vertices PAD_NE, glow colours {colours:?}, \
                 bound {} unread {}",
                report.pad_ne_bound, report.pad_ne_unread
            );
        }
    }
    Ok(())
}
