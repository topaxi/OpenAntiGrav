//! The AI speed plan, built on every layout on the disc at every speed class.
//!
//! `oag_ai::SpeedPlan::build` drives the racing line in our own physics and
//! learns where it has to brake (`docs/gameplay/ai.md`, "The speed plan"). Its
//! own verification is two laps from the grid following the finished plan,
//! with nothing learned; a plan is **clean** when that run touches no wall and
//! never has to be put back on the line.
//!
//! [`NOT_CLEAN`] is the exact set that is not, asserted as a set like
//! `ai_clean_lap_gate.rs`'s status column: a layout leaving it is an
//! improvement and a failure that says to tighten the list, a layout joining
//! it is a regression. Split by class and direction so nextest runs eight
//! slices side by side.
//!
//! **A layout's verdict is the majority of five builds**, one per scale of
//! `perturb::GATE` (`tests/perturb/mod.rs`: every craft's airborne gravity
//! times `1 + eps`). A single build is a coin on the circuits whose
//! verification turns on a landing (`06_Track`'s gap at 1196-1200).
//!
//! All 96 but `06_Track` RAPIER ([`NOT_CLEAN`]) verify, each under every offset of `perturb::BOUND_OFFSETS` and
//! `perturb::HELD_OUT` (`docs/gameplay/ai.md`, "Plans that do not verify").

use std::path::PathBuf;

use oag_raceplay as race;
use oag_raceplay::catalogue;

use crate::perturb;

/// `(layout, class)` pairs whose plan does not verify clean, and why each is
/// believed to be so. A row that joins it is named on
/// `docs/gameplay/ai.md`'s speed-plan section.
const NOT_CLEAN: &[(&str, &str)] = &[
    // The damped landing coin after 06's gap at 1196-1200: with Pulse's body damping the
    // verification meets the walls at 1232-1242 on 6 of 17 offsets (2 of 17 undamped), which
    // stay unresolved in clean builds either way (`docs/gameplay/ai.md`, "06_Track RAPIER with
    // the body damping").
    ("06_Track", "RAPIER"),
];

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn layouts(reversed: bool) -> Vec<(String, String)> {
    let Some(image) = image() else {
        return Vec::new();
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    catalogue::tracks(&definition)
        .into_iter()
        .filter(|track| track.reversed == reversed)
        .map(|track| (track.id.clone(), track.entry_name()))
        .collect()
}

fn slice(class: &str, reversed: bool) {
    let Some(image) = image() else {
        return;
    };
    let mut wrong = Vec::new();
    for (id, entry) in layouts(reversed) {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: class.to_string(),
            mode: oag_race::Mode::SingleRace,
            track: Some(entry.clone()),
            ..race::Options::default()
        })
        .unwrap_or_else(|error| panic!("loading {entry} at {class}: {error}"));
        let builds: Vec<_> = perturb::GATE
            .iter()
            .map(|&eps| {
                let mut setup = loaded.setup.clone();
                perturb::apply(&mut setup, eps);
                let race = race::Race::start(setup);
                let (plan, report) = race.build_player_speed_plan();
                assert_eq!(plan.len(), race.racing_line().len());
                (plan, report)
            })
            .collect();
        let verdicts: Vec<bool> = builds
            .iter()
            .map(|(_, report)| report.verify_failures == 0 && report.verify_respawns == 0)
            .collect();
        let clean = verdicts.iter().filter(|&&c| c).count() * 2 > verdicts.len();
        let (plan, report) = &builds[0];
        let expected = !NOT_CLEAN.contains(&(id.as_str(), class));
        println!(
            "{id} {class}: clean {clean} {verdicts:?}, steps {}, lap {:?}, unresolved {:?}, \
             tight {} of {}",
            report.steps,
            report.verify_lap_ticks,
            report.unresolved,
            plan.tight_samples(),
            plan.len()
        );
        if clean != expected {
            wrong.push(format!(
                "{id} {class}: {} (builds clean {verdicts:?}; unperturbed: verify failures {}, \
                 respawns {}, unresolved {:?})",
                if clean {
                    "now clean - remove it from NOT_CLEAN"
                } else {
                    "no longer clean - a regression"
                },
                report.verify_failures,
                report.verify_respawns,
                report.unresolved
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

macro_rules! slice_test {
    ($name:ident, $class:expr, $reversed:expr) => {
        #[test]
        #[ignore = "needs a disc image in data/images/"]
        fn $name() {
            slice($class, $reversed);
        }
    };
}

slice_test!(speed_plan_venom_forward, "VENOM", false);
slice_test!(speed_plan_venom_reversed, "VENOM", true);
slice_test!(speed_plan_flash_forward, "FLASH", false);
slice_test!(speed_plan_flash_reversed, "FLASH", true);
slice_test!(speed_plan_rapier_forward, "RAPIER", false);
slice_test!(speed_plan_rapier_reversed, "RAPIER", true);
slice_test!(speed_plan_phantom_forward, "PHANTOM", false);
slice_test!(speed_plan_phantom_reversed, "PHANTOM", true);

/// Plans learned ahead on a thread (`Race::begin_speed_plans`, what the load
/// screen starts) are the plans the first tick would have learned, and adopting
/// them is not the cost of learning them. Outpost 7 (`07_Track`) has a fork, so
/// its race learns plans even under the original driver; run to 120 ticks, the
/// two races have the same state hash.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn plans_learned_ahead_are_the_plans_the_first_tick_learns() {
    use std::time::{Duration, Instant};

    let Some(image) = image() else {
        return;
    };
    let (_, entry) = layouts(false)
        .into_iter()
        .find(|(id, _)| id == "07_Track")
        .expect("07_Track is on the disc");
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "RAPIER".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(entry),
        ..race::Options::default()
    })
    .expect("loading 07_Track");
    let mut lazy = race::Race::start(loaded.setup.clone());
    let mut ahead = race::Race::start(loaded.setup);
    assert!(
        lazy.use_original_ai(None, 1) && ahead.use_original_ai(None, 1),
        "the original driver flies, so only the fork asks for plans"
    );

    let starting = Instant::now();
    ahead.begin_speed_plans();
    println!("starting the job took {:?}", starting.elapsed());
    let waited = Instant::now();
    while !ahead.speed_plans_ready() {
        assert!(
            waited.elapsed() < Duration::from_secs(300),
            "the plans never landed"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let adopting = Instant::now();
    ahead.ensure_speed_plans();
    let adopted = adopting.elapsed();
    let learning = Instant::now();
    lazy.ensure_speed_plans();
    let learned = learning.elapsed();
    println!("adopting {adopted:?}, learning on the frame thread {learned:?}");
    assert!(
        adopted * 20 < learned,
        "a first tick after the plans landed must not learn them: adopting {adopted:?}, learning {learned:?}"
    );

    assert_eq!(ahead.speed_plan().cloned(), lazy.speed_plan().cloned());
    for slot in 0..8 {
        assert_eq!(
            ahead.plan_of(slot).cloned(),
            lazy.plan_of(slot).cloned(),
            "slot {slot}"
        );
    }
    for tick in 0..120 {
        ahead.tick_sim_only(&oag_gameplay::PlayerInputs::none());
        lazy.tick_sim_only(&oag_gameplay::PlayerInputs::none());
        assert_eq!(ahead.sim.state_hash(), lazy.sim.state_hash(), "tick {tick}");
    }
}

/// Whether the ring plan of `track` on `source` verifies clean at `class`:
/// the majority of the [`perturb::GATE`] builds, as [`slice`] judges a layout.
fn ring_plan_verifies(source: &std::path::Path, track: &str, class: &str) -> bool {
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .unwrap_or_else(|error| panic!("loading {track} at {class}: {error}"));
    let verdicts: Vec<bool> = perturb::GATE
        .iter()
        .map(|&eps| {
            let mut setup = loaded.setup.clone();
            perturb::apply(&mut setup, eps);
            let (_, report) = race::Race::start(setup).build_player_speed_plan();
            println!(
                "{track} {class} {eps}: failures {}, respawns {}, lap {:?}, unresolved {:?}",
                report.verify_failures,
                report.verify_respawns,
                report.verify_lap_ticks,
                report.unresolved
            );
            report.verify_failures == 0 && report.verify_respawns == 0
        })
        .collect();
    verdicts.iter().filter(|&&clean| clean).count() * 2 > verdicts.len()
}

/// 2048's `square` has a corner (ring samples 577-641) a craft at full
/// throttle runs wide of, off the edge and onto a level 20 units under the
/// line, which it then drives for 900 samples to a wall. The plan counts that
/// road as a failure and brakes for the corner (`docs/gameplay/ai.md`, "A road
/// under the line is a failure"). PHANTOM is the class that takes off
/// furthest from where it lands. `arena`'s line runs 21 units over its own
/// road for 250 samples with the craft inside its corridor, which is not one.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_2048_square_ring_plan_brakes_for_the_corner_it_ran_wide_of() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let track = |name: &str| format!(r"Data\art\published\environments\{name}\track.vex");
    assert!(
        ring_plan_verifies(&source, &track("square"), "PHANTOM"),
        "square"
    );
    assert!(
        ring_plan_verifies(&source, &track("arena"), "VENOM"),
        "arena's line is over its own road"
    );
}

/// Omega re-ships `square` at the same layout, and its plan fails and
/// verifies at the same corner.
#[test]
#[ignore = "needs the extracted game under data/extracted/ps4/"]
fn the_omega_square_ring_plan_brakes_for_the_corner_it_ran_wide_of() {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    assert!(
        ring_plan_verifies(
            &source,
            r"Data\environments2048\square\track.vex",
            "PHANTOM"
        ),
        "square"
    );
}
