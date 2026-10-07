//! HD's pad `_ne` mask reaches the frame: bound third beside the lightmap,
//! read as the program reads it, coloured by the pad's own authored value.
//!
//! `#[ignore]`d: needs the PS3 disc image, and the render half also a GPU
//! adapter. Run by hand:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_pad_ne_ground_truth)'
//! ```
//!
//! # What each test would catch being dropped
//!
//! - **The binding test** fails if a pad material loses its lightmap to the
//!   `_ne` file (the trap `skin::picks` was built to avoid, and the one a
//!   lit-pixel floor does not see), if the mask is not the material's own
//!   `_ne` entry, or if a drawn pad vertex lacks `slots::PAD_NE`.
//! - **The colour test** fails if the glow colour stops being the material's
//!   own authored `W_Cycle`/`Colour` - the 2026-09-16 census, red on
//!   `talons_junction`'s `Weapon Pad`, cyan on `12_sol_2`'s both.
//! - **The render test** fails if the bars do not glow (the dropped wiring),
//!   or if the pad **outside the bars** stops matching what the lightmap and
//!   vertex normal alone draw - which is what dropping the lightmap, or
//!   mis-decoding the normal, does and a pixel count does not show.

mod archive_cache;

use std::path::Path;

use oag_mesh::mesh::{self, Model, slots};
use oag_mesh::mesh_render::{self, Anisotropy};

const IMAGE: &str = "data/images/hdfury-ps3-eu-dec.iso";

/// `(archive, track, glow colour of the Weapon Pad)`.
const CIRCUITS: &[(&str, &str, [f32; 3])] = &[
    ("DATA00", "talons_junction", [1.0, 0.0, 0.0]),
    ("DATA02", "12_sol_2", [0.0, 0.768_628, 0.992_157]),
];

fn image() -> Option<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(IMAGE);
    if path.exists() {
        return Some(path.display().to_string());
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// Both pad models of one circuit, plus the `.rcsmodel` they were built from.
fn pads(image: &str, archive: &str, track: &str) -> (Vec<(&'static str, Model)>, Vec<u8>) {
    let spec = format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC");
    let name = format!("/data/environments/{track}/track.vex");
    let data = archive_cache::read(&spec, &name).expect("reading the track");
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data).expect("a .rcsmodel beside it");
    let mut read = |path: &str| archive_cache::read(&spec, path);
    let (speedup, _) = mesh::rcs::build_pads(&name, &data, &geometry, &mut read).expect("speedup");
    let (weapon, _) =
        mesh::rcs::build_weapon_pads(&name, &data, &geometry, &mut read).expect("weapon");
    (
        vec![("Speedup Pad", speedup), ("Weapon Pad", weapon)],
        geometry,
    )
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_drawn_pad_binds_its_mask_third_and_keeps_its_lightmap() {
    let Some(image) = image() else { return };
    for (archive, track, _) in CIRCUITS {
        let (models, geometry) = pads(&image, archive, track);
        let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
        for (label, model) in &models {
            if model.vertices.is_empty() {
                continue;
            }
            let mut used = std::collections::BTreeSet::new();
            for v in &model.vertices {
                assert!(
                    v.slots & slots::PAD_NE != 0,
                    "{track} {label}: a drawn pad vertex without slots::PAD_NE"
                );
            }
            for draw in model
                .draws
                .iter()
                .chain(&model.alpha_tested_draws)
                .chain(&model.transparent_draws)
            {
                used.extend(draw.texture);
            }
            assert!(
                !used.is_empty(),
                "{track} {label}: no draw names a material"
            );
            for slot in used {
                let material = &rcs.materials[slot];
                let lightmap = material
                    .lightmap_entry()
                    .unwrap_or_else(|| panic!("{track} {label} slot {slot}: no lightmap entry"));
                let bound_second = model.lightmaps[slot]
                    .as_ref()
                    .unwrap_or_else(|| panic!("{track} {label} slot {slot}: lightmap dropped"));
                assert_eq!(
                    bound_second.label, lightmap,
                    "{track} {label} slot {slot}: the second binding is not the lightmap"
                );
                let mask = model.pad_masks[slot]
                    .as_ref()
                    .unwrap_or_else(|| panic!("{track} {label} slot {slot}: no _ne mask bound"));
                let ne = material
                    .samplers
                    .iter()
                    .find(|(hash, _)| *hash == 0xa2d5_55b9)
                    .and_then(|(_, path)| path.as_deref())
                    .expect("the material names its _ne entry");
                assert_eq!(
                    mask.label, ne,
                    "{track} {label} slot {slot}: wrong third binding"
                );
                assert!(
                    ne.ends_with("_ne.gtf"),
                    "{track} {label} slot {slot}: {ne} is not an _ne file"
                );
                assert_ne!(
                    mask.label, bound_second.label,
                    "{track} {label} slot {slot}: one texture bound twice"
                );
            }
        }
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_glow_colour_is_each_pads_own_authored_value() {
    let Some(image) = image() else { return };
    for (archive, track, expected) in CIRCUITS {
        let (models, _) = pads(&image, archive, track);
        let (_, weapon) = models.iter().find(|(l, _)| *l == "Weapon Pad").unwrap();
        assert!(!weapon.vertices.is_empty(), "{track} authors no Weapon Pad");
        for v in &weapon.vertices {
            let index = slots::material_index(v.slots) as usize;
            let layer = weapon.emissive[index - 1];
            assert_eq!(layer.tint, *expected, "{track}: Weapon Pad glow colour");
            assert_eq!(layer.rate, 0.0, "{track}: the pad glow does not scroll");
        }
    }
}

/// One node's geometry, rebased, so the close-up fills the frame.
fn isolate(model: &Model, range: std::ops::Range<u32>) -> Model {
    let mut out = model.clone();
    out.vertices = model.vertices[range.start as usize..range.end as usize].to_vec();
    let mut indices = Vec::new();
    let mut draws = Vec::new();
    for draw in model
        .draws
        .iter()
        .chain(&model.alpha_tested_draws)
        .chain(&model.transparent_draws)
    {
        let slice = &model.indices[draw.range.start as usize..draw.range.end as usize];
        if !slice.is_empty() && slice.iter().all(|i| range.contains(i)) {
            let start = indices.len() as u32;
            indices.extend(slice.iter().map(|i| i - range.start));
            draws.push(mesh::DrawCall {
                range: start..indices.len() as u32,
                ..draw.clone()
            });
        }
    }
    out.indices = indices;
    out.draws = draws;
    out.alpha_tested_draws = Vec::new();
    out.transparent_draws = Vec::new();
    out.node_vertex_ranges = Vec::new();
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for v in &out.vertices {
        for k in 0..3 {
            lo[k] = lo[k].min(v.position[k]);
            hi[k] = hi[k].max(v.position[k]);
        }
    }
    out.centre = [0, 1, 2].map(|k| (lo[k] + hi[k]) * 0.5);
    out.radius = (0..3).map(|k| (hi[k] - lo[k]) * 0.5).fold(0.5, f32::max);
    out
}

fn capture(model: &Model) -> Vec<u8> {
    mesh_render::capture_pixels_from(model, 640, 480, 0.6, 0.9, Anisotropy::Off, 0.0)
        .expect("capturing the pad")
}

fn luma(px: &[u8]) -> f32 {
    0.299 * f32::from(px[0]) + 0.587 * f32::from(px[1]) + 0.114 * f32::from(px[2])
}

/// A pixel the glow colour dominates: the red or cyan the bars take, well
/// past the grey plate.
fn glow_pixels(pixels: &[u8], colour: [f32; 3]) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|px| {
            let c = [f32::from(px[0]), f32::from(px[1]), f32::from(px[2])];
            let want = colour;
            // The channel the colour lacks is at least 60 below the one it has most of.
            let top = want.iter().cloned().fold(0.0, f32::max);
            let weak = want.iter().cloned().fold(1.0, f32::min);
            let hi = c[want.iter().position(|&w| w == top).unwrap()];
            let lo = c[want.iter().position(|&w| w == weak).unwrap()];
            hi > 120.0 && hi - lo > 60.0
        })
        .count()
}

#[test]
#[ignore = "needs a GPU adapter and a disc image in data/images/"]
fn the_bars_glow_and_the_rest_of_the_pad_is_lit_as_before() {
    let Some(image) = image() else { return };
    for (archive, track, colour) in CIRCUITS {
        let (models, _) = pads(&image, archive, track);
        let (_, weapon) = models
            .into_iter()
            .find(|(l, _)| *l == "Weapon Pad")
            .unwrap();
        let range = weapon
            .node_vertex_ranges
            .iter()
            .find(|r| !r.is_empty())
            .expect("a Weapon Pad node decoded")
            .clone();
        let with = isolate(&weapon, range);

        // What the same pad drew before this wiring: no mask, no flag.
        let mut without = with.clone();
        without.pad_masks = Vec::new();
        for v in &mut without.vertices {
            v.slots &= !slots::PAD_NE;
        }
        // The wiring with its glow colour zeroed: the normal map's effect on
        // the light alone, which is what "the rest of the pad" means.
        let mut unlit_bars = with.clone();
        for layer in &mut unlit_bars.emissive {
            layer.tint = [0.0; 3];
        }

        let a = capture(&with);
        let b = capture(&without);
        let d = capture(&unlit_bars);

        let glow_with = glow_pixels(&a, *colour);
        let glow_without = glow_pixels(&b, *colour);
        println!("{track}: glow pixels {glow_with} with the mask, {glow_without} without");
        assert!(
            glow_with > 200 && glow_with > 10 * glow_without.max(1),
            "{track}: the light bars do not glow ({glow_with} glow pixels with the mask, \
             {glow_without} without)"
        );

        // Over the pixels the pad covers (lit in the before picture), the
        // light-only render must match the before picture's mean luma. A
        // dropped lightmap, or a mis-decoded normal, moves this far more.
        let covered: Vec<usize> = b
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .filter(|(_, px)| u32::from(px[0]) + u32::from(px[1]) + u32::from(px[2]) > 100)
            .map(|(i, _)| i)
            .collect();
        assert!(
            covered.len() > 2000,
            "{track}: the pad covers {} pixels",
            covered.len()
        );
        let mean = |pixels: &[u8]| {
            covered
                .iter()
                .map(|&i| luma(&pixels[i * 4..i * 4 + 4]))
                .sum::<f32>()
                / covered.len() as f32
        };
        let (m_before, m_light) = (mean(&b), mean(&d));
        println!("{track}: mean luma over the pad, before {m_before:.1}, light only {m_light:.1}");
        assert!(
            (m_light - m_before).abs() < 0.12 * m_before,
            "{track}: outside the glow the pad's mean luma moved from {m_before:.1} to \
             {m_light:.1} - the lightmap or the normal is no longer what the program reads"
        );
    }
}
