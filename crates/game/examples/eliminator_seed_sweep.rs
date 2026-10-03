//! The Eliminator finish sweep: seeds 1 to 24, player parked, target five,
//! `16_Track` at VENOM - the measurement `eliminator_finish_ground_truth.rs`
//! pins four seeds of, and the handover thread quotes as a median.
//!
//! ```sh
//! cargo run --release -p oag-game --example eliminator_seed_sweep
//! ```

use oag_game::race;
use oag_gameplay::PlayerInputs;

/// Six game-minutes, the finish test's own window.
const TICKS: u64 = 60 * 60 * 6;

fn main() {
    let image =
        std::env::var("OAG_IMAGE").unwrap_or_else(|_| "data/images/pulse-psp-usa.chd".to_string());
    let parked = PlayerInputs::single(oag_gameplay::InputSnapshot::new());
    let mut finishes = Vec::new();
    for seed in 1..=24u64 {
        let loaded = race::load(&race::Options {
            source: image.clone(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::Eliminator,
            eliminator_kill_target: Some(5),
            seed: Some(seed),
            ..race::Options::default()
        })
        .expect("loading the race");
        let mut race = race::Race::start(loaded.setup);
        let mut ended = None;
        for tick in 0..TICKS {
            race.tick(&parked);
            if race.sim.world.primary_race().finished {
                ended = Some(tick as f32 / 60.0);
                break;
            }
        }
        let kills: Vec<u32> = (0..race.sim.world.ship_count as usize)
            .map(|slot| race.sim.world.ships[slot].standing.kills)
            .collect();
        println!("seed {seed:2}: {ended:?} s, kills {kills:?}");
        if let Some(seconds) = ended {
            finishes.push(seconds);
        }
    }
    finishes.sort_by(f32::total_cmp);
    let median = finishes.get(finishes.len() / 2).copied();
    println!(
        "{} of 24 finish, {:?} to {:?} s, median {median:?} s (original 85 s)",
        finishes.len(),
        finishes.first(),
        finishes.last()
    );
}
