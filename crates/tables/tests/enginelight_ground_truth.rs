//! What every `enginelightdata.xml` on the Wipeout HD / Fury disc actually
//! says.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run
//! with `just test-data`.
//!
//! The counts and ranges here are the disc's, not the docs': renderer.md's
//! load-site entry (2026-09-20) counted 36 files; the manifest lists **37** -
//! 13 base directories (12 teams and `zone`) across `DATA02`/`DATA03` and 24
//! `_c1`/`_n1` variants in `DATA06`. `detonator`, `test` and `zone battle`
//! ship none, so a lookup on those must come back absent rather than default.

use oag_tables::enginelight::{self, EngineLightData};

/// The three archives the files sit in, and the directories each carries -
/// every one measured off `scripts/psarc.py list`.
const ARCHIVES: &[(&str, &[&str])] = &[
    (
        "DATA02",
        &[
            "piranha",
            "qirex",
            "triakis",
            "egx",
            "ag_systems",
            "goteki",
            "assegai",
            "feisar",
            "zone",
        ],
    ),
    ("DATA03", &["auricom", "harimau", "icaras", "mirage"]),
    (
        "DATA06",
        &[
            "ag_systems_c1",
            "ag_systems_n1",
            "assegai_c1",
            "assegai_n1",
            "auricom_c1",
            "auricom_n1",
            "egx_c1",
            "egx_n1",
            "feisar_c1",
            "feisar_n1",
            "goteki_c1",
            "goteki_n1",
            "harimau_c1",
            "harimau_n1",
            "icaras_c1",
            "icaras_n1",
            "mirage_c1",
            "mirage_n1",
            "piranha_c1",
            "piranha_n1",
            "qirex_c1",
            "qirex_n1",
            "triakis_c1",
            "triakis_n1",
        ],
    ),
];

fn read_all() -> Option<Vec<(String, EngineLightData)>> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let mut out = Vec::new();
    for (archive, dirs) in ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut archive = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        for dir in *dirs {
            let path = format!("/data/ships/{dir}/enginelightdata.xml");
            let bytes = archive
                .read_path(&path)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            let text = String::from_utf8(bytes).unwrap_or_else(|_| panic!("{path} is not UTF-8"));
            let data = enginelight::parse(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
            out.push((dir.to_string(), data));
        }
    }
    Some(out)
}

/// **All 37 files parse, and every number is inside the span the live
/// capture pinned**: `Radius` 0.7-2.0, which is `D` 0.62-2.05 in
/// `data/traces/hd-spu-light-companion/s*_slot.bin` less the `+-0.1`
/// jitter, and `Distance` -0.4 to 1.5.
#[test]
#[ignore]
fn every_ship_directory_authors_two_numbers_in_the_captured_span() {
    let Some(all) = read_all() else { return };
    assert_eq!(all.len(), 37, "the manifest lists 37 enginelightdata.xml");
    let radius_min = all.iter().map(|(_, d)| d.radius).fold(f32::MAX, f32::min);
    let radius_max = all.iter().map(|(_, d)| d.radius).fold(f32::MIN, f32::max);
    let distance_min = all.iter().map(|(_, d)| d.distance).fold(f32::MAX, f32::min);
    let distance_max = all.iter().map(|(_, d)| d.distance).fold(f32::MIN, f32::max);
    assert_eq!((radius_min, radius_max), (0.7, 2.0));
    assert_eq!((distance_min, distance_max), (-0.4, 1.5));
}

/// A handful of exact rows, so a parser that read the wrong element or the
/// wrong file would show as a wrong number rather than a wrong range: the
/// player's default Fury hull, its base team, the Zone ship, and the one
/// negative `Distance`.
#[test]
#[ignore]
fn named_rows_read_exactly() {
    let Some(all) = read_all() else { return };
    let of = |dir: &str| all.iter().find(|(d, _)| d == dir).map(|(_, v)| *v);
    assert_eq!(
        of("feisar"),
        Some(EngineLightData {
            distance: 0.8,
            radius: 1.0
        })
    );
    assert_eq!(
        of("feisar_c1"),
        Some(EngineLightData {
            distance: -0.4,
            radius: 1.0
        })
    );
    assert_eq!(
        of("zone"),
        Some(EngineLightData {
            distance: 1.3,
            radius: 1.8
        })
    );
    assert_eq!(
        of("qirex"),
        Some(EngineLightData {
            distance: 0.1,
            radius: 0.7
        })
    );
    assert_eq!(
        of("piranha"),
        Some(EngineLightData {
            distance: 0.4,
            radius: 2.0
        })
    );
}

/// The three ship directories with no file: the mode ships and the
/// development leftover. Read through the same archive so the negative is a
/// measured absence, not an unmounted archive.
#[test]
#[ignore]
fn the_mode_ships_author_none() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    for (archive, dir) in [
        ("DATA00", "detonator"),
        ("DATA02", "test"),
        ("DATA00", "zone battle"),
    ] {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut archive = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let path = format!("/data/ships/{dir}/enginelightdata.xml");
        assert!(
            archive.read_path(&path).is_err(),
            "{path} should not exist - the mode ships hang no engine light"
        );
    }
}
