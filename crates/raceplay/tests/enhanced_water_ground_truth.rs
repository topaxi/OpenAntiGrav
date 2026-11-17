//! `enhanced` water on Pulse's and Pure's circuits: the batches that bind each circuit's listed
//! sea texture are drawn with the generated wave normal map, and nothing else is touched.
//!
//! The lists are `oag_pulse::race::WATER_CIRCUITS` and `oag_pure::race::WATER_CIRCUITS`; each name
//! was confirmed on a frame (`docs/rendering/water.md`, "Enhanced"). The counts are the number
//! of opaque batches and their vertices that bind the texture, read off the disc.
//!
//! **`#[ignore]`d and never run in CI**: it needs `pulse-psp-eu.chd` and `pure-psp-eu.chd`.

use oag_display::display::Water;
use oag_mesh::mesh::vex_water;
use oag_mesh::mesh::{Model, PROGRAM_WATER, slots};

fn load(image: &str, class: &str, track: &str, water: Water) -> Option<oag_raceplay::Loaded> {
    let image = oag_testdata::image(image)?;
    Some(
        oag_raceplay::load(&oag_raceplay::Options {
            source: image.display().to_string(),
            class: class.to_string(),
            mode: oag_race::Mode::TimeTrial,
            track: Some(track.to_string()),
            seed: Some(1),
            water,
            ..oag_raceplay::Options::default()
        })
        .expect("loading a circuit"),
    )
}

/// The vertices whose glow entry is an enhanced water entry, and how many carry a tangent.
fn tagged(model: &Model) -> (usize, usize) {
    let water = |entry: u32| {
        entry != 0
            && model
                .emissive
                .get(entry as usize - 1)
                .is_some_and(|e| e.program == [PROGRAM_WATER, vex_water::ENHANCED_KIND])
    };
    let tagged: Vec<_> = model
        .vertices
        .iter()
        .filter(|v| water(v.slots >> slots::MATERIAL_SHIFT))
        .collect();
    (
        tagged.len(),
        tagged.iter().filter(|v| v.tangent != 0).count(),
    )
}

fn check(image: &str, class: &str, circuit: &str, track: &str, vertices: usize) {
    let (Some(enhanced), Some(original)) = (
        load(image, class, track, Water::Enhanced),
        load(image, class, track, Water::Original),
    ) else {
        return;
    };
    assert_eq!(
        tagged(&enhanced.track_model),
        (vertices, vertices),
        "{circuit}: every vertex of a water batch names the entry and carries a tangent"
    );
    assert!(vex_water::is_tagged(&enhanced.track_model), "{circuit}");
    assert!(
        enhanced
            .track_model
            .pad_masks
            .iter()
            .flatten()
            .any(|map| map.label.contains("generated normal map")),
        "{circuit}: the generated map is bound third"
    );
    assert!(
        enhanced
            .report
            .iter()
            .any(|line| line.contains("enhanced water on")),
        "{circuit}: the loader says so"
    );
    // The sky the sea reflects: a mesh sky has no map of its own, so one is cast out of the dome,
    // full size, opaque, and more than one colour (a flat black map would reflect nothing).
    let sky = enhanced.sky_model.as_ref().expect("the circuit has a sky");
    assert!(oag_mesh::mesh::sky_paraboloid::from_sky(sky).is_none());
    let map = oag_mesh::mesh::sky_paraboloid::from_mesh(sky).expect("a sky mesh casts a map");
    let (w, h) = oag_mesh::mesh::sky_paraboloid::SIZE;
    assert_eq!(map.len(), (w * h * 4) as usize, "{circuit}");
    assert!(map.chunks_exact(4).all(|t| t[3] == 255), "{circuit}");
    let first = &map[..3];
    assert!(
        map.chunks_exact(4)
            .any(|t| t[..3].iter().zip(first).any(|(a, b)| a.abs_diff(*b) > 24)),
        "{circuit}: the sky map has more than one colour"
    );
    // Under the circuit's own law nothing is laid: the entries, the map and every vertex stay as
    // the plain mesh has them.
    assert_eq!(tagged(&original.track_model), (0, 0), "{circuit}");
    assert!(!vex_water::is_tagged(&original.track_model), "{circuit}");
    assert!(
        original
            .track_model
            .pad_masks
            .iter()
            .flatten()
            .next()
            .is_none()
    );
    assert_eq!(
        enhanced.track_model.vertices.len(),
        original.track_model.vertices.len(),
        "{circuit}: the geometry is the same"
    );
}

const PULSE: &str = "pulse-psp-eu.chd";
const PURE: &str = "pure-psp-eu.chd";

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn moa_therma_s_sea_is_drawn_enhanced() {
    check(
        PULSE,
        "VENOM",
        "03_Track",
        r"Data\Environments\03_Track\track.vex",
        7210,
    );
}

/// The PS2 pressing embeds no texture names: the names resolve through its texture set, and the
/// sea is found by them there too (fewer batches, the PS2 build splits the mesh differently).
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn the_ps2_pressing_draws_moa_therma_s_sea_enhanced() {
    check(
        "pulse-ps2-eu.chd",
        "VENOM",
        "03_Track",
        r"Data\Environments\03_Track\track.vex",
        4775,
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn moa_therma_reversed_draws_the_same_sea() {
    check(
        PULSE,
        "VENOM",
        "03_Track",
        r"Data\Environments\03_Track\track_reversed.vex",
        7210,
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn the_06_track_sea_is_drawn_enhanced() {
    check(
        PULSE,
        "VENOM",
        "06_Track",
        r"Data\Environments\06_Track\track.vex",
        1788,
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn fort_gale_s_sea_is_drawn_enhanced() {
    check(
        PULSE,
        "VENOM",
        "14_Track",
        r"Data\Environments\14_Track\track.vex",
        468,
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn the_16_track_river_is_drawn_enhanced() {
    check(
        PULSE,
        "VENOM",
        "16_Track",
        r"Data\Environments\16_Track\track.vex",
        75,
    );
}

#[test]
#[ignore = "needs data/images/pure-psp-eu.chd"]
fn vineta_k_s_two_seas_are_drawn_enhanced() {
    check(
        PURE,
        "VECTOR",
        "01_Vineta_K",
        r"Data\Environments\01_Vineta_K\track.vex",
        5795,
    );
}

/// A circuit the title lists no sea for draws as it did, whatever the law.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn a_circuit_with_no_listed_sea_is_untouched() {
    let track = r"Data\Environments\02_Track\track.vex";
    assert!(oag_pulse::TITLE.water.textures_of(track).is_none());
    let Some(loaded) = load(PULSE, "VENOM", track, Water::Enhanced) else {
        return;
    };
    assert_eq!(tagged(&loaded.track_model), (0, 0));
    assert!(!vex_water::is_tagged(&loaded.track_model));
}
