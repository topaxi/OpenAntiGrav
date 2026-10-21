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
/// The most ticks a one-lap race is flown to find its finish line (the longest lap is 4,000).
const FINISH: usize = 6_000;

fn work_dir(name: &str) -> PathBuf {
    let dir = oag_testdata::repo_root().join("target/oag-extract-ground-truth");
    std::fs::create_dir_all(&dir).expect("work dir");
    let out = dir.join(name);
    let _ = std::fs::remove_dir_all(&out);
    out
}

/// The state hash after each tick of `params` flown from `source`: `ticks` of them, or, when
/// `to_the_finish`, until every person has finished (their craft flown by the autopilot, which
/// stands in for the clients) and a second more, at most `ticks`.
fn hashes(source: &str, params: &RaceParams, ticks: usize, to_the_finish: bool) -> Vec<u64> {
    let (mut race, _) = oag_net::race::load(params, source).expect("the race loads");
    if to_the_finish {
        for slot in 0..usize::from(params.people) {
            race.sim.world.controllers[slot] = oag_gameplay::Controller::Local;
        }
        race.set_autopilot(true);
    }
    let none = PlayerInputs::none();
    let mut out = Vec::new();
    let mut after = 0;
    while out.len() < ticks && after < 60 {
        race.tick_sim_only(&none);
        out.push(race.sim.state_hash());
        if to_the_finish && race.finished() {
            after += 1;
        }
    }
    out
}

/// The races compared: one person on the first circuit, and eight people with the title's first
/// eight teams on the last .
fn races(offer: &Offer) -> Vec<(Planned, usize, bool)> {
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
            false,
        ),
        (
            Planned {
                label: "one person, one lap to the finish line".into(),
                params: RaceParams {
                    track: offer.tracks[0].clone(),
                    seed: 13,
                    people: 1,
                    laps: 1,
                    ..base.clone()
                },
            },
            FINISH,
            true,
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
            false,
        ),
    ]
}

/// Extracts `source` for the one race of [`races`] at `which`, requires it to load identically
/// from the result, then flies it from both and compares every tick. Each race is its own test
/// (and its own extraction, so the extraction is proven to serve that race alone), because one
/// test carrying all three ran past the per-test ceiling on one core.
fn check(source: &Path, name: &str, which: usize) {
    let out = work_dir(name);
    let source = source.display().to_string();
    let report = oag_net::extract::extract(
        &source,
        &out,
        "ground truth",
        &|offer| {
            let (planned, _, _) = races(offer).swap_remove(which);
            vec![planned]
        },
        |_| {},
    )
    .expect("extracts");
    assert!(report.gaps.is_empty(), "{name}: {:#?}", report.gaps);
    assert_eq!(report.loaded, 1, "{name}: {:#?}", report.refused);
    let offer = oag_net::extract::offer(&source).expect("offers");
    let extraction = out.display().to_string();
    let (planned, ticks, to_the_finish) = races(&offer).swap_remove(which);
    let params = planned.params;
    // One after the other, each on a thread with room for the race: a title's loaded race
    // is gigabytes resident, and six titles run at once.
    let fly = |at: &str| {
        let (at, params) = (at.to_string(), params.clone());
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || hashes(&at, &params, ticks, to_the_finish))
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
    assert!(
        !to_the_finish || from_source.len() < ticks,
        "{name}: {}: no finish line in {ticks} ticks",
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
    let _ = std::fs::remove_dir_all(&out);
}

macro_rules! per_title {
    ($($title:ident: $image:expr;)*) => {$(
        mod $title {
            use super::*;

            fn run(which: usize) {
                if let Some(source) = oag_testdata::exact($image) {
                    check(&source, &format!("{}_{which}", stringify!($title)), which);
                }
            }

            #[test]
            #[ignore = "needs the title's disc image or extracted folder"]
            fn first_circuit_reproduces() {
                run(0);
            }

            #[test]
            #[ignore = "needs the title's disc image or extracted folder"]
            fn one_lap_to_the_finish_reproduces() {
                run(1);
            }

            #[test]
            #[ignore = "needs the title's disc image or extracted folder"]
            fn full_grid_reproduces() {
                run(2);
            }
        }
    )*};
}

per_title! {
    pulse_psp: "pulse-psp-eu.chd";
    pulse_ps2: "pulse-ps2-eu.chd";
    pure: "pure-psp-eu.chd";
    hd: "hdfury-ps3-eu.iso";
    wipeout_2048: "data/extracted/vita";
    omega: "data/extracted/ps4";
}
