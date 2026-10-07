//! Wipeout HD's additive glow layer, from the material record to a lit pixel.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_emissive_glow_ground_truth)'
//! ```
//!
//! # What this measures
//!
//! HD's emissive family **adds** its second texture to the albedo where
//! `mesh.wgsl` *selects* between the two, so before 2026-08-31 every one of
//! those surfaces drew its diffuse and its glow was simply absent. The
//! mechanism is on `docs/formats/rcsmaterial.md`, "A surface scrolls off an
//! engine `time`"; three claims here:
//!
//! 1. **A circuit builds a glow table**, and the materials that get an entry
//!    are the ones whose own microcode accumulates - not the ones whose name
//!    says `uvanim`.
//! 2. **A material never gets one over the circuit's baked atlas.** Adding a
//!    light term and a sun mask to an albedo paints a shadow map as a glow,
//!    which is the failure `skin::roles` already refuses for albedo and
//!    coverage.
//! 3. **It reaches the frame**, and it moves: two clocks are two pictures.

mod archive_cache;

use std::path::{Path, PathBuf};

use oag_mesh::mesh::{self, slots};

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

/// Circuits with a glow table, and how many distinct layers each authors after
/// deduplication. Modesto Heights is the busiest on the disc.
const CIRCUITS: &[(&str, &str)] = &[
    (
        "DATA00.PSARC",
        "/data/environments/talons_junction/track.vex",
    ),
    (
        "DATA00.PSARC",
        "/data/environments/modesto_heights/track.vex",
    ),
    (
        "DATA02.PSARC",
        "/data/environments/15_anulpha_pass/track.vex",
    ),
];

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

fn build(image: &Path, archive: &str, path: &str) -> Option<mesh::Model> {
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let data = archive_cache::read(&spec, path)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    let (model, _) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        archive_cache::read(&spec, name)
    })
    .expect("the scene builds");
    Some(model)
}

/// Claims 1 and 2: the table exists, the bit and the index agree, and no
/// lightmapped material is in it.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn a_circuit_authors_a_glow_table_and_never_over_its_lightmap() {
    let Some(image) = image() else { return };
    let mut circuits = 0usize;
    for (archive, path) in CIRCUITS {
        let Some(model) = build(&image, archive, path) else {
            continue;
        };
        circuits += 1;
        let mut alpha_from_second = 0usize;
        let carrying: Vec<usize> = model
            .material_slots
            .iter()
            .enumerate()
            .filter(|(_, packed)| **packed & slots::ADD_SECOND != 0)
            .map(|(slot, _)| slot)
            .collect();
        println!(
            "{path}: {} distinct glow layers over {} material slots",
            model.emissive.len(),
            carrying.len()
        );
        assert!(
            !model.emissive.is_empty(),
            "{path} builds no glow table at all"
        );
        assert!(
            !carrying.is_empty(),
            "{path} has a table nothing indexes into"
        );

        for slot in carrying {
            let packed = model.material_slots[slot];
            // Claim 2, the one that would paint a shadow map as a glow.
            assert_eq!(
                packed & slots::SECOND_IS_LIGHTMAP,
                0,
                "{path} slot {slot} adds the circuit's baked atlas"
            );
            // The bit and the index move together or the shader reads slot 0
            // and adds nothing - or worse, reads a neighbour's colour.
            let index = slots::material_index(packed);
            assert!(
                index > 0 && (index as usize) <= model.emissive.len(),
                "{path} slot {slot} carries ADD_SECOND with index {index} against \
                 a table of {}",
                model.emissive.len()
            );
            // There has to be something to add.
            assert!(
                model.lightmaps.get(slot).is_some_and(Option::is_some),
                "{path} slot {slot} adds a texture that did not decode"
            );
            // **Adding and replacing the picture are exclusive**, and this
            // asserts it rather than inferring it from what `Texel::merge`
            // does with an accumulate: both bits firing would sample unit 1
            // twice, once as the surface and once as a glow on top of itself.
            // Zero of the 131 slots measured across these three circuits do.
            assert_eq!(
                packed & slots::ALBEDO_FROM_SECOND,
                0,
                "{path} slot {slot} both adds and replaces its picture"
            );
            // **The alpha is a different question and does co-occur**, on 30
            // of those slots: a material can take its coverage from unit 1 and
            // add unit 1's colour as a glow, which is two channels rather than
            // two pictures. Asserted as a measurement so a future change that
            // stopped it happening is noticed.
            alpha_from_second += usize::from(packed & slots::ALPHA_FROM_SECOND != 0);
        }

        // The index lives in the high half and the roles in the low half; a
        // role bit that had crept above the split would be read as an index.
        // **`slots::ZONE_TRACK` (bit 9) is deliberately excluded**: it is the
        // one bit in this word that is per chunk, not per material
        // (`mesh::rcs::surface` ORs it in after `model.material_slots` is
        // built), so it never appears here and its absence is not a gap in
        // this reading. `slots::FACING_RAMP_SHEEN` (bit 10) is a tenth role
        // bit, added 2026-09-17 for the glass family's facing-ramp combine -
        // see `mesh::rcs::glass_sheen`. `slots::PAD_NE` (bit 14) is the
        // eleventh: since 2026-10-05 the scene binds it on the speed-pad
        // materials of the four original circuits, whose pad nodes name no
        // chunk (`hd_original_speed_pad_ground_truth`). `slots::MAG_WAVE` and
        // `slots::MAG_LOOP` (bits 17 and 18) are the twelfth and thirteenth:
        // the magstrip wave, 2026-10-05 - see `mesh::rcs::mag_wave`.
        // `slots::LIGHT_CONE` (bit 19) is the fourteenth: `dc_lightcone`'s
        // combine - see `mesh::rcs::light_cone`.
        for packed in &model.material_slots {
            assert_eq!(
                packed
                    & slots::ROLE_MASK
                    & !(0x1ffu32
                        | slots::FACING_RAMP_SHEEN
                        | slots::PAD_NE
                        | slots::MAG_WAVE
                        | slots::MAG_LOOP
                        | slots::LIGHT_CONE),
                0,
                "a role bit outside the ten this reading defines"
            );
        }
        assert!(
            model.emissive.len() < mesh::rcs::EMISSIVE_LIMIT,
            "{path} needs more glow slots than the shader's table holds"
        );
        println!("  {alpha_from_second} of them also take their coverage from unit 1");
    }
    assert_eq!(circuits, CIRCUITS.len(), "every circuit was reachable");
}

/// Borrowed as `[u8; 4]` chunks, so the fold above reads two frames at once.
fn dark_pixels(pixels: &[u8]) -> impl Iterator<Item = &[u8; 4]> {
    pixels.as_chunks::<4>().0.iter()
}

/// Claim 3: the glow reaches the frame, and the clock moves it.
///
/// **Measured against the same model with the layer switched off**, which is
/// the only way to attribute a pixel to it: a circuit's `Anim Transform` nodes
/// move geometry on the same clock, and a whole-circuit camera puts every
/// individual mover at a handful of pixels. Clearing `ADD_SECOND` leaves
/// everything else - the geometry, the lighting, the node animation - exactly
/// as it was, so what changes is the glow and nothing else.
///
/// Offscreen, because this machine's windowed wgpu path screenshots black
/// while the headless one works - the same arrangement
/// `hd_scenery_animation_ground_truth.rs` uses.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_glow_reaches_the_frame_and_the_clock_moves_it() {
    let Some(image) = image() else { return };
    for (archive, path) in CIRCUITS {
        let Some(model) = build(&image, archive, path) else {
            continue;
        };
        let shot = |model: &mesh::Model, seconds: f32| {
            oag_mesh::mesh_render::capture_pixels_from(
                model,
                320,
                240,
                0.0,
                0.3,
                oag_mesh::mesh_render::Anisotropy::default(),
                seconds,
            )
            .expect("the offscreen capture runs")
        };
        let differing = |a: &[u8], b: &[u8]| {
            a.as_chunks::<4>()
                .0
                .iter()
                .zip(b.as_chunks::<4>().0)
                .filter(|(x, y)| x != y)
                .count()
        };

        // The same model with the layer switched off at the vertex, which is
        // where the shader reads it.
        let mut dark = model.clone();
        for vertex in &mut dark.vertices {
            vertex.slots &= !slots::ADD_SECOND;
        }

        // **The camera-independent number, reported beside the pixels because
        // the pixels understate it.** `capture_pixels_from` frames the whole
        // circuit by its bounding sphere, which puts a trackside sign at a
        // handful of pixels; in a race these surfaces are metres from the eye.
        // The share of geometry carrying the layer is what does not depend on
        // where the camera happens to be.
        let carrying = model
            .vertices
            .iter()
            .filter(|v| v.slots & slots::ADD_SECOND != 0)
            .count();
        // **At a non-zero clock.** At `0.0` a layer whose scroll is wrong is
        // still sampling row 0, which looks like a plausible colour; the
        // defect only opens up once the clock advances. Reported as a
        // magnitude rather than a count of differing pixels, because a count
        // cannot tell a glow that is framed small from one that computes to
        // near zero.
        let lit = shot(&model, 2.0);
        let unlit = shot(&dark, 2.0);
        let total = lit.len() / 4;
        let contributed = differing(&lit, &unlit);
        let (peak, sum) = lit.as_chunks::<4>().0.iter().zip(dark_pixels(&unlit)).fold(
            (0i32, 0i64),
            |(peak, sum), (a, b)| {
                let d = (0..3)
                    .map(|k| i32::from(a[k]).abs_diff(i32::from(b[k])) as i32)
                    .max()
                    .unwrap_or(0);
                (peak.max(d), sum + i64::from(d))
            },
        );
        println!(
            "{path}: glow peak {peak}/255, mean {:.3}/255 over the frame",
            sum as f64 / total as f64
        );
        assert!(
            peak > 1,
            "{path} draws a glow whose brightest pixel moves by {peak}/255 - \
             the table reaches the shader and computes to nothing"
        );
        println!(
            "{path}: the glow is on {carrying} of {} vertices and paints \
             {contributed} of {total} pixels at this camera",
            model.vertices.len()
        );
        assert!(carrying > 0, "{path} puts the layer on no geometry");
        assert!(
            contributed > 0,
            "{path} builds a glow table that changes no pixel, so nothing \
             reaches the shader"
        );

        // And it moves: the same model at another clock, against itself.
        let later = shot(&model, 1.5);
        let moved = differing(&lit, &later);
        println!("{path}: {moved} pixels differ between 0 s and 1.5 s");
        assert!(moved > 0, "{path} draws the same frame at two clocks");
    }
}
