//! What one `oag_physics::step` costs for one craft against a real circuit's
//! collision, and what one `oag_ai::Driver::drive` costs beside it.
//!
//! Written for the AI speed plan (`docs/gameplay/ai.md`, "The speed plan"),
//! whose build cost is a count of physics steps, and for the short-horizon
//! rollout idea after it, whose feasibility is the same count multiplied out.
//! Needs a disc image, and is only meaningful in release:
//!
//! ```sh
//! cargo run --release -p oag-game --example physics_step_cost [track] [class]
//! ```
//!
//! The environment is built exactly as `Race::step_opponents` builds it - the
//! track sample the driver stands on and the one after - because the cost is
//! almost all collision raycasts, and a flat plane would measure nothing.

use std::time::{Duration, Instant};

use oag_physics::Environment;
use oag_raceplay as race;
use oag_raceplay::Spline;

fn main() {
    let mut args = std::env::args().skip(1);
    let track = args.next().unwrap_or_else(|| "16_Track".to_string());
    let class = args.next().unwrap_or_else(|| "VENOM".to_string());
    let image =
        std::env::var("OAG_IMAGE").unwrap_or_else(|_| "data/images/pulse-psp-usa.chd".to_string());
    let mut archives = oag_pulse::open(&image).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    let entry = oag_raceplay::catalogue::tracks(&definition)
        .into_iter()
        .find(|t| !t.reversed && t.id == track)
        .expect("a circuit by that id")
        .entry_name();
    let loaded = race::load(&race::Options {
        source: image,
        class: class.clone(),
        mode: oag_race::Mode::SingleRace,
        track: Some(entry),
        ..race::Options::default()
    })
    .expect("loading the race");
    let gravity = loaded.setup.class_gravity_scale;
    let race = race::Race::start(loaded.setup);

    let ship = race.sim.world.ships[1];
    let handling = ship.handling;
    let mut state = ship.physics;
    let mut driver = ship.driver;
    let line = race.racing_line().clone();
    let tuning = oag_ai::Tuning::default();
    let ceiling = oag_ai::hull_yaw_ceiling(&handling, &state.body);
    let dt = race.dt();

    let ticks = 10_000u32;
    let (mut in_step, mut in_drive) = (Duration::ZERO, Duration::ZERO);
    for _ in 0..ticks {
        let begun = Instant::now();
        let controls = driver.drive(
            &state,
            &oag_ai::Context {
                yaw_ceiling: Some(ceiling),
                ..oag_ai::Context::new(&line, &tuning)
            },
        );
        let driven = Instant::now();
        let index = driver.index as usize;
        let env = Environment {
            track_sample: race.ai_sample(index).map(Spline::track_sample),
            track_sample_next: race
                .ai_sample(index + 1)
                .map(Spline::track_sample)
                .or_else(|| race.ai_sample(index).map(Spline::track_sample)),
            class_gravity_scale: gravity,
            ..Environment::default()
        };
        let stepped = Instant::now();
        oag_physics::step(&mut state, &controls, &handling, &env, race.collision(), dt);
        in_step += stepped.elapsed();
        in_drive += driven - begun;
    }
    let step_us = in_step.as_secs_f64() * 1e6 / f64::from(ticks);
    let drive_us = in_drive.as_secs_f64() * 1e6 / f64::from(ticks);
    println!(
        "{track} {class}: {ticks} ticks, final index {}",
        driver.index
    );
    println!("  physics step  {step_us:8.2} us/tick");
    println!("  driver drive  {drive_us:8.2} us/tick");
    // Option 2 of the speed-plan brief: 8 craft x 9 candidates x 90 ticks.
    let rollout = 8.0 * 9.0 * 90.0;
    let per_replan_ms = rollout * (step_us + drive_us) / 1000.0;
    println!(
        "  rollout replan (8 x 9 x 90 = {rollout} steps): {per_replan_ms:.1} ms, \
         against a 16.7 ms tick"
    );
}
