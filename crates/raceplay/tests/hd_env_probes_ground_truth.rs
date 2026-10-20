//! A Wipeout HD race load carries the circuit's baked environment probes, and a title
//! that ships none carries none.
//!
//! **`#[ignore]`d, needs the decrypted HD image** (and Pulse's for the negative case).
//! Dropping the `.probes` read from the load turns the first test red: the hull would
//! silently keep its constant ambient.

use oag_raceplay as race;

const HD: &str = "data/images/hdfury-ps3-eu-dec.iso";
const PULSE: &str = "data/images/pulse-psp-usa.chd";

/// Every HD circuit on the disc, forward. Each loads its own `.probes`.
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

fn load(image: &str, track: Option<&str>) -> Option<race::Loaded> {
    let path = oag_testdata::image(image)?;
    Some(
        race::load(&race::Options {
            source: path.display().to_string(),
            class: "VENOM".to_string(),
            track: track.map(str::to_string),
            ..race::Options::default()
        })
        .expect("loading the race"),
    )
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_circuit_loads_its_own_probes() {
    for (track, probes) in [
        ("/data/environments/talons_junction/track.vex", 862),
        ("/data/environments/talons_junction/track_reversed.vex", 862),
    ] {
        let Some(loaded) = load(HD, Some(track)) else {
            return;
        };
        let line = loaded.report.iter().find(|l| l.contains(".probes"));
        let found = loaded.probes.as_ref().map(oag_vex::probes::Probes::len);
        assert_eq!(found, Some(probes), "{track}: {line:?}");
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_title_with_no_probes_loads_none() {
    let Some(loaded) = load(PULSE, None) else {
        return;
    };
    assert!(loaded.probes.is_none());
}

/// Every circuit loads its own probes, and **no circuit surface is tagged as reading the
/// reflection probe**: the tag is for hull materials, and a track (water included, whose
/// program reads a probe of its own) keeps the glow index its own family took. A hull
/// is tagged on every circuit, which is the positive control.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_circuit_loads_probes_and_none_of_its_surfaces_reflects_them() {
    for environment in ENVIRONMENTS {
        let track = format!("/data/environments/{environment}/track.vex");
        let Some(loaded) = load(HD, Some(&track)) else {
            return;
        };
        let count = loaded.probes.as_ref().map(oag_vex::probes::Probes::len);
        assert!(count.is_some_and(|n| n >= 2), "{environment}: {count:?}");
        let tagged = |line: &&String| line.contains("reading the reflection probe");
        for line in loaded.report.iter().filter(tagged) {
            assert!(
                line.starts_with("Data\\Ships\\"),
                "{environment}: a surface that is not a hull was tagged: {line}"
            );
        }
        assert!(
            loaded.report.iter().any(|line| tagged(&line)),
            "{environment}: no hull was tagged at all"
        );
    }
}

/// Wipeout 2048 ships a `.probes` beside every circuit in a **different layout**
/// (little-endian, a second header word of 1 or 2). Its load says so by name rather than
/// reporting HD's reader failing, and binds no probes.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_2048_circuit_names_its_probes_as_another_layout() {
    let package = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/extracted/vita/PCSF00007");
    if !package.exists() {
        eprintln!("skipping: {} not present", package.display());
        return;
    }
    let loaded = race::load(&race::Options {
        source: package.display().to_string(),
        class: "VENOM".to_string(),
        track: Some("Data\\art\\published\\environments\\altima\\track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    assert!(loaded.probes.is_none());
    let line = loaded.report.iter().find(|l| l.contains(".probes"));
    assert!(
        line.is_some_and(|l| l.contains("a different layout")),
        "{line:?}"
    );
}
