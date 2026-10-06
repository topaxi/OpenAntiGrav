//! Scratch probe: the alpha histogram of every `Transparency::Mode2` material
//! on the Wipeout HD disc, in the channel the shader actually tests.
//!
//! Mode 2 is a `GL_GREATER`/`0.5` alpha-test cutout - see
//! `docs/ghidra/functions/ps3-hdfury-eu/material-state.md`. Whether that
//! cutoff *changes the picture* depends on the distribution of the coverage
//! channel: a bimodal `0`/`255` texture looks the same tested or blended and
//! differs only in depth, while a texture with a real gradient loses that
//! gradient to the test. `nr_crowd_bustle` was measured; this measures the
//! rest.
//!
//! The channel is not assumed to be entry 0's `a`: `mesh::slots` carries which
//! of the two bound textures the coverage comes from and which of its four
//! channels, read off each material's own fragment microcode, and this reads
//! the same bits `mesh.wgsl` does.

use oag_mesh::mesh::{self, ModelTexture, slots};
use oag_rcs::rcsmodel::{Material, Transparency};
use std::collections::{BTreeMap, BTreeSet};

/// The circuit directories, copied from `oag_hd::names::ENVIRONMENTS` because
/// `oag-render` does not depend on `oag-hd` and an example may not add one.
const ENVIRONMENTS: &[&str] = &[
    "amphiseum",
    "modesto_heights",
    "talons_junction",
    "tech_de_ra",
    "zone_1",
    "zone_2",
    "zone_3",
    "zone_4",
    "01_vineta_k",
    "02_track",
    "03_track",
    "04_chenghou_project",
    "05_ubermall",
    "10_sebenco_climb",
    "12_sol_2",
    "15_anulpha_pass",
];

/// One material name's running histogram of its coverage channel.
#[derive(Default)]
struct Row {
    /// 256 buckets, one per 8-bit value.
    bins: Vec<u64>,
    /// Distinct `(alpha_func, alpha_ref bits)` pairs seen under this name.
    funcs: BTreeSet<(u32, u32)>,
    /// Distinct `(from_second, channel)` readings seen under this name.
    channels: BTreeSet<(bool, u32)>,
    /// Distinct coverage `.gtf` paths.
    textures: BTreeSet<String>,
    /// Which archive/circuit pairs carried the name.
    circuits: BTreeSet<String>,
    /// How many material records carried it.
    records: usize,
    /// How many `.rcsmodel` chunk surfaces name one of those records - the
    /// unit `docs/formats/rcsmodel.md`'s earlier mode-1/mode-2 census counted,
    /// and not the same number as [`Self::records`]: a material table carries
    /// entries no chunk uses.
    chunks: usize,
}

fn stem(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

/// Every texel of level 0 as flat RGBA bytes, decoded once per texture.
fn texels<'a>(
    cache: &'a mut BTreeMap<usize, Vec<u8>>,
    texture: &std::sync::Arc<ModelTexture>,
) -> &'a [u8] {
    cache
        .entry(std::sync::Arc::as_ptr(texture) as usize)
        .or_insert_with(|| {
            texture
                .to_rgba()
                .map(std::borrow::Cow::into_owned)
                .unwrap_or_default()
        })
}

/// Which `.gtf` path a material's coverage comes out of, for the report.
fn coverage_path(material: &Material, from_second: bool) -> String {
    if from_second {
        material
            .second_texture
            .clone()
            .unwrap_or_else(|| "<none>".into())
    } else {
        material.texture.clone()
    }
}

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());

    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    let mut mode2_records = 0usize;

    for archive in ["DATA00", "DATA01", "DATA02"] {
        let spec = format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC");
        for environment in ENVIRONMENTS {
            let name = format!("/data/environments/{environment}/track.vex");
            let Ok(data) = mesh::read_blob(&spec, &name) else {
                continue;
            };
            let Some(geometry) = mesh::rcs::sibling_geometry(&spec, &name, &data) else {
                continue;
            };
            let Ok(source) = oag_rcs::rcsmodel::Model::parse(&geometry) else {
                continue;
            };
            let Ok(Some((model, _))) = mesh::rcs::scene_from(&spec, &name, &data) else {
                continue;
            };
            // How many chunk surfaces name each material slot, so the row
            // below can be compared against a census counted in chunks.
            let mut per_slot: BTreeMap<usize, usize> = BTreeMap::new();
            for chunk in &source.meshes {
                for surface in chunk.surfaces() {
                    *per_slot.entry(surface.material as usize).or_default() += 1;
                }
            }
            let mut cache: BTreeMap<usize, Vec<u8>> = BTreeMap::new();
            for (slot, material) in source.materials.iter().enumerate() {
                if material.transparency() != Some(Transparency::Mode2) {
                    continue;
                }
                mode2_records += 1;
                let row = rows.entry(stem(&material.name).to_string()).or_default();
                if row.bins.is_empty() {
                    row.bins = vec![0; 256];
                }
                row.records += 1;
                row.chunks += per_slot.get(&slot).copied().unwrap_or(0);
                row.circuits.insert(format!("{archive}/{environment}"));
                row.funcs
                    .insert((material.alpha_func, material.alpha_ref.to_bits()));
                let bits = model
                    .material_slots
                    .get(slot)
                    .copied()
                    .unwrap_or(slots::DEFAULT);
                let from_second = bits & slots::ALPHA_FROM_SECOND != 0;
                let channel = (bits >> 3) & 3;
                row.channels.insert((from_second, channel));
                let coverage = if from_second {
                    model.lightmaps.get(slot).and_then(Option::as_ref)
                } else {
                    model.textures.get(slot).and_then(Option::as_ref)
                };
                let Some(coverage) = coverage else { continue };
                row.textures.insert(coverage_path(material, from_second));
                for t in texels(&mut cache, coverage).as_chunks::<4>().0 {
                    row.bins[usize::from(t[channel as usize])] += 1;
                }
            }
        }
    }

    println!(
        "{mode2_records} Transparency::Mode2 material record(s), {} distinct name(s)\n",
        rows.len()
    );
    for (name, row) in &rows {
        let total: u64 = row.bins.iter().sum();
        let zero = row.bins[0];
        let full = row.bins[255];
        let between = total - zero - full;
        // `GL_GREATER` against `0.5`: 8-bit 128..=255 pass, 0..=127 fail.
        let passes: u64 = row.bins[128..].iter().sum();
        println!(
            "{name}  ({} record(s), {} chunk surface(s), {} circuit(s))",
            row.records,
            row.chunks,
            row.circuits.len()
        );
        println!(
            "  alpha_func/ref {:?}",
            row.funcs
                .iter()
                .map(|&(f, r)| format!("{f:#06x}/{}", f32::from_bits(r)))
                .collect::<Vec<_>>()
        );
        println!(
            "  coverage {:?}  texture(s) {:?}",
            row.channels
                .iter()
                .map(|&(second, c)| format!(
                    "{}.{}",
                    if second { "second" } else { "first" },
                    ["r", "g", "b", "a"][c as usize]
                ))
                .collect::<Vec<_>>(),
            row.textures
        );
        if total == 0 {
            println!("  no coverage texture decoded\n");
            continue;
        }
        let pct = |n: u64| n as f64 / total as f64 * 100.0;
        // **The mean, which is what a fully averaged mip level converges to.**
        // A cutout is discarded wholesale under minification when this falls
        // below the reference and drawn solid when it rises above it - see
        // `oag_mesh::mesh::rcs::cutout`.
        let mean: f64 = row
            .bins
            .iter()
            .enumerate()
            .map(|(v, &n)| v as f64 * n as f64)
            .sum::<f64>()
            / (total as f64 * 255.0);
        println!("  mean alpha {mean:.4}");
        println!(
            "  {total} texel(s): 0 {:.1}%  255 {:.1}%  between {:.2}%  |  GL_GREATER 0.5 keeps {:.1}%",
            pct(zero),
            pct(full),
            pct(between),
            pct(passes)
        );
        if between > 0 {
            let lo = row.bins[1..255].iter().position(|&n| n > 0).unwrap_or(0) + 1;
            let hi = 254
                - row.bins[1..255]
                    .iter()
                    .rev()
                    .position(|&n| n > 0)
                    .unwrap_or(0);
            println!("  intermediate values span {lo}..={hi}");
        }
        println!();
    }
    Ok(())
}
