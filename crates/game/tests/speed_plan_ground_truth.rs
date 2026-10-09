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

use std::path::PathBuf;

use oag_raceplay as race;
use oag_raceplay::catalogue;

/// `(layout, class)` pairs whose plan does not verify clean, and why each is
/// believed to be so. Every one is named on `docs/gameplay/ai.md`'s speed-plan
/// section.
const NOT_CLEAN: &[(&str, &str)] = &[
    // The corner after the gap at 1196-1200: walls at 1234-1238, and at
    // VENOM a stall at 1217 once pad boosts are in the learning run.
    ("06_Track", "VENOM"),
    ("06_Track", "RAPIER"),
    // Rescued - in the air too long, or off the line - at 1247 and at
    // 2402-2441, with every ceiling the search tried.
    ("14_Track", "PHANTOM"),
    ("29_Track", "RAPIER"),
    ("29_Track", "PHANTOM"),
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
        let race = race::Race::start(loaded.setup);
        let (plan, report) = race.build_speed_plan(1);
        assert_eq!(plan.len(), race.racing_line().len());
        let clean = report.verify_failures == 0 && report.verify_respawns == 0;
        let expected = !NOT_CLEAN.contains(&(id.as_str(), class));
        println!(
            "{id} {class}: clean {clean}, steps {}, lap {:?}, unresolved {:?}, tight {} of {}",
            report.steps,
            report.verify_lap_ticks,
            report.unresolved,
            plan.tight_samples(),
            plan.len()
        );
        if clean != expected {
            wrong.push(format!(
                "{id} {class}: {} (verify failures {}, respawns {}, unresolved {:?})",
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
