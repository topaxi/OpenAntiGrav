//! How many of a Wipeout HD circuit's surfaces an additive emissive layer
//! would actually change.
//!
//! `scripts/hd_time_shapes.py` established that the mechanism is uniform - 290
//! of 291 materials take `time` into a texture coordinate and 96.7 % of blocks
//! combine the sample by accumulate. That answers "is this one change or a
//! classifier problem". It does **not** answer "is there a surface to show it
//! on", and this project has twice now measured a mechanism whose reach turned
//! out to be a rounding error (`uvScale`, and the whole-tile half of
//! `uvOffset`).
//!
//! A surface can only gain the layer where **all three** hold:
//!
//! 1. the resolved lit variant's fragment program accumulates the unit-1
//!    sample rather than selecting or modulating with it;
//! 2. `Model::lightmaps[slot]` is `Some` - the second `.gtf` was named *and*
//!    decoded, so there is something to add;
//! 3. `slots::SECOND_IS_LIGHTMAP` is clear, because adding the circuit's baked
//!    atlas paints a shadow map as a glow - the same refusal `skin::roles`
//!    already makes for albedo and coverage.
//!
//! ```sh
//! cargo run -p oag-render --example hd_emissive_reach
//! ```

use oag_formats::rcsmaterial::{self, fragment};
use oag_render::mesh::{self, slots};

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
];

/// `~crc32("time")`.
const TIME: u32 = 0x906b_67ba;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());

    let mut totals = [0usize; 5];
    for archive in ARCHIVES {
        let spec = format!("{image}:{archive}");
        let Ok(open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let tracks: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.starts_with("/data/environments/") && p.ends_with("/track.vex"))
            .cloned()
            .collect();
        for path in tracks {
            let Ok(data) = mesh::read_blob(&spec, &path) else {
                continue;
            };
            let Some(geometry) = mesh::rcs::sibling_geometry(&spec, &path, &data) else {
                continue;
            };
            let Ok((model, _report)) =
                mesh::rcs::build_scene(&path, &data, &geometry, &mut |name| {
                    mesh::read_blob(&spec, name).ok()
                })
            else {
                continue;
            };
            let Ok(geometry_model) = oag_formats::rcsmodel::Model::parse(&geometry) else {
                continue;
            };

            let mut declares_time = 0usize;
            let mut adds = 0usize;
            let mut has_second = 0usize;
            let mut not_lightmap = 0usize;
            let mut reachable = 0usize;
            let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();

            for (slot, material) in geometry_model.materials.iter().enumerate() {
                let Some(variant) = model.material_variants.get(slot).copied().flatten() else {
                    continue;
                };
                let blob = cache
                    .entry(material.name.clone())
                    .or_insert_with(|| mesh::read_blob(&spec, &format!("/{}", material.name)).ok())
                    .clone();
                let Some(blob) = blob else { continue };
                let declared = rcsmaterial::Declared::parse(&blob, variant.fragment.offset);
                if !declared
                    .as_ref()
                    .is_some_and(|d| d.parameters.contains(&TIME))
                {
                    continue;
                }
                declares_time += 1;

                let Some(program) = fragment::Program::parse(&blob, variant.fragment.offset) else {
                    continue;
                };
                // Unit 1 is the second texture on every material this reading
                // has met; `skin::units` is what says so per material and is
                // what a real implementation would ask.
                let accumulating = program.accumulates(1);
                adds += usize::from(accumulating);

                let second = model.lightmaps.get(slot).is_some_and(Option::is_some);
                has_second += usize::from(second);
                let roles = model.material_slots.get(slot).copied().unwrap_or(0);
                let lightmapped = roles & slots::SECOND_IS_LIGHTMAP != 0;
                not_lightmap += usize::from(!lightmapped);
                reachable += usize::from(accumulating && second && !lightmapped);
            }

            println!(
                "{path}\n  {declares_time} slots declare `time`; {adds} accumulate, \
                 {has_second} have a decoded second texture, {not_lightmap} are not \
                 lightmapped -> {reachable} would draw the layer"
            );
            for (n, v) in
                totals
                    .iter_mut()
                    .zip([declares_time, adds, has_second, not_lightmap, reachable])
            {
                *n += v;
            }
        }
    }

    println!(
        "\ntotals: {} declare `time`, {} accumulate, {} have a second texture, \
         {} not lightmapped, **{} would draw the layer**",
        totals[0], totals[1], totals[2], totals[3], totals[4]
    );
    Ok(())
}
