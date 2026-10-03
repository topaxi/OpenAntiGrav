//! A lone opponent's forward speed against the speed plan's target, a line
//! per tick, for reading where a race departs from the plan's own lap.
//!
//! ```sh
//! cargo run --release -p oag-game --example speed_plan_trace -- 06_Track FLASH [rev] > trace.tsv
//! ```

use oag_game::race;
use oag_gameplay::PlayerInputs;

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
        class,
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(entry),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    race.sim.world.ships[0].active = false;
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    if std::env::var_os("OAG_TRACE_PLAIN").is_some() {
        race.sim.world.ships[1].driver.seed = 0;
    }
    println!("tick\tlap\tindex\tspeed\ttarget\tpace\tcontact");
    for tick in 0..12_000u32 {
        race.tick(&PlayerInputs::none());
        let ship = &race.sim.world.ships[1];
        let body = &ship.physics.body;
        let speed = body.linear_velocity.dot(body.forward());
        let index = ship.driver.index as usize;
        let (target, pace) = race.speed_plan().map_or((f32::NAN, f32::NAN), |plan| {
            (plan.target(index), plan.pace(index))
        });
        println!(
            "{tick}\t{}\t{index}\t{speed:.1}\t{target:.1}\t{pace:.1}\t{}",
            ship.standing.lap,
            race.wall_contact_ticks_of(1)
        );
        if ship.standing.finished() {
            break;
        }
    }
}
