//! Builds the AI speed plan for one layout and class and prints what the build
//! did: steps, passes, unresolved corners, the verification lap, wall time.
//!
//! ```sh
//! cargo run --release -p oag-game --example speed_plan_probe -- 07_Track VENOM [rev]
//! ```
//!
//! See `docs/gameplay/ai.md`, "The speed plan".

use std::time::Instant;

use oag_game::race;

fn main() {
    let mut args = std::env::args().skip(1);
    let track = args.next().unwrap_or_else(|| "16_Track".to_string());
    let class = args.next().unwrap_or_else(|| "VENOM".to_string());
    let reversed = args.next().is_some_and(|a| a == "rev");
    let image =
        std::env::var("OAG_IMAGE").unwrap_or_else(|_| "data/images/pulse-psp-usa.chd".to_string());
    let mut archives = oag_pulse::open(&image).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    let entry = oag_game::catalogue::tracks(&definition)
        .into_iter()
        .find(|t| t.reversed == reversed && t.id == track)
        .expect("a layout by that id")
        .entry_name();
    let loaded = race::load(&race::Options {
        source: image,
        class: class.clone(),
        mode: oag_race::Mode::SingleRace,
        track: Some(entry),
        team: std::env::var("OAG_TEAM").ok(),
        ..race::Options::default()
    })
    .expect("loading the race");
    let race = race::Race::start(loaded.setup);
    let begun = Instant::now();
    let (plan, report) = race.build_speed_plan(1);
    let took = begun.elapsed();
    println!(
        "{track} {class} {}: {:.2}s, {} steps ({:.1} us/step), passes {}, lowerings {}, \
         unresolved {:?}, respawns {}, verify failures {} contacts {} respawns {} lap {:?}",
        if reversed { "rev" } else { "fwd" },
        took.as_secs_f32(),
        report.steps,
        took.as_secs_f64() * 1e6 / report.steps.max(1) as f64,
        report.passes,
        report.lowerings,
        report.unresolved,
        report.respawns,
        report.verify_failures,
        report.verify_contacts,
        report.verify_respawns,
        report.verify_lap_ticks.map(|t| t as f32 / 60.0),
    );
    if std::env::var_os("OAG_PLAN_DUMP").is_some() {
        for index in (0..plan.len()).step_by(25) {
            println!(
                "{index:5} ceiling {:8.1} target {:8.1}{}",
                plan.ceiling(index),
                plan.target(index),
                if plan.holds(index) { " hold" } else { "" }
            );
        }
    }
}
