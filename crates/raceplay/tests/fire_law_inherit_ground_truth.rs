//! Every title's opponents fire on Pulse's law, on a real disc, and an
//! opponent the original driver flies spends what it picks up.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^fire_law_inherit_ground_truth::/)'
//! ```
//!
//! The maintainer's rule (2026-10-03): a title with no measured law of its
//! own runs Pulse's, never this project's chosen fallback. HD, Omega and 2048
//! each ship a `WeaponAIstats.xml` (`docs/gameplay/ai.md`), so each runs
//! `oag_ai::weapon_ai` on its own odds: **inherited from Pulse, unmeasured on
//! that title.** Dropping a title's `Weapons::ai` falls back to
//! `FireLaw::Ours` and fails here.
//!
//! Each title's single race then flies two minutes under the original driver
//! (`Race::use_original_ai`, what every campaign and race box race with
//! opponents runs) and counts the projectiles opponents put in the air. Until
//! 2026-10-19 that was **zero on all five titles**: the original branch never
//! advanced `oag_ai::Driver`'s noise phase, so every craft rolled one constant
//! number all race and held its first pickup to the flag
//! (`oag_ai::Driver::attend`, `docs/gameplay/ai.md`).

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_raceplay::FireLaw;

fn started(source: String, mode: oag_race::Mode) -> race::Race {
    let loaded = race::load(&race::Options {
        source,
        mode,
        seed: Some(1),
        ..race::Options::default()
    })
    .expect("loading the race");
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("opponents fire on its odds")),
        "no weapon-AI line in the loader report: {:#?}",
        loaded.report
    );
    race::Race::start(loaded.setup)
}

/// Seconds of racing after the green light [`opponents_fire_under_the_original_driver`]
/// flies. Two minutes: the slowest title measured (Pure) put 38 projectiles
/// up in four.
const SECONDS: u64 = 120;

/// Flies `race` under the original driver with the player parked and returns
/// how many projectiles opponents launched.
fn opponents_fire_under_the_original_driver(mut race: race::Race) -> u32 {
    assert!(race.use_original_ai(None, 1), "the original driver flies");
    let count = usize::from(race.ship_count());
    let mut held = vec![0usize; count];
    let mut fired = 0;
    while race.sim.world.tick < oag_race::COUNTDOWN_TICKS + SECONDS * 60 {
        race.tick(&PlayerInputs::none());
        let mut owned = vec![0usize; count];
        for projectile in &race.sim.world.projectiles.slots {
            let owner = usize::from(projectile.owner);
            if projectile.kind.is_some() && owner < count {
                owned[owner] += 1;
            }
        }
        for slot in 1..count {
            fired += owned[slot].saturating_sub(held[slot]) as u32;
            held[slot] = owned[slot];
        }
    }
    fired
}

/// One title, one mode: a race load each, which is most of a 2048 test's time, so the
/// modes are the test axis rather than a loop. A single race also flies.
fn fires_original(source: String, mode: oag_race::Mode) {
    let race = started(source, mode);
    assert_eq!(race.fire_law(), FireLaw::Original, "{mode:?}");
    if mode == oag_race::Mode::SingleRace {
        let fired = opponents_fire_under_the_original_driver(race);
        println!("opponent projectiles in {SECONDS} s: {fired}");
        assert!(fired > 0, "no opponent fired in {SECONDS} s");
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_pulse_single_race_fires_on_the_original_law() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    fires_original(image.display().to_string(), oag_race::Mode::SingleRace);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_pure_single_race_fires_on_the_original_law() {
    let Some(image) = oag_testdata::image("data/images/pure-psp-usa.chd") else {
        return;
    };
    fires_original(image.display().to_string(), oag_race::Mode::SingleRace);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_hd_single_race_fires_on_the_original_law() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    fires_original(image.display().to_string(), oag_race::Mode::SingleRace);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_hd_eliminator_fires_on_the_original_law() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    fires_original(image.display().to_string(), oag_race::Mode::Eliminator);
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_2048_single_race_fires_on_the_original_law() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    fires_original(path.display().to_string(), oag_race::Mode::SingleRace);
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_2048_eliminator_fires_on_the_original_law() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    fires_original(path.display().to_string(), oag_race::Mode::Eliminator);
}

#[test]
#[ignore = "needs the extracted Omega packages in data/extracted/ps4/"]
fn an_omega_single_race_fires_on_the_original_law() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    fires_original(path.display().to_string(), oag_race::Mode::SingleRace);
}

#[test]
#[ignore = "needs the extracted Omega packages in data/extracted/ps4/"]
fn an_omega_eliminator_fires_on_the_original_law() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    fires_original(path.display().to_string(), oag_race::Mode::Eliminator);
}
