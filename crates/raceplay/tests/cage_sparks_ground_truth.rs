//! The collision cage's contacts throw the original's no-damage collision spark and its
//! `.COLLISIONS` cue (`FUN_088418e0`'s contact-ring loop, `Ship_DispatchCollisionFx`), on Pulse's
//! own circuit and the disc's own `WO_SHIP_COLL_SPARK_NODAMAGE.POB`.
//!
//! De Konstruct Black's forward Ace field (`cage_effect_sweep`'s board: its cage contacts are
//! 208 resets and many more touches over 108 races) flown without a person for a few thousand
//! ticks: with the circuit's cage some craft's cage contact starts a spark, and with the cage
//! removed (`Spline::with_cage(None)`) no cage spark starts, so the count is the cage's and
//! nothing else's. `#[ignore]`d: needs the disc image, never runs in CI.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_raceplay::catalogue;

use crate::in_parallel;
use crate::perturb;

const TICKS: u64 = 6_000;

fn image() -> Option<std::path::PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn entry() -> Option<String> {
    let image = image()?;
    let mut archives = oag_pulse::open(&image.display().to_string()).ok()?;
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .ok()?;
    let definition = oag_tables::fexml::expand(&blob).ok()?;
    catalogue::tracks(&definition)
        .into_iter()
        .find(|track| track.location.ends_with("05_Track") && !track.reversed)
        .map(|track| track.entry_name())
}

/// The cage sparks started over `TICKS` and the cues raised, for the board at `eps`.
fn run(eps: f32, cage: bool) -> (u32, usize) {
    let image = image().expect("image");
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(entry().expect("05_Track")),
        seed: Some(1),
        ..race::Options::default()
    })
    .expect("loading");
    let mut setup = loaded.setup;
    perturb::apply(&mut setup, eps);
    if !cage {
        setup.spline = setup.spline.clone().with_cage(None);
    }
    let mut race = race::Race::start(setup);
    race.sim.world.ships[0].active = false;
    let mut cues = 0;
    for _ in 0..TICKS {
        race.tick(&PlayerInputs::none());
        cues += race
            .drain_cues()
            .into_iter()
            .filter(|cue| cue.cue == oag_gameplay::cue::Cue::Collision)
            .count();
    }
    (race.cage_sparks_started_for_tests(), cues)
}

#[test]
#[ignore = "needs the Pulse disc image"]
fn a_cage_contact_throws_the_no_damage_spark_and_the_cage_alone_does() {
    if image().is_none() {
        return;
    }
    let cells: Vec<(f32, bool)> = perturb::SCALES
        .iter()
        .flat_map(|&eps| [(eps, true), (eps, false)])
        .collect();
    let rows = in_parallel::map(&cells, 6, |&(eps, cage)| run(eps, cage));
    let (mut with, mut without) = (0, 0);
    for (&(eps, cage), &(sparks, cues)) in cells.iter().zip(&rows) {
        println!("eps {eps:e} cage {cage}: {sparks} cage spark(s), {cues} collision cue(s)");
        if cage {
            with += sparks;
        } else {
            without += sparks;
        }
    }
    assert!(
        with > 0,
        "no cage contact threw a spark in {} races",
        cells.len() / 2
    );
    assert_eq!(without, 0, "a race with no cage threw a cage spark");
}
