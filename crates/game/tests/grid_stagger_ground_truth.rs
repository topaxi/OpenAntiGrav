//! Where the eight grid slots sit against the track's own corridor, on every
//! Pulse circuit in both directions.
//!
//! `grid_layout_survey` prints one line per slot (`OAG_GRID=1` to run) and asserts
//! nothing; `every_slot_starts_on_the_track` is the assertion, and
//! `metropia_reversed_is_the_originals_grid` pins the layout against eight craft
//! read out of the original.

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
    position: Vec3,
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
            position,
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
                "GRID {label} slot {} lat {:+.2} corridor [{:+.2},{:+.2}] mid {:+.2} off_mid {:+.2} sec {} ground {} pos ({:.2}, {:.2}, {:.2})",
                s.slot,
                s.lateral,
                s.left,
                s.right,
                s.mid,
                s.lateral - s.mid,
                s.section,
                s.ground,
                s.position.x,
                s.position.y,
                s.position.z
            );
        }
    }
}

/// What the regression gate's lone craft (slot 1, Ace, everyone else parked)
/// is doing when it respawns: tick, cause, where.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn lone_craft_respawns() {
    let Ok(only) = std::env::var("OAG_GRID_TRACK") else {
        return;
    };
    let Some(circuit) = circuits_all()
        .into_iter()
        .find(|c| c.id == only && !c.reversed)
    else {
        return;
    };
    let image = image().expect("image");
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(circuit.entry.clone()),
        ..race::Options::default()
    })
    .expect("load");
    let mut race = race::Race::start(loaded.setup);
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    race.sim.world.ships[0].active = false;
    let mut seen = race.respawns_of(1);
    for tick in 0..18_000u64 {
        race.tick(&oag_gameplay::PlayerInputs::none());
        if race.respawns_of(1) != seen {
            seen = race.respawns_of(1);
            let ship = &race.sim.world.ships[1];
            let position = ship.physics.body.position;
            println!(
                "RESPAWN tick {tick} cause {:?} pos ({:.1}, {:.1}, {:.1}) index {}",
                race.last_respawn_cause_of(1),
                position.x,
                position.y,
                position.z,
                ship.driver.index
            );
        }
        if tick < 400 && tick % 50 == 0 {
            let p = race.sim.world.ships[1].physics.body.position;
            let (i, s, _) = race.spline().nearest(p).unwrap();
            let lat = (p - Vec3::from_array(s.pos)).dot(Vec3::from_array(s.lateral));
            println!(
                "TRACE tick {tick} index {i} lat {lat:.1} corridor [{:.1},{:.1}] ",
                s.ai_bound_left, s.ai_bound_right
            );
        }
    }
}

/// Every slot of every circuit's grid, both directions, is over collision and
/// inside the AI corridor, and is laid where `Race_ComputeGridLayout` puts it:
/// `GRID_COLUMN_OFFSET / 2` either side of the corridor midpoint, the even slots
/// (slot 8 is the authored node) on the node's side and the odd ones opposite.
///
/// **Found by** the falloff survey: the odd column was 30 units off the track on
/// `01_Track` and `17_Track`, whose node sits on the left of the midpoint, and
/// the player's own time-trial start (slot 1) was on it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_slot_starts_on_the_track() {
    let circuits = circuits_all();
    if circuits.is_empty() {
        return;
    }
    let half = 0.5 * oag_gameplay::GRID_COLUMN_OFFSET;
    let mut checked = 0;
    for circuit in circuits {
        let Some(race) = start(&circuit) else {
            continue;
        };
        let label = format!("{}{}", circuit.id, if circuit.reversed { "r" } else { "" });
        let slots = slots(&race);
        assert_eq!(slots.len(), 8, "{label} should field a full grid");
        // The node's side of the midpoint, read off slot 8.
        let side = slots
            .iter()
            .find(|s| s.slot == 8)
            .map(|s| (s.lateral - s.mid).signum())
            .expect("slot 8");
        for s in &slots {
            assert!(
                s.ground,
                "{label} slot {} has no collision under it",
                s.slot
            );
            assert!(
                s.lateral >= s.left && s.lateral <= s.right,
                "{label} slot {} is at {:+.2}, outside the corridor [{:+.2}, {:+.2}]",
                s.slot,
                s.lateral,
                s.left,
                s.right
            );
            let want = if s.slot.is_multiple_of(2) {
                side
            } else {
                -side
            } * half;
            assert!(
                (s.lateral - s.mid - want).abs() < 1.5,
                "{label} slot {} is {:+.2} from the midpoint, the original's rule says {want:+.1}",
                s.slot,
                s.lateral - s.mid
            );
            checked += 1;
        }
    }
    println!("{checked} slots checked");
}

/// Eight craft read out of the running original on Metropia (`18_Track`,
/// `02_Track` reversed) at the start of a single race, 2026-09-29: PPSSPP, the
/// racer table at `0x08b34420` (stride `0x370`, position at entry `+0x10`),
/// slot 1 first. `docs/ghidra/functions/psp-pulse-usa/grid.md`.
const METROPIA_REVERSED_GRID: [[f32; 3]; 8] = [
    [529.33, -12.92, 169.73],
    [510.08, -12.98, 190.08],
    [490.04, -13.05, 170.47],
    [470.72, -13.11, 190.84],
    [450.61, -13.18, 171.24],
    [431.44, -13.24, 191.62],
    [411.30, -13.31, 172.04],
    [392.11, -13.38, 192.44],
];

/// A reversed circuit's grid is laid out the same way as a forward one, node at
/// the back: the decompile of `Race_ComputeGridLayout` reads as though a
/// reversed circuit's node were the *front* slot, and this is the measurement
/// that says it is not (worst slot 1.13 units, all eight).
///
/// The bound is 1.5, **chosen, not measured**: the worst residual is the anchor's
/// own along-track quantisation to the resampled spline.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn metropia_reversed_is_the_originals_grid() {
    let Some(circuit) = circuits_all().into_iter().find(|c| c.id == "18_Track") else {
        return;
    };
    let Some(race) = start(&circuit) else { return };
    let mut worst = 0.0f32;
    for (index, ship) in race
        .sim
        .world
        .ships
        .iter()
        .enumerate()
        .filter(|(_, s)| s.active)
    {
        let slot = if index == 0 { 8 } else { index };
        let want = Vec3::from_array(METROPIA_REVERSED_GRID[slot - 1]);
        let error = (ship.physics.body.position - want).length();
        println!("slot {slot}: {error:.3} apart");
        worst = worst.max(error);
    }
    assert!(worst < 1.5, "worst slot is {worst:.3} from the original's");
}
