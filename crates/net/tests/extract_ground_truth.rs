//! The server extractor against the six sources a lobby can host: each is extracted, every
//! race the extractor plans is loaded from the extraction and must reproduce the source's
//! tick-0 state hash (`extract` does that itself and reports a `gap`), and then one
//! race is run tick by tick from both and its state hash compared at every tick.
//!
//! A race here is the AI field flying the grid (no person's input) for a minute and a half of
//! ticks: the simulation the server runs, with nothing the extraction could be hiding behind.

use std::path::{Path, PathBuf};

use oag_gameplay::PlayerInputs;
use oag_net::wire::RaceParams;

/// Ticks flown on the first circuit: a minute and a half at 60 Hz, past the start, the first
/// corners and the AI field spreading out. A three-lap race is 15,000 and more, which at the
/// simulation's debug-profile speed (about 250 ticks a second on a loaded machine) is five minutes
/// a title, over the test ceiling.
const LONG: usize = 5_400;
/// The grid of eight people is only compared this far: every seat's craft is read at load.
const SHORT: usize = 600;

fn work_dir(name: &str) -> PathBuf {
    let dir = oag_testdata::repo_root().join("target/oag-extract-ground-truth");
    std::fs::create_dir_all(&dir).expect("work dir");
    let out = dir.join(name);
    let _ = std::fs::remove_dir_all(&out);
    out
}

fn hashes(source: &str, params: &RaceParams, ticks: usize) -> Vec<u64> {
    let (mut race, _) = oag_net::race::load(params, source).expect("the race loads");
    let none = PlayerInputs::none();
    (0..ticks)
        .map(|_| {
            race.tick_sim_only(&none);
            race.sim.state_hash()
        })
        .collect()
}

/// Extracts `source`, requires every planned race to load identically from the result, and
/// compares a full race on the first circuit and an eight-person one on the middle circuit.
fn check(source: &Path, name: &str) {
    let out = work_dir(name);
    let source = source.display().to_string();
    let report =
        oag_net::extract::extract(&source, &out, "ground truth", |_| {}).expect("extracts");
    assert!(report.gaps.is_empty(), "{name}: {:#?}", report.gaps);
    assert!(
        report.loaded > 20,
        "{name}: only {} races loaded",
        report.loaded
    );
    let offer = oag_net::extract::offer(&source).expect("offers");
    let extraction = out.display().to_string();
    for (circuit, people, seed, ticks) in
        [(0, 1u8, 7, LONG), (offer.tracks.len() / 2, 8, 11, SHORT)]
    {
        let params = RaceParams {
            title: offer.title.to_string(),
            track: offer.tracks[circuit].clone(),
            class: "VENOM".into(),
            mode: "single_race".into(),
            difficulty: "elite".into(),
            scheme: "veteran".into(),
            seed,
            people,
            opponents: true,
            teams: offer
                .teams
                .iter()
                .take(usize::from(people))
                .cloned()
                .collect(),
            team: offer.teams.first().cloned().unwrap_or_default(),
            weapons: false,
        };
        let fly = |at: &str| {
            let (at, params) = (at.to_string(), params.clone());
            std::thread::Builder::new()
                .stack_size(64 << 20)
                .spawn(move || hashes(&at, &params, ticks))
                .expect("spawns")
        };
        let (a, b) = (fly(&source), fly(&extraction));
        let (from_source, from_extraction) = (
            a.join().expect("source race"),
            b.join().expect("extraction race"),
        );
        assert_eq!(
            from_source.len(),
            from_extraction.len(),
            "{name}: ticks flown"
        );
        let missed = oag_assets::record::take_misses();
        assert!(
            missed.is_empty(),
            "{name}: reads the extraction could not answer: {missed:#?}"
        );
        let first_difference = from_source
            .iter()
            .zip(&from_extraction)
            .position(|(a, b)| a != b);
        assert_eq!(first_difference, None, "{name}: {}", params.track);
        assert!(
            from_source.windows(2).any(|pair| pair[0] != pair[1]),
            "{name}: the race never moved, so the comparison proves nothing"
        );
    }
    let _ = std::fs::remove_dir_all(&out);
}

macro_rules! per_title {
    ($($test:ident: $image:expr;)*) => {$(
        #[test]
        #[ignore = "needs the title's disc image or extracted folder"]
        fn $test() {
            if let Some(source) = oag_testdata::exact($image) {
                check(&source, stringify!($test));
            }
        }
    )*};
}

per_title! {
    pulse_psp_extraction_reproduces_its_races: "pulse-psp-eu.chd";
    pulse_ps2_extraction_reproduces_its_races: "pulse-ps2-eu.chd";
    pure_extraction_reproduces_its_races: "pure-psp-eu.chd";
    hd_extraction_reproduces_its_races: "hdfury-ps3-eu.iso";
    wipeout_2048_extraction_reproduces_its_races: "data/extracted/vita";
    omega_extraction_reproduces_its_races: "data/extracted/ps4";
}
