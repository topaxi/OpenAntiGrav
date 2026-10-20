//! A Wipeout HD race load carries the circuit's baked environment probes, and a title
//! that ships none carries none.
//!
//! **`#[ignore]`d, needs the decrypted HD image** (and Pulse's for the negative case).
//! Dropping the `.probes` read from the load turns the first test red: the hull would
//! silently keep its constant ambient.

use oag_raceplay as race;

const HD: &str = "data/images/hdfury-ps3-eu-dec.iso";
const PULSE: &str = "data/images/pulse-psp-usa.chd";

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
