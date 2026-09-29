//! Where the eight grid slots sit against the track's own corridor, on every
//! Pulse circuit in both directions.
//!
//! Prints one line per slot (`OAG_GRID=1` to run) and asserts nothing; the
//! asserting test is `every_slot_starts_on_the_track`.

use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_game::race;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

struct Circuit {
    id: String,
    entry: String,
    reversed: bool,
}

fn circuits_all() -> Vec<Circuit> {
    let Some(image) = image() else {
        return Vec::new();
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    oag_game::catalogue::tracks(&definition)
        .into_iter()
        .map(|track| Circuit {
            id: track.id.clone(),
            entry: track.entry_name(),
            reversed: track.reversed,
        })
        .collect()
}

fn start(circuit: &Circuit) -> Option<race::Race> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "Venom".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(circuit.entry.clone()),
        ..race::Options::default()
    })
    .ok()?;
    Some(race::Race::start(loaded.setup))
}

struct Slot {
    slot: usize,
    lateral: f32,
    left: f32,
    right: f32,
    mid: f32,
    section: u8,
    ground: bool,
}

fn slots(race: &race::Race) -> Vec<Slot> {
    let mut out = Vec::new();
    for (index, ship) in race.sim.world.ships.iter().enumerate() {
        if !ship.active {
            continue;
        }
        let position = ship.physics.body.position;
        let slot = if index == 0 { 8 } else { index };
        let (_, s, _) = race.spline().nearest(position).expect("samples");
        let lateral = (position - Vec3::from_array(s.pos)).dot(Vec3::from_array(s.lateral));
        let origin = position + Vec3::Y * 2.0;
        let ray = oag_physics::Ray::new(origin, Vec3::NEG_Y, 12.0);
        let ground = oag_physics::Raycaster::raycast(race.collision(), ray, None, false).is_some();
        out.push(Slot {
            slot,
            lateral,
            left: s.ai_bound_left,
            right: s.ai_bound_right,
            mid: 0.5 * (s.ai_bound_left + s.ai_bound_right),
            section: s.section_id,
            ground,
        });
    }
    out
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn grid_layout_survey() {
    if std::env::var_os("OAG_GRID").is_none() {
        return;
    }
    for circuit in circuits_all() {
        let Some(race) = start(&circuit) else {
            continue;
        };
        let label = format!("{}{}", circuit.id, if circuit.reversed { "r" } else { "" });
        for s in slots(&race) {
            println!(
                "GRID {label} slot {} lat {:+.2} corridor [{:+.2},{:+.2}] mid {:+.2} off_mid {:+.2} sec {} ground {}",
                s.slot,
                s.lateral,
                s.left,
                s.right,
                s.mid,
                s.lateral - s.mid,
                s.section,
                s.ground
            );
        }
    }
}
