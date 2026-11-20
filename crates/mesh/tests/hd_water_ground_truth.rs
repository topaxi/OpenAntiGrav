//! HD's water family, over every circuit that ships: which materials the
//! classifiers claim, what they bind, and what the programs read.
//!
//! **`#[ignore]`d and never run in CI**; needs the decrypted PS3 image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-mesh --run-ignored all \
//!     -E 'test(/^hd_water_ground_truth::/)'
//! ```
//!
//! The census this pins (`hd_water_census.rs`, `hd_water_reach.rs`, 28 circuit
//! models): `water_test_2` and `water_noref` (Vineta K, both directions) are
//! the only materials whose program declares the paraboloid probe, reads its
//! picture as a normal map and is not lit by the baked lightmap formula; and
//! `sebenco_ice` (Sebenco Climb, both directions) is the only one that patches
//! the three authored water colours.

use crate::archive_cache;

use archive_cache::Reads;
use oag_assets::Container;
use oag_mesh::mesh::{self, slots};
use oag_rcs::rcsmaterial::{self, fragment::Program};

const IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn archive(image: &std::path::Path, n: u32) -> String {
    format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", image.display())
}

fn build(
    reads: &Reads,
    image: &std::path::Path,
    path: &str,
) -> Option<(mesh::Model, mesh::rcs::Report)> {
    build_under(reads, image, path, oag_display::display::Water::default())
}

fn build_under(
    reads: &Reads,
    image: &std::path::Path,
    path: &str,
    water: oag_display::display::Water,
) -> Option<(mesh::Model, mesh::rcs::Report)> {
    let specs: Vec<String> = (0..4).map(|n| archive(image, n)).collect();
    let (spec, data) = reads.read_any(&specs, path)?;
    let geometry = mesh::rcs::sibling_geometry(spec, path, &data)?;
    mesh::rcs::build_scene_with(
        path,
        &data,
        &geometry,
        &mut |name| reads.read_any(&specs, name).map(|(_, bytes)| bytes),
        oag_gpu::formats::SceneStore::default(),
        water,
    )
    .ok()
}

/// `graphics.water` reaches the build: `off` (and the laws not drawn here, `2048` and
/// `omega`) keep the constant reading, one lit-colour and three glint-only slots with no
/// taps; `original` and `hd` draw the taps on all four and change the vertices' slots.
/// Dropping the setting's wiring leaves the report naming `original` under every value and
/// fails here; dropping the taps leaves `original` equal to `off`.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_water_setting_reaches_the_build_and_only_hd_s_laws_draw_the_taps() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let reads = Reads::new();
    let path = "/data/environments/01_vineta_k/track.vex";
    let mut seen: Vec<(oag_display::display::Water, Vec<u32>)> = Vec::new();
    for law in oag_display::display::Water::ALL {
        let (model, report) = build_under(&reads, &image, path, law).expect("Vineta K builds");
        assert_eq!(report.water_law, law, "the build was asked for {law}");
        assert!(
            report.describe().contains(&format!("water setting: {law}")),
            "the loader report names the law"
        );
        let taps = matches!(
            law,
            oag_display::display::Water::Original | oag_display::display::Water::Hd
        );
        let counts = (
            report.water_lit_colour,
            report.water_glint_only,
            report.water_taps_drawn,
        );
        assert_eq!(counts, if taps { (0, 0, 4) } else { (1, 3, 0) }, "{law}");
        seen.push((law, model.vertices.iter().map(|v| v.slots).collect()));
    }
    let slots_of = |law| &seen.iter().find(|(l, _)| *l == law).expect("built").1;
    use oag_display::display::Water;
    assert!(slots_of(Water::Off) == slots_of(Water::Vita2048));
    assert!(slots_of(Water::Off) == slots_of(Water::Omega));
    assert!(slots_of(Water::Original) == slots_of(Water::Hd));
    assert!(
        slots_of(Water::Off) != slots_of(Water::Original),
        "the taps move the slots"
    );
}

/// The sea's table entry carries the material's own `Reflectivity` (Moa Therma 0.3, Ubermall
/// 0.36; `docs/rendering/water.md`, "HD/Fury: `reflectplane_dc_seawater`"), which the shader
/// weights the sea's stand-in sky reflection by. Dropping the lane leaves a sea with no
/// reflection term and fails here.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_sea_entry_carries_its_materials_reflectivity() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let reads = Reads::new();
    for (circuit, authored) in [("03_track", 0.3_f32), ("05_ubermall", 0.36)] {
        let path = format!("/data/environments/{circuit}/track.vex");
        let (model, _) = build(&reads, &image, &path).expect("the circuit builds");
        let seas: Vec<f32> = model
            .emissive
            .iter()
            .filter(|entry| entry.program == [mesh::PROGRAM_WATER, 1.0])
            .map(|entry| entry.offset)
            .collect();
        assert!(!seas.is_empty(), "{circuit}: no sea entry");
        assert!(
            seas.iter().all(|&weight| (weight - authored).abs() < 1e-6),
            "{circuit}: the sea entries weight the reflection {seas:?}, authored {authored}"
        );
    }
}

/// The planar reflection target's chunks are exactly the chunks with render flag bit 3
/// (`rcsmodel::RENDER_REFLECTED`), split by bit 5 into its two fog groups
/// (`docs/ghidra/functions/ps3-hdfury-eu/track-reflect.md`): Moa Therma's and Ubermall's
/// seas have them, Chenghou's has none (its target holds the sky alone, as read live).
/// Dropping the view's filter, or drawing the behind-the-glass chunks instead, fails here.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_reflection_target_draws_the_bit_3_chunks() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let reads = Reads::new();
    let specs: Vec<String> = (0..4).map(|n| archive(&image, n)).collect();
    for circuit in ["03_track", "05_ubermall", "04_chenghou_project"] {
        let path = format!("/data/environments/{circuit}/track.vex");
        let (spec, data) = reads.read_any(&specs, &path).expect("the circuit ships");
        let geometry = mesh::rcs::sibling_geometry(spec, &path, &data).expect("its geometry");
        let (main, _) = build(&reads, &image, &path).expect("the circuit builds");
        let groups =
            mesh::rcs::build_reflection(&path, &data, &geometry, &main).expect("the groups build");
        let drawn = |group: &Option<mesh::Model>| group.as_ref().map_or(0, |m| m.mesh_count);
        let flagged = oag_rcs::rcsmodel::Model::parse(&geometry)
            .expect("the geometry parses")
            .meshes
            .iter()
            .filter(|chunk| chunk.is_reflected() && !chunk.is_behind_glass())
            .map(|chunk| chunk.uses_alternate_fog())
            .fold((0, 0), |(alt, pri), alternate| {
                if alternate {
                    (alt + 1, pri)
                } else {
                    (alt, pri + 1)
                }
            });
        let got = (drawn(&groups.alternate_fog), drawn(&groups.primary_fog));
        println!("{circuit}: reflected chunks (alternate, primary) {got:?}, flagged {flagged:?}");
        assert_eq!(got, flagged, "{circuit}");
        assert_eq!(
            got.0 + got.1 > 0,
            circuit != "04_chenghou_project",
            "{circuit}"
        );
    }
}

/// How many drawn materials and chunks resolve no shader variant, per circuit
/// and direction, measured 2026-10-19: **a ratchet over a known gap, not zero**.
/// Only Modesto Heights, Talon's Junction and Tech De Ra have one (the others
/// resolve every drawn material); those materials draw through `mesh.wgsl`'s
/// unresolved fallback. May shrink, never grow.
fn variant_gap(circuit: &str, track: &str) -> (usize, usize) {
    match (circuit, track) {
        ("modesto_heights", _) => (21, 176),
        ("talons_junction", _) => (16, 54),
        ("tech_de_ra", "track.vex") => (18, 103),
        ("tech_de_ra", _) => (17, 103),
        _ => (0, 0),
    }
}

/// What one circuit's two directions classify, under `original`: the water
/// families' chunks per direction (forward, reverse; `(sea, sky, lit colour, glint)`),
/// the water materials drawn with their taps and the ice materials bound, with every one
/// of the models building and `ice_unread` zero.
#[derive(Debug, PartialEq, Eq)]
struct Reach {
    chunks: [(usize, usize, usize, usize); 2],
    taps_drawn: usize,
    ice: usize,
}

fn reach(circuit: &str) -> Option<Reach> {
    let image = oag_testdata::image(IMAGE)?;
    let reads = Reads::new();
    let mut got = Reach {
        chunks: [(0, 0, 0, 0); 2],
        taps_drawn: 0,
        ice: 0,
    };
    for (direction, track) in ["track.vex", "track_reversed.vex"].into_iter().enumerate() {
        let path = format!("/data/environments/{circuit}/{track}");
        let Some((model, report)) = build(&reads, &image, &path) else {
            continue;
        };
        assert_eq!(report.ice_unread, 0, "{path}");
        let c = report.water_chunks;
        got.chunks[direction] = (c.sea, c.sky, c.lit_colour, c.glint);
        got.taps_drawn += report.water_taps_drawn;
        println!(
            "{path}: taps drawn {}, unread {}, blocked {}",
            report.water_taps_drawn, report.water_taps_unread, report.water_taps_blocked
        );
        let (unshipped, missed) = variant_gap(circuit, track);
        assert!(
            report.variants_unshipped <= unshipped && report.variant_chunks_missed <= missed,
            "{path}: {} material(s) and {} chunk(s) without a shader variant, known {unshipped} and {missed}",
            report.variants_unshipped,
            report.variant_chunks_missed
        );
        let faults = crate::track_albedo::faults(&model, true);
        assert!(
            faults.is_empty(),
            "{path}: a drawn colour texture is not a picture: {}",
            crate::track_albedo::describe(&faults)
        );
        assert_eq!(
            (report.water_lit_colour, report.water_glint_only),
            (0, 0),
            "{path}: under `original` no water keeps the constant reading"
        );
        got.ice += report.ice_bound;
        if report.ice_bound > 0 {
            assert!(
                model.vertices.iter().any(|v| v.slots & slots::ICE != 0),
                "{path}: an ice material's vertices carry the ICE bit"
            );
            let ice_vertices: Vec<_> = model
                .vertices
                .iter()
                .filter(|v| v.slots & slots::ICE != 0)
                .collect();
            println!(
                "{path}: {} ice vertices, {} with a second coordinate",
                ice_vertices.len(),
                ice_vertices
                    .iter()
                    .filter(|v| v.texcoord2 != [0.0, 0.0])
                    .count()
            );
            assert!(
                ice_vertices.iter().any(|v| v.texcoord2 != [0.0, 0.0]),
                "{path}: and its second coordinate set is read"
            );
        }
    }
    Some(got)
}

macro_rules! reach_test {
    ($name:ident, $circuit:literal, $chunks:expr, $taps:expr, $ice:expr, $why:literal) => {
        #[test]
        #[ignore = "needs a decrypted PS3 disc image in data/images"]
        fn $name() {
            let Some(got) = reach($circuit) else {
                return;
            };
            let want = Reach {
                chunks: $chunks,
                taps_drawn: $taps,
                ice: $ice,
            };
            assert_eq!(got, want, $why);
        }
    };
}

reach_test!(
    vineta_k_has_two_lit_colour_chunks_and_three_glint_chunks_per_direction,
    "01_vineta_k",
    [(0, 0, 2, 3); 2],
    8,
    0,
    "water_test_2 x1 (2 chunks) and water_noref x3 (3 chunks), both directions"
);
reach_test!(
    sebenco_climb_has_three_ice_materials_per_direction,
    "10_sebenco_climb",
    [(0, 0, 0, 0); 2],
    0,
    6,
    "sebenco_ice x3, both directions"
);
reach_test!(
    moa_therma_has_thirty_one_sea_chunks_per_direction,
    "03_track",
    [(31, 0, 0, 0); 2],
    8,
    0,
    "reflectplane_dc_seawater: 12 + 1 + 14 + 4 chunks over four slots, both directions"
);
reach_test!(
    ubermall_has_thirteen_sea_chunks_forward_and_seven_reversed,
    "05_ubermall",
    [(13, 0, 0, 0), (7, 0, 0, 0)],
    6,
    0,
    "reflectplane_dc_seawater: 11 + 1 + 1 forward, 5 + 1 + 1 reversed"
);
reach_test!(
    chenghou_project_has_three_sea_chunks_per_direction,
    "04_chenghou_project",
    [(3, 0, 0, 0); 2],
    2,
    0,
    "reflectplane_dc_seawater x1, both directions"
);
reach_test!(
    modesto_heights_has_five_sea_chunks_per_direction,
    "modesto_heights",
    [(5, 0, 0, 0); 2],
    4,
    0,
    "reflectplane_dc_seawater: 5 chunks, and a lightmapped slot drawn only as a chunk's second surface"
);
reach_test!(
    amphiseum_has_two_sky_lit_pool_chunks_per_direction,
    "amphiseum",
    [(0, 2, 0, 0); 2],
    4,
    0,
    "water x2 slots, one chunk each, both directions"
);
reach_test!(
    no_other_circuit_is_reached_02,
    "02_track",
    [(0, 0, 0, 0); 2],
    0,
    0,
    "no water and no ice"
);
reach_test!(
    no_other_circuit_is_reached_12,
    "12_sol_2",
    [(0, 0, 0, 0); 2],
    0,
    0,
    "no water and no ice"
);
reach_test!(
    no_other_circuit_is_reached_15,
    "15_anulpha_pass",
    [(0, 0, 0, 0); 2],
    0,
    0,
    "no water and no ice"
);
reach_test!(
    no_other_circuit_is_reached_talons,
    "talons_junction",
    [(0, 0, 0, 0); 2],
    0,
    0,
    "no water and no ice"
);
reach_test!(
    no_other_circuit_is_reached_tech,
    "tech_de_ra",
    [(0, 0, 0, 0); 2],
    0,
    0,
    "no water and no ice"
);
reach_test!(
    no_other_circuit_is_reached_zone1,
    "zone_1",
    [(0, 0, 0, 0); 2],
    0,
    0,
    "no water and no ice"
);
reach_test!(
    no_other_circuit_is_reached_zone2,
    "zone_2",
    [(0, 0, 0, 0); 2],
    0,
    0,
    "no water and no ice"
);
reach_test!(
    no_other_circuit_is_reached_zone3,
    "zone_3",
    [(0, 0, 0, 0); 2],
    0,
    0,
    "no water and no ice"
);
reach_test!(
    no_other_circuit_is_reached_zone4,
    "zone_4",
    [(0, 0, 0, 0); 2],
    0,
    0,
    "no water and no ice"
);

/// What the resolved lit-race program of one material reads, by unit.
fn program_of(
    image: &std::path::Path,
    circuit: &str,
    material: &str,
    slot_name: &str,
) -> (rcsmaterial::Declared, Program) {
    let path = format!("/data/environments/{circuit}/track.vex");
    let (spec, data) = (0..4)
        .find_map(|n| {
            let spec = archive(image, n);
            mesh::read_blob(&spec, &path).ok().map(|d| (spec, d))
        })
        .expect("the track reads");
    let geometry = mesh::rcs::sibling_geometry(&spec, &path, &data).expect("a sibling model");
    let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the model parses");
    let mut decl_of = std::collections::HashMap::new();
    for chunk in rcs
        .meshes
        .iter()
        .flat_map(oag_rcs::rcsmodel::Mesh::surfaces)
    {
        decl_of.entry(chunk.material).or_insert(chunk.decl.as_ref());
    }
    let (slot, _) = rcs
        .materials
        .iter()
        .enumerate()
        .find(|(_, m)| m.name.ends_with(material))
        .unwrap_or_else(|| panic!("{slot_name}: no {material}"));
    let decl = decl_of[&u32::try_from(slot).unwrap()].expect("a declaration");
    let mut container = Container::open(&archive(image, 2)).expect("DATA02 opens");
    let blob = container
        .read_entry(&format!("/{}", rcs.materials[slot].name))
        .expect("the material reads");
    let parsed = rcsmaterial::RcsMaterial::parse(&blob).expect("parses");
    let word = rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, Some(decl));
    let key = rcsmaterial::Features::from_pass_word(word);
    let variant = parsed
        .variant(rcsmaterial::Class::Static, key)
        .expect("a variant");
    (
        rcsmaterial::Declared::parse(&blob, variant.fragment.offset).expect("declared"),
        Program::parse(&blob, variant.fragment.offset).expect("program"),
    )
}

/// `water_test_2` decodes `waves2` at unit 0 as `2 * tap - 1`; a picture read
/// as a colour is not a normal; `sebenco_ice` reads its mask one lane wide and
/// its snow as a colour.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_programs_read_their_units_as_the_census_says() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let (declared, program) = program_of(&image, "01_vineta_k", "/water_test_2.rcsmaterial", "sea");
    let waves = declared
        .samplers
        .iter()
        .find(|&&(h, _)| h != 0x9edd_3243)
        .expect("the waves sampler");
    assert!(
        program.samples_as_normal(waves.1 as u8),
        "waves2 is a normal map"
    );
    assert!(program.samples_colour(waves.1 as u8));

    let (declared, program) = program_of(
        &image,
        "10_sebenco_climb",
        "/sebenco_ice.rcsmaterial",
        "pool",
    );
    let unit = |hash: u32| {
        declared
            .samplers
            .iter()
            .find(|&&(h, _)| h == hash)
            .map(|&(_, u)| u as u8)
            .expect("declared")
    };
    assert!(
        !program.samples_colour(unit(0xf1d8_75a1)),
        "the pond mask is one lane"
    );
    assert!(
        program.samples_colour(unit(0x2e7d_71db)),
        "the snow is a colour"
    );
    assert!(
        !program.samples_as_normal(unit(0x2e7d_71db)),
        "the snow is not a normal"
    );
    assert!(
        program.samples_as_normal(unit(0xfe9b_d1f3)),
        "256norm4 is decoded 2x - 1 at its first tap"
    );
}
