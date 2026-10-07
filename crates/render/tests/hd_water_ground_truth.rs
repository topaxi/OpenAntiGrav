//! HD's water family, over every circuit that ships: which materials the
//! classifiers claim, what they bind, and what the programs read.
//!
//! **`#[ignore]`d and never run in CI**; needs the decrypted PS3 image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_water_ground_truth)'
//! ```
//!
//! The census this pins (`hd_water_census.rs`, `hd_water_reach.rs`, 28 circuit
//! models): `water_test_2` and `water_noref` (Vineta K, both directions) are
//! the only materials whose program declares the paraboloid probe, reads its
//! picture as a normal map and is not lit by the baked lightmap formula; and
//! `sebenco_ice` (Sebenco Climb, both directions) is the only one that patches
//! the three authored water colours.

use oag_assets::Container;
use oag_mesh::mesh::{self, slots};
use oag_rcs::rcsmaterial::{self, fragment::Program};

const IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn archive(image: &std::path::Path, n: u32) -> String {
    format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", image.display())
}

fn build(image: &std::path::Path, path: &str) -> Option<(mesh::Model, mesh::rcs::Report)> {
    let (spec, data) = (0..4).find_map(|n| {
        let spec = archive(image, n);
        mesh::read_blob(&spec, path).ok().map(|d| (spec, d))
    })?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        (0..4).find_map(|n| mesh::read_blob(&archive(image, n), name).ok())
    })
    .ok()
}

/// `(water_lit_colour, water_glint_only, ice_bound)` for one circuit's two
/// directions, with every one of the models building and `ice_unread` zero.
fn reach(circuit: &str) -> (usize, usize, usize) {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return (usize::MAX, 0, 0);
    };
    let (mut lit, mut glint, mut ice) = (0, 0, 0);
    for track in ["track.vex", "track_reversed.vex"] {
        let path = format!("/data/environments/{circuit}/{track}");
        let Some((model, report)) = build(&image, &path) else {
            continue;
        };
        assert_eq!(report.ice_unread, 0, "{path}");
        lit += report.water_lit_colour;
        glint += report.water_glint_only;
        ice += report.ice_bound;
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
    (lit, glint, ice)
}

macro_rules! reach_test {
    ($name:ident, $circuit:literal, $expected:expr, $why:literal) => {
        #[test]
        #[ignore = "needs a decrypted PS3 disc image in data/images"]
        fn $name() {
            let got = reach($circuit);
            if got.0 == usize::MAX {
                return;
            }
            assert_eq!(got, $expected, $why);
        }
    };
}

reach_test!(
    vineta_k_has_one_lit_colour_sea_and_three_glint_seas_per_direction,
    "01_vineta_k",
    (2, 6, 0),
    "water_test_2 x1 and water_noref x3, both directions"
);
reach_test!(
    sebenco_climb_has_three_ice_materials_per_direction,
    "10_sebenco_climb",
    (0, 0, 6),
    "sebenco_ice x3, both directions"
);
reach_test!(
    no_other_circuit_is_reached_02,
    "02_track",
    (0, 0, 0),
    "none"
);
reach_test!(
    no_other_circuit_is_reached_03,
    "03_track",
    (0, 0, 0),
    "none"
);
reach_test!(
    no_other_circuit_is_reached_04,
    "04_chenghou_project",
    (0, 0, 0),
    "none"
);
reach_test!(
    no_other_circuit_is_reached_05,
    "05_ubermall",
    (0, 0, 0),
    "none"
);
reach_test!(
    no_other_circuit_is_reached_12,
    "12_sol_2",
    (0, 0, 0),
    "none"
);
reach_test!(
    no_other_circuit_is_reached_15,
    "15_anulpha_pass",
    (0, 0, 0),
    "none"
);
reach_test!(
    no_other_circuit_is_reached_amphiseum,
    "amphiseum",
    (0, 0, 0),
    "none"
);
reach_test!(
    no_other_circuit_is_reached_modesto,
    "modesto_heights",
    (0, 0, 0),
    "none"
);
reach_test!(
    no_other_circuit_is_reached_talons,
    "talons_junction",
    (0, 0, 0),
    "none"
);
reach_test!(
    no_other_circuit_is_reached_tech,
    "tech_de_ra",
    (0, 0, 0),
    "none"
);
reach_test!(
    no_other_circuit_is_reached_zone1,
    "zone_1",
    (0, 0, 0),
    "none"
);
reach_test!(
    no_other_circuit_is_reached_zone2,
    "zone_2",
    (0, 0, 0),
    "none"
);
reach_test!(
    no_other_circuit_is_reached_zone3,
    "zone_3",
    (0, 0, 0),
    "none"
);
reach_test!(
    no_other_circuit_is_reached_zone4,
    "zone_4",
    (0, 0, 0),
    "none"
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
