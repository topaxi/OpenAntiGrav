//! A Wipeout HD race load carries the circuit's baked environment probes, and a title
//! that ships none carries none.
//!
//! **`#[ignore]`d, needs the decrypted HD image** (and Pulse's for the negative case).
//! Dropping the `.probes` read from the load turns the first test red: the hull would
//! silently keep its constant ambient.
//!
//! The same loads carry a second claim, [`runs_no_cage`]: no HD circuit runs the collision
//! cage, Chenghou Project included although its header sets Pulse's flag, and Pulse's
//! circuit still does (`oag_title::craft_laws::CraftLaws::cage_from_track_header`).

use oag_raceplay as race;

const HD: &str = "data/images/hdfury-ps3-eu-dec.iso";
const PULSE: &str = "data/images/pulse-psp-usa.chd";

/// HD's original runs no cage off the track header (`docs/ghidra/functions/ps3-hdfury-eu/
/// collision-cage.md`): on Chenghou Project ours pulled the craft down on both jumps with a
/// one-tick `-35` vertical velocity step the original never shows. Chenghou is the one
/// circuit whose header sets the flag, so its load says it held the cage off by the title.
fn runs_no_cage(environment: &str, loaded: &race::Loaded) {
    assert!(
        loaded.setup.spline.cage().is_none(),
        "{environment}: an HD circuit runs the collision cage"
    );
    if environment == "04_chenghou_project" {
        assert!(
            loaded
                .report
                .iter()
                .any(|l| l.contains("runs no cage off the track header's flag")),
            "{environment}: the header flag is set, so the title must be what holds it off"
        );
    }
}

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
    let track = "/data/environments/talons_junction/track_reversed.vex";
    let Some(loaded) = load(HD, Some(track)) else {
        return;
    };
    loads_its_own_probes(track, &loaded);
}

/// Talon's Junction's 862 probes, forward and reversed, and no SH probes beside them.
fn loads_its_own_probes(track: &str, loaded: &race::Loaded) {
    let line = loaded.report.iter().find(|l| l.contains(".probes"));
    let found = loaded.probes.hd.as_ref().map(oag_vex::probes::Probes::len);
    assert_eq!(found, Some(862), "{track}: {line:?}");
    assert!(loaded.probes.sh.is_none(), "{track}: HD binds no SH probes");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_title_with_no_probes_loads_none() {
    let Some(loaded) = load(PULSE, None) else {
        return;
    };
    assert!(loaded.probes.hd.is_none() && loaded.probes.sh.is_none());
    assert!(
        loaded.setup.spline.cage().is_some(),
        "Pulse's circuit runs the collision cage off its header flag"
    );
}

/// Every circuit loads its own probes, and **no circuit surface is tagged as reading the
/// reflection probe**: the tag is for hull materials, and a track (water included, whose
/// program reads a probe of its own) keeps the glow index its own family took. A hull
/// is tagged on every circuit, which is the positive control. Talon's Junction's forward
/// load also carries [`an_hd_circuit_loads_its_own_probes`]'s count.
fn every_circuit_loads_probes_and_none_of_its_surfaces_reflects_them(environment: &str) {
    let track = format!("/data/environments/{environment}/track.vex");
    let Some(loaded) = load(HD, Some(&track)) else {
        return;
    };
    if environment == "talons_junction" {
        loads_its_own_probes(&track, &loaded);
    }
    runs_no_cage(environment, &loaded);
    let count = loaded.probes.hd.as_ref().map(oag_vex::probes::Probes::len);
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

/// One test per HD circuit on the disc, forward: each is a whole race load, so the circuits
/// run side by side rather than in series in one test.
mod every_circuit {
    macro_rules! circuits {
        ($($name:ident: $environment:literal;)*) => {$(
            #[test]
            #[ignore = "needs a disc image in data/images/"]
            fn $name() {
                super::every_circuit_loads_probes_and_none_of_its_surfaces_reflects_them(
                    $environment,
                );
            }
        )*};
    }

    circuits! {
        amphiseum: "amphiseum";
        modesto_heights: "modesto_heights";
        talons_junction: "talons_junction";
        tech_de_ra: "tech_de_ra";
        zone_1: "zone_1";
        zone_2: "zone_2";
        zone_3: "zone_3";
        zone_4: "zone_4";
        vineta_k: "01_vineta_k";
        track_02: "02_track";
        track_03: "03_track";
        chenghou_project: "04_chenghou_project";
        ubermall: "05_ubermall";
        sebenco_climb: "10_sebenco_climb";
        sol_2: "12_sol_2";
        anulpha_pass: "15_anulpha_pass";
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
    assert!(loaded.probes.hd.is_none());
    assert!(loaded.probes.sh.is_none(), "2048 ships no .shprobes");
    let line = loaded.report.iter().find(|l| l.contains(".probes"));
    assert!(
        line.is_some_and(|l| l.contains("a different layout")),
        "{line:?}"
    );
}
