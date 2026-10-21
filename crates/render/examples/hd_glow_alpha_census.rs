//! What the output alpha of every fragment program a race draws evaluates to.
//!
//! The bloom's glow mask on Wipeout HD is the scene target's alpha, written by
//! each material's own program. This walks every circuit's and ship's material
//! variants the race resolves and tallies, per material family, whether the
//! alpha lane is a constant, a texel expression or something else, so the
//! wiring in `oag_mesh::mesh::rcs` is checked against the whole disc.
//!
//! `cargo run --example hd_glow_alpha_census -p oag-render`

use std::collections::BTreeMap;

use oag_mesh::mesh;
use oag_rcs::rcsmaterial::fragment::Lane;
use oag_rcs::{rcsmaterial, rcsmodel};

const CIRCUITS: &[(&str, &str)] = &[
    ("DATA00", "/data/environments/amphiseum/track.vex"),
    ("DATA00", "/data/environments/modesto_heights/track.vex"),
    ("DATA00", "/data/environments/talons_junction/track.vex"),
    ("DATA00", "/data/environments/tech_de_ra/track.vex"),
    ("DATA02", "/data/environments/04_chenghou_project/track.vex"),
    ("DATA02", "/data/environments/01_vineta_k/track.vex"),
    ("DATA02", "/data/environments/15_anulpha_pass/track.vex"),
    ("DATA02", "/data/environments/03_track/track.vex"),
    ("DATA02", "/data/environments/02_track/track.vex"),
    ("DATA02", "/data/environments/10_sebenco_climb/track.vex"),
    ("DATA02", "/data/environments/05_ubermall/track.vex"),
    ("DATA02", "/data/environments/12_sol_2/track.vex"),
    ("DATA03", "/data/ships/harimau/ship.vex"),
];

fn class(lane: Lane) -> String {
    match lane {
        Lane::Unwritten => "unwritten".into(),
        Lane::Constant(v) => format!("const {v:.3}"),
        Lane::Texel {
            unit,
            channel,
            scale,
            bias,
        } => {
            format!("unit{unit}.{channel} x{scale:.3} +{bias:.3}")
        }
        Lane::Other => "other".into(),
    }
}

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut census: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (archive, name) in CIRCUITS {
        let spec = format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC");
        let Ok(data) = mesh::read_blob(&spec, name) else {
            continue;
        };
        let Ok(Some((model, _))) = mesh::rcs::scene_from(&spec, name, &data) else {
            continue;
        };
        let Some(geometry) = mesh::rcs::sibling_geometry(&spec, name, &data) else {
            continue;
        };
        let Ok(source) = rcsmodel::Model::parse(&geometry) else {
            continue;
        };
        for (slot, variant) in model.material_variants.iter().enumerate() {
            let Some(variant) = variant else { continue };
            let Some(material) = source.materials.get(slot) else {
                continue;
            };
            let Ok(blob) = mesh::read_blob(&spec, &format!("/{}", material.name)) else {
                continue;
            };
            let Some(program) =
                rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset)
            else {
                continue;
            };
            let lane = program.output_alpha(|hash| {
                material
                    .parameters
                    .iter()
                    .find(|p| p.hash == hash)
                    .map(|p| p.value)
            });
            let family = material.name.rsplit('/').next().unwrap_or("?").to_string();
            *census.entry((family, class(lane))).or_default() += 1;
        }
    }
    for ((family, class), n) in &census {
        println!("{n:4}  {family:55}  {class}");
    }
    Ok(())
}
