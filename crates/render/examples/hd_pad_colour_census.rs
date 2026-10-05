//! Reads the Speedup Pad's and Weapon Pad's own colour parameter directly out
//! of each circuit's material record - never through the register trace
//! [`pads.md`](../../../docs/rendering/pads.md) used to read it on `12_sol_2`
//! alone - across every circuit on the disc that authors pads.
//!
//! **Settles pads.md's open question, and the answer is neither of the two on
//! the table.** The value is not always cyan (refuting "shared circuit
//! tint") and the hash is not fixed per pad type either (`W_Cycle`,
//! `0xce5c4410`, is the Weapon Pad's parameter on all twelve circuits, but the
//! Speedup Pad's own varies - `Colour` on six, `W_Cycle` itself on `02_track`,
//! and an unnamed `0x7611a2d8` on two more). What *is* fixed: Weapon Pad's
//! authored colour is red on 4 of 12 circuits (including `talons_junction`,
//! this project's default) and cyan/blue on the other 8; Speedup Pad's is
//! cyan (or near-white on `15_anulpha_pass`) everywhere it has geometry. See
//! `docs/rendering/pads.md`, "Which alpha channel gates the accumulate",
//! 2026-09-16 update, for the full table and the reading.
//!
//! ```sh
//! cargo run -p oag-render --example hd_pad_colour_census
//! ```

use oag_mesh::mesh;
use oag_rcs::{rcsmaterial, rcsmodel};
use oag_vex::vex;

/// Every `track.vex` on the disc, archive by archive - the full set, not a
/// sample, since a full sweep is a stronger negative than three circuits.
const TRACKS: &[(&str, &str)] = &[
    (
        "PS3_GAME/USRDIR/DATA00.PSARC",
        "/data/environments/amphiseum/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA00.PSARC",
        "/data/environments/modesto_heights/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA00.PSARC",
        "/data/environments/talons_junction/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA00.PSARC",
        "/data/environments/tech_de_ra/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA00.PSARC",
        "/data/environments/zone_1/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA00.PSARC",
        "/data/environments/zone_2/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA00.PSARC",
        "/data/environments/zone_3/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA00.PSARC",
        "/data/environments/zone_4/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA02.PSARC",
        "/data/environments/01_vineta_k/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA02.PSARC",
        "/data/environments/02_track/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA02.PSARC",
        "/data/environments/03_track/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA02.PSARC",
        "/data/environments/04_chenghou_project/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA02.PSARC",
        "/data/environments/05_ubermall/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA02.PSARC",
        "/data/environments/10_sebenco_climb/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA02.PSARC",
        "/data/environments/12_sol_2/track.vex",
    ),
    (
        "PS3_GAME/USRDIR/DATA02.PSARC",
        "/data/environments/15_anulpha_pass/track.vex",
    ),
];

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());

    let mut rows: Vec<(String, &str, usize, u32, [f32; 4])> = Vec::new();
    let mut no_pads = Vec::new();
    let mut multi_param = Vec::new();

    for (archive, track) in TRACKS {
        let spec = format!("{image}:{archive}");
        let Ok(data) = mesh::read_blob(&spec, track) else {
            continue;
        };
        let Some(model_name) = mesh::rcs::sibling_name(track) else {
            continue;
        };
        let Ok(model_blob) = mesh::read_blob(&spec, &model_name) else {
            continue;
        };
        let Ok(model) = rcsmodel::Model::parse(&model_blob) else {
            continue;
        };
        let Ok(classes) = vex::classes_of(&data) else {
            continue;
        };
        let Ok(nodes) = vex::nodes(&data) else {
            continue;
        };
        let order = vex::byte_order(&data);
        let circuit = track
            .trim_start_matches("/data/environments/")
            .trim_end_matches("/track.vex")
            .to_string();

        let mut found_any = false;
        for (label, class_id) in [
            ("Speedup Pad", classes.speedup_pad),
            ("Weapon Pad", classes.weapon_pad),
        ] {
            let Some(class_id) = class_id else { continue };
            for node in nodes.iter().filter(|n| n.class_id == class_id) {
                let payload = &data[node.payload()];
                if payload.len() < 0x34 {
                    continue;
                }
                let hash = order.u32(payload, 0x30);
                let Some(chunk) = model.mesh(hash) else {
                    continue;
                };
                for surface in chunk.surfaces() {
                    let slot = surface.material as usize;
                    let Some(material) = model.materials.get(slot) else {
                        continue;
                    };
                    found_any = true;
                    if material.parameters.len() != 1 {
                        multi_param.push((circuit.clone(), label, material.parameters.len()));
                    }
                    for p in &material.parameters {
                        rows.push((circuit.clone(), label, slot, p.hash, p.value));
                    }
                }
            }
        }
        if !found_any {
            no_pads.push(circuit);
        }
    }

    rows.sort_by(|a, b| (a.0.as_str(), a.1, a.2, a.3).cmp(&(b.0.as_str(), b.1, b.2, b.3)));
    rows.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1 && a.2 == b.2 && a.3 == b.3 && a.4 == b.4);
    println!("circuit                label        slot   hash        name         value");
    for (circuit, label, slot, hash, value) in &rows {
        let name = rcsmaterial::names::parameter_name(*hash).unwrap_or("?");
        println!("{circuit:<22} {label:<12} {slot:>5}  {hash:#010x}  {name:<12} {value:?}");
    }

    println!("\ncircuits with no pad geometry at all: {no_pads:?}");
    println!("materials whose pad carries other than exactly one parameter: {multi_param:?}");

    let mut distinct_values: Vec<[f32; 4]> = rows.iter().map(|(_, _, _, _, v)| *v).collect();
    distinct_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    distinct_values.dedup();
    println!(
        "\n{} rows, {} distinct authored values: {:?}",
        rows.len(),
        distinct_values.len(),
        distinct_values
    );
    let distinct_hashes_by_label: std::collections::BTreeMap<
        &str,
        std::collections::BTreeSet<u32>,
    > = {
        let mut m: std::collections::BTreeMap<&str, std::collections::BTreeSet<u32>> =
            Default::default();
        for (_, label, _, hash, _) in &rows {
            m.entry(label).or_default().insert(*hash);
        }
        m
    };
    for (label, hashes) in &distinct_hashes_by_label {
        println!("{label}: distinct parameter hashes across the disc: {hashes:#010x?}");
    }
    Ok(())
}
