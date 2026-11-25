//! On Pulse's steepest bank a Rocket and a Plasma bolt ride the floor to the collision cage, update
//! for update as the original's do, under Pulse's floor arms (`FloorArms::SteerToRidePoint`), and
//! not under the parallel turn HD's arms take.
//!
//! **`#[ignore]`d and never run in CI**: needs `data/images/pulse-psp-usa.chd`.
//!
//! The rows are the original's own entity at the entry of every update, recorded from PPSSPP on
//! Moa Therma White (`03`), whose helix is the largest bank of any Pulse circuit (85.8 degrees at
//! spline sample 996; 80 to 86 degrees from sample 600 to 1500). One Plasma (fired at sample 760,
//! released 60 frames later) and one Rocket volley (fired at sample 845), each from the original's Autopilot-flown craft
//! (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`, "Pulse on the steepest bank").
//! Each flight is seeded from the original's state at update 1 and chained on the original's own
//! `dt`, so the firing pose is the original's.

#![allow(
    clippy::excessive_precision,
    clippy::unreadable_literal,
    reason = "the recorded f32 values, written so they round-trip"
)]

use oag_core::math::Vec3;
use oag_physics::cage::Containment;
use oag_raceplay as race;
use oag_tables::weapons::Weapon;
use oag_weapons::projectile::{
    BlastPush, FloorArms, Impact, MAX_FLIGHT_SECONDS, Projectile, WeaponHit, step,
};

/// `dt`, age, position, velocity and carried normal at the entry of one update.
type Row = (f32, f32, [f32; 3], [f32; 3], [f32; 3]);

include!("projectile_bank_ground_truth/rows.rs");

fn v3(v: [f32; 3]) -> Vec3 {
    Vec3::new(v[0], v[1], v[2])
}

fn start() -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(r"Data\Environments\03_Track\track.vex".to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(race::Race::start(loaded.setup, loaded.view, loaded.modules))
}

/// The worst distance from the original's entry position over the chained flight, the update the
/// flight ended on (1-based, counting the one that ended it) and its impact.
struct Chained {
    worst: f32,
    worst_update: usize,
    ended: Option<(usize, Impact)>,
}

fn chain(
    race: &race::Race,
    kind: Weapon,
    rows: &[Row],
    launch_speed_kmh: f32,
    arms: FloorArms,
) -> Chained {
    let cage: &dyn Containment = race
        .core
        .sim
        .spline()
        .cage()
        .expect("Pulse runs the cage on 03");
    let (_, age, position, velocity, surface) = rows[0];
    let mut world = race.core.sim.world.clone();
    world.projectiles.clear();
    world.projectiles.slots[0] = Projectile {
        kind: Some(kind),
        position: v3(position),
        velocity: v3(velocity),
        surface: v3(surface),
        owner: 0,
        lifetime: MAX_FLIGHT_SECONDS - age,
        launch_speed_kmh,
        ..Projectile::default()
    };
    let count = world.ship_count as usize;
    let mut out = Chained {
        worst: 0.0,
        worst_update: 0,
        ended: None,
    };
    for update in 1..=rows.len() + 20 {
        let dt = rows[(update - 1).min(rows.len() - 1)].0;
        let mut hits = vec![WeaponHit::default(); world.ships.len()];
        let impacts = step(
            &mut world.projectiles,
            &mut world.ships[..count],
            dt,
            race.core.sim.collision(),
            Some(cage),
            race.core.sim.weapon_stats(),
            "VENOM",
            arms,
            race.core.sim.own_craft_probe(),
            race.core.sim.plasma_release(),
            BlastPush::Posted,
            oag_physics::DamageRules::default(),
            &mut hits,
        );
        if let Some(impact) = impacts[0] {
            out.ended = Some((update, impact));
            return out;
        }
        if let Some(next) = rows.get(update) {
            let error = (world.projectiles.slots[0].position - v3(next.2)).length();
            if error > out.worst {
                out.worst = error;
                out.worst_update = update + 1;
            }
        }
    }
    out
}

/// The flight matches the original's update for update and ends on its update, at its probe
/// point.
fn matches(c: &Chained, tolerance: f32, calls: usize, end: [f32; 3]) {
    assert!(
        c.worst < tolerance,
        "{} units off the original at update {}",
        c.worst,
        c.worst_update
    );
    let (update, impact) = c.ended.expect("the flight ends");
    assert_eq!(
        update, calls,
        "ended on update {update}, the original on {calls}"
    );
    let off = (impact.point - v3(end)).length();
    assert!(
        off < 0.5,
        "ends at {:?}, {off} from the original's {end:?}",
        impact.point
    );
}

/// The Plasma rides the mag floor (probe code `3`) from its first update and the original's probe
/// returns `4`, outside the cage, at update 47.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_plasma_rides_the_helix_to_the_cage() {
    let Some(race) = start() else { return };
    let arms = race.core.sim.floor_arms();
    assert_eq!(arms, FloorArms::SteerToRidePoint);
    let flight = chain(&race, Weapon::Plasma, &PLASMA, PLASMA_LAUNCH_KMH, arms);
    matches(&flight, 0.5, 47, [337.065796, 128.953918, -570.552124]);
}

/// The three rockets of one volley ride the helix for 58, 69 and 49 updates and end at the cage.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_rockets_ride_the_helix_to_the_cage() {
    let Some(race) = start() else { return };
    let arms = race.core.sim.floor_arms();
    let ends = [
        (&ROCKET_0[..], 58, [329.985352, 134.227844, -580.010132]),
        (&ROCKET_1[..], 69, [304.260071, 134.886993, -615.15387]),
        (&ROCKET_2[..], 49, [343.052612, 132.4911499, -547.001343]),
    ];
    for (rows, calls, end) in ends {
        let flight = chain(&race, Weapon::Rocket, rows, ROCKET_KMH, arms);
        matches(&flight, 0.01, calls, end);
    }
}
