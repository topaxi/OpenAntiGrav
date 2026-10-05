//! A lone Ace opponent sent down a circuit's first route, logged every tick
//! across a window of its line: the diagnostic behind `docs/gameplay/ai.md`'s
//! "Branch choice at a fork" findings.
//!
//! ```sh
//! cargo run --release -p oag-game --example fork_trace -- <source> <track entry> <pre-fork path> <from> <to> [class] [difficulty]
//! ```
use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

fn main() {
    let mut args = std::env::args().skip(1);
    let source = args.next().expect("source");
    let track = args.next().expect("track");
    let pre_fork: u16 = args.next().expect("pre-fork path").parse().expect("a path");
    let from: u32 = args.next().expect("from").parse().expect("an index");
    let to: u32 = args.next().expect("to").parse().expect("an index");
    let class = args.next().unwrap_or_else(|| "VENOM".to_string());
    let difficulty = match args.next().as_deref() {
        Some("novice") => oag_ai::Difficulty::Novice,
        _ => oag_ai::Difficulty::Ace,
    };
    let loaded = race::load(&race::Options {
        source,
        mode: oag_race::Mode::SingleRace,
        track: Some(track),
        difficulty,
        class,
        ..race::Options::default()
    })
    .expect("loading");
    let mut race = race::Race::start(loaded.setup);
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    race.sim.world.ships[0].active = false;
    for tick in 0..60 * 120u32 {
        {
            // Always draw route 1 at this fork.
            let b = &mut race.sim.world.ships[1].driver.branching;
            if b.route == 0 && b.decided_at == pre_fork + 1 {
                b.pending = 1;
            }
        }
        race.tick(&PlayerInputs::none());
        let ship = &race.sim.world.ships[1];
        let b = ship.driver.branching;
        let idx = ship.driver.index;
        if b.route == 1 && (from..to).contains(&idx) {
            println!(
                "t{tick:<5} lap {} idx {idx} y {:>6.1} v {:>6.1} air {:.2} contact {} resp {}",
                ship.standing.lap,
                ship.physics.body.position.y,
                ship.physics.body.linear_velocity.length(),
                ship.physics.time_airborne,
                race.wall_contact_ticks_of(1),
                race.respawns_of(1)
            );
        }
    }
}
