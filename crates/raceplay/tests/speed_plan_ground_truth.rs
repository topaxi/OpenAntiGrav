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
//! **A layout's verdict is the majority of five builds** (2026-10-20, lane
//! `ai-gate-robustness`), one per scale of `perturb::GATE`
//! (`tests/perturb/mod.rs`: every craft's airborne gravity times `1 + eps`).
//! A single build is a coin on the circuits whose verification turns on a
//! landing: over the nine scales of `perturb::SCALES`, `06_Track` VENOM and
//! RAPIER failed under 5 and 6 of the 8 non-zero scales while clean at `0`, and
//! `14_Track` PHANTOM verified clean under 7 of 8 while not clean at `0`.

use std::path::PathBuf;

use oag_raceplay as race;
use oag_raceplay::catalogue;

use crate::perturb;

/// `(layout, class)` pairs whose plan does not verify clean, and why each is
/// believed to be so. Every one is named on `docs/gameplay/ai.md`'s speed-plan
/// section.
const NOT_CLEAN: &[(&str, &str)] = &[("29_Track", "PHANTOM")];

/// `(layout, class)` pairs whose majority flips between offsets of
/// `perturb::BOUND_OFFSETS`, so neither verdict is asserted; each with the
/// share of offsets whose majority verified clean (2026-10-20). Printed, so a
/// change that settles one is visible. `14_Track` PHANTOM, on [`NOT_CLEAN`]
/// before 2026-10-20 on its single build, is clean under all nine.
const UNSETTLED: &[(&str, &str)] = &[
    // 3 of 9: the gap landing at 1233-1241 (walls at 1233-1239).
    ("06_Track", "VENOM"),
    // 8 of 9 (2026-10-20, lane `pulse-air-pitch`; 0 of 9 before, on `NOT_CLEAN`): the same
    // gap landing, settled by the hover escape off a wall under a probe
    // (`oag_physics::hover::Rig::escape_floor_only`). Only offset `5e-5` failed its majority.
    ("06_Track", "RAPIER"),
    // 8 of 9: a rescue at 2402-2405.
    ("29_Track", "RAPIER"),
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
        if UNSETTLED.contains(&(id.as_str(), class)) {
            println!("{id} {class}: unsettled, not asserted");
        } else if clean != expected {
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
