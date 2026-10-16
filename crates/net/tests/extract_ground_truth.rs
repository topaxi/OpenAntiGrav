//! The server extractor against the six sources a lobby can host: for each, the races below are
//! extracted, loaded again from the extraction (`extract` requires the same tick-0 state hash of
//! every one and reports a `gap` otherwise), and then each is flown tick by tick from the source
//! and from the extraction and its state hash compared at every tick.
//!
//! These tests run with oag-raceplay's `view` feature on (the crate's dev-dependency turns it on),
//! so the loads here read textures, meshes and sound a lean server never reads and the extractions
//! are supersets of a lean one: they prove the mechanism, not the sizes. `oag-server extract`
//! itself refuses to run in such a build (`oag_raceplay::VIEW`); the sizes in
//! docs/networking/online-lobby.md come from a lean build.
//!
//! The AI field flies the grid and nobody gives input: the simulation the server runs, with
//! nothing the extraction could be hiding behind. **Not a three-lap race**: that is 15,000 ticks
//! and more, and a title's loaded race is a gigabyte, so the long case flies 1,800 ticks (thirty
//! seconds, past the start and the first corners) and the eight-person grid 300. The whole offer
//! of every title is proven by `oag-server extract`'s own check, which loads every race the
//! server offers from the extraction (docs/networking/online-lobby.md, part 6).

use std::path::{Path, PathBuf};

use oag_gameplay::PlayerInputs;
use oag_net::extract::{Offer, Planned};
use oag_net::wire::RaceParams;

const LONG: usize = 1_800;
const SHORT: usize = 300;

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

/// The races compared: one person on the first circuit, and eight people with the title's first
/// eight teams on the last .
fn races(offer: &Offer) -> Vec<(Planned, usize)> {
    let base = RaceParams {
        title: offer.title.to_string(),
        class: "VENOM".into(),
        mode: "single_race".into(),
        difficulty: "elite".into(),
        scheme: "veteran".into(),
        opponents: true,
        weapons: false,
        ..RaceParams::default()
    };
    let seats: Vec<String> = offer.teams.iter().take(8).cloned().collect();
    vec![
        (
            Planned {
                label: "one person, first circuit".into(),
                params: RaceParams {
                    track: offer.tracks[0].clone(),
                    seed: 7,
                    people: 1,
                    ..base.clone()
                },
            },
            LONG,
        ),
        (
            Planned {
                label: "a full grid of people, last circuit".into(),
                params: RaceParams {
                    track: offer.tracks[offer.tracks.len() - 1].clone(),
                    seed: 11,
                    people: u8::try_from(seats.len()).unwrap_or(8),
                    team: seats[0].clone(),
                    teams: seats,
                    ..base
                },
            },
            SHORT,
        ),
    ]
}

/// Extracts `source` for [`races`], requires them to load identically from the result, then
/// flies each from both and compares every tick.
fn check(source: &Path, name: &str) {
    let out = work_dir(name);
    let source = source.display().to_string();
    let report = oag_net::extract::extract(
        &source,
        &out,
        "ground truth",
        &|offer| {
            races(offer)
                .into_iter()
                .map(|(planned, _)| planned)
                .collect()
        },
        |_| {},
    )
    .expect("extracts");
    assert!(report.gaps.is_empty(), "{name}: {:#?}", report.gaps);
    assert_eq!(report.loaded, 2, "{name}: {:#?}", report.refused);
    let offer = oag_net::extract::offer(&source).expect("offers");
    let extraction = out.display().to_string();
    for (planned, ticks) in races(&offer) {
        let params = planned.params;
        // One after the other, each on a thread with room for the race: a title's loaded race
        // is gigabytes resident, and six titles run at once.
        let fly = |at: &str| {
            let (at, params) = (at.to_string(), params.clone());
            std::thread::Builder::new()
                .stack_size(64 << 20)
                .spawn(move || hashes(&at, &params, ticks))
                .expect("spawns")
                .join()
                .expect("the race flies")
        };
        let (from_source, from_extraction) = (fly(&source), fly(&extraction));
        let missed = oag_assets::record::take_misses();
        assert!(
            missed.is_empty(),
            "{name}: {}: reads the extraction could not answer: {missed:#?}",
            planned.label
        );
        let first_difference = from_source
            .iter()
            .zip(&from_extraction)
            .position(|(a, b)| a != b);
        assert_eq!(first_difference, None, "{name}: {}", planned.label);
        assert!(
            from_source.windows(2).any(|pair| pair[0] != pair[1]),
            "{name}: {}: the race never moved, so the comparison proves nothing",
            planned.label
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
