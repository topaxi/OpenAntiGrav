//! Omega's panel-map road (`road_alphaspec_normal_emissive_panelmap1`) on Sol draws through
//! `mesh::rcs::psp2::panel_map`, and Sol's sky carries `skyCube.gnf` as its reflection map
//! (`mesh::rcs::psp2::sky_reflection`).
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! Builds Sol's `track.final.rcsmodel` and holds that every vertex of a panel-map material names
//! a glow-table entry of [`PROGRAM_PANEL_MAP`] whose next two entries carry the material's
//! authored `SpecStrength`/`SpecPower` and `Depth`/`SlopeWidth`/`Aspect`, and that its draws bind
//! the panel map fifth as one half-float channel; then builds `skycube.rcsmodel` and holds that
//! it carries a four-level sky map. Remove the builder's `panel_map::plan` or
//! `sky_reflection::load` call and it fails.

use std::path::Path;

use oag_mesh::mesh::rcs::psp2;
use oag_mesh::mesh::{BlockFormat, PROGRAM_PANEL_MAP, Texels, sky_paraboloid, slots};

const ARCHIVES: [&str; 9] = [
    "ps4/omega-eu/uroot/data00.psarc",
    "ps4/omega-eu/uroot/data01.psarc",
    "ps4/omega-eu/uroot/data02.psarc",
    "ps4/omega-eu/uroot/data03.psarc",
    "ps4/omega-eu/uroot/data04.psarc",
    "ps4/omega-eu-patch/uroot/data05.psarc",
    "ps4/omega-eu-patch/uroot/data07.psarc",
    "ps4/omega-eu-patch/uroot/data08.psarc",
    "ps4/omega-eu-patch/uroot/data09.psarc",
];

fn open() -> Option<Vec<oag_assets::psarc::Archive>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
    if !root.join("ps4").exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} has no ps4/",
            root.display()
        );
        println!("skipping: Omega not extracted");
        return None;
    }
    Some(
        ARCHIVES
            .iter()
            .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
            .collect(),
    )
}

fn build(
    archives: &mut [oag_assets::psarc::Archive],
    path: &str,
) -> (oag_mesh::mesh::Model, psp2::Report) {
    let index = (0..archives.len())
        .find(|&i| archives[i].contains(path))
        .unwrap_or_else(|| panic!("{path} is in no archive"));
    let blob = archives[index].read_path(path).expect("the model");
    psp2::build(path, &blob, None, &mut |p| {
        archives.iter_mut().find_map(|a| a.read_path(p).ok())
    })
    .expect("builds")
}

#[test]
#[ignore]
fn sols_panel_map_road_marches_and_its_sky_is_the_reflection_map() {
    let Some(mut archives) = open() else { return };
    let (model, report) = build(
        &mut archives,
        "Data/environments2048/sol/track.final.rcsmodel",
    );
    let decoded = oag_rcs::rcsmodel::psp2::parse(
        &archives
            .iter_mut()
            .find_map(|a| {
                a.read_path("Data/environments2048/sol/track.final.rcsmodel")
                    .ok()
            })
            .expect("the model"),
    )
    .expect("parses");
    super::omega_crowd_shadowed_check::check("sol", &decoded, &model, &report);
    assert!(report.panel.materials > 0, "{}", report.describe());
    assert_eq!(report.panel.unread, 0, "{}", report.describe());
    let mut carried = 0usize;
    for vertex in &model.vertices {
        let entry = ((vertex.slots >> slots::MATERIAL_SHIFT) & slots::MATERIAL_MASK) as usize;
        if entry == 0 || model.emissive[entry - 1].program[0] != PROGRAM_PANEL_MAP {
            continue;
        }
        let rows = &model.emissive[entry - 1..entry + 2];
        assert_eq!(
            rows[0].tint,
            [0.5, 0.6, 10.0],
            "Sol's Reflectivity Amount, Min, Scale"
        );
        assert_eq!(rows[0].offset, 4.0, "Sol's ReflectivityPower");
        assert!((rows[0].scale - 0.6).abs() < 1e-6, "2 * Blurrines");
        assert_eq!(
            [rows[1].tint[0], rows[1].tint[1]],
            [5.0, 100.0],
            "SpecStrength, SpecPower"
        );
        assert_eq!(rows[2].tint, [3.5, 1.8, 4.0], "Depth, SlopeWidth, Aspect");
        assert_ne!(vertex.tangent, 0, "the march needs the tangent frame");
        carried += 1;
    }
    assert!(carried > 6_749, "{carried} panel-map road vertices");
    let panel_maps = model
        .fifth_maps
        .iter()
        .flatten()
        .filter(|t| {
            t.label
                .to_ascii_lowercase()
                .contains("track_a_floor_panel_df")
                && matches!(
                    t.texels,
                    Texels::Blocks {
                        format: BlockFormat::R16Float,
                        ..
                    }
                )
        })
        .count();
    assert!(
        panel_maps > 0,
        "no draw binds the panel map as a half float"
    );

    let (sky, sky_report) = build(&mut archives, "Data/environments2048/sol/skycube.rcsmodel");
    let map = sky
        .sky_reflection
        .as_ref()
        .expect("skyCube.gnf beside the skycube");
    assert_eq!(
        map.len(),
        sky_paraboloid::chain_bytes(4),
        "four cubes, four levels"
    );
    assert!(sky_report.describe().contains("skyCube.gnf"));
    assert!(
        model.sky_reflection.is_none(),
        "only the sky carries the map"
    );
}
