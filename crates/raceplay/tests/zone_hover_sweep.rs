//! A scratch sweep: every Pulse circuit raced in Zone with the player on autopilot, printing how
//! the four-corner hover flies it. Printing only, so `#[ignore]`d and gated on `OAG_SWEEP`.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_raceplay::catalogue;

const TICKS: u64 = 7_200;

#[test]
#[ignore = "sweep: prints, asserts nothing; set OAG_SWEEP=1"]
fn zone_hover_sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(path) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let source = path.display().to_string();
    let mut archives = oag_pulse::open(&source).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    for track in catalogue::tracks(&definition)
        .into_iter()
        .filter(|track| !track.reversed)
    {
        let Ok(loaded) = race::load(&race::Options {
            source: source.clone(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::Zone,
            track: Some(track.entry_name()),
            ..race::Options::default()
        }) else {
            println!("{:<10} did not load", track.id);
            continue;
        };
        let mut zone = race::Race::start(loaded.setup);
        zone.set_autopilot(true);
        let mut lap = zone.sim.world.ships[0].standing.lap;
        let mut lap_shield = Vec::new();
        let mut ticks = 0;
        let mut airborne = 0u32;
        let mut partial = 0u32;
        let mut speed = 0.0f64;
        for tick in 0..TICKS {
            zone.tick(&PlayerInputs::none());
            ticks = tick + 1;
            let ship = &zone.sim.world.ships[0];
            if ship.standing.lap != lap {
                lap = ship.standing.lap;
                lap_shield.push(ship.physics.shield);
            }
            let grounded = ship.physics.grounded;
            if grounded <= 0.0 {
                airborne += 1;
            } else if grounded < 1.0 {
                partial += 1;
            }
            speed += f64::from(ship.physics.body.linear_velocity.length());
            if !ship.active {
                break;
            }
        }
        let ship = &zone.sim.world.ships[0];
        println!(
            "{:<10} probes {} ticks {:<5} air {:<4} partial {:<4} mean speed {:.1} laps {:<3} respawns {:<3} per-lap shield {:?} end shield {:.1}",
            track.id,
            ship.physics.hover_rig.count,
            ticks,
            airborne,
            partial,
            speed / ticks as f64,
            ship.standing.lap,
            zone.respawns_of(0),
            lap_shield
                .iter()
                .map(|s| (s * 10.0).round() / 10.0)
                .collect::<Vec<_>>(),
            ship.physics.shield,
        );
    }
}
