//! On Pulse's steepest bank a Rocket and a Plasma bolt ride the floor to the collision cage, update
//! for update as the original's do, under Pulse's floor arms (`FloorArms::SteerToRidePoint`), and
//! not under the parallel turn HD's arms take.
//!
//! **`#[ignore]`d and never run in CI**: needs `data/images/pulse-psp-usa.chd` and
//! `data/images/pure-psp-usa.chd`.
//!
//! The rows are the original's own entity at the entry of every update, recorded from PPSSPP on
//! Moa Therma White (`03`), whose helix is the largest bank of any Pulse circuit (85.8 degrees at
//! spline sample 996; 80 to 86 degrees from sample 600 to 1500). One Plasma (fired at sample 760,
//! released 60 frames later) and one Rocket volley (fired at sample 845), each from the original's Autopilot-flown craft
//! (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`, "Pulse on the steepest bank").
//! Each flight is seeded from the original's state at update 1 and chained on the original's own
//! `dt`, so the firing pose is the original's.
//!
//! Pure's Rocket and Plasma ride another law, the normal they were launched with
//! (`FloorArms::LaunchNormal`, `docs/ghidra/functions/psp-pure-usa/floor-follow.md`): five flights
//! recorded the same way on Vineta K and on Sebenco Climb (`pure-psp-usa.chd`), whose floor there
//! is 15 degrees off the launch normal.

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
include!("projectile_bank_ground_truth/pure_rows.rs");

fn v3(v: [f32; 3]) -> Vec3 {
    Vec3::new(v[0], v[1], v[2])
}

fn start() -> Option<race::Race> {
    start_on(
        "data/images/pulse-psp-usa.chd",
        r"Data\Environments\03_Track\track.vex",
        "VENOM",
    )
}

fn start_on(image: &str, track: &str, class: &str) -> Option<race::Race> {
    let image = oag_testdata::image(image)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(track.to_string()),
        class: class.to_string(),
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
    class: &str,
    kind: Weapon,
    rows: &[Row],
    launch_speed_kmh: f32,
    arms: FloorArms,
) -> Chained {
    let cage = race
        .core
        .sim
        .spline()
        .cage()
        .map(|cage| cage as &dyn Containment);
    let (_, age, position, velocity, surface) = rows[0];
    let mut world = race.core.sim.world.clone();
    world.projectiles.clear();
    world.projectiles.slots[0] = Projectile {
        kind: Some(kind),
        position: v3(position),
        velocity: v3(velocity),
        // Pure's carried normal is stored at the craft's display scale; ours is a unit vector.
        surface: v3(surface).normalize(),
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
            cage,
            race.core.sim.weapon_stats(),
            class,
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
    let flight = chain(
        &race,
        "VENOM",
        Weapon::Plasma,
        &PLASMA,
        PLASMA_LAUNCH_KMH,
        arms,
    );
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
        let flight = chain(&race, "VENOM", Weapon::Rocket, rows, ROCKET_KMH, arms);
        matches(&flight, 0.01, calls, end);
    }
}

fn pure(track: &str) -> Option<race::Race> {
    start_on("data/images/pure-psp-usa.chd", track, "VECTOR")
}

/// The flight matches the original's update for update and ends, on a wall, on its last one.
fn matches_to_the_end(c: &Chained, tolerance: f32, calls: usize) {
    assert!(
        c.worst < tolerance,
        "{} units off the original at update {}",
        c.worst,
        c.worst_update
    );
    let (update, _) = c.ended.expect("the flight ends");
    assert_eq!(
        update, calls,
        "ended on update {update}, the original on {calls}"
    );
}

/// Pure's Vector speeds: the Rocket's class speed and the Plasma's, read live (277.8 and 250
/// units/s after the first ride).
const PURE_ROCKET_KMH: f32 = 1000.0;
const PURE_PLASMA_KMH: f32 = 900.0;

/// Pure's Rockets and Plasma on Vineta K's start straight, 30 updates a second: each rides 2.25
/// (4.5) along its launch normal and ends on a wall on the original's update. Pulse's arm is
/// 0.67 to 0.94 units off by the second update.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn pures_rockets_and_plasma_ride_their_launch_normal_on_vineta_k() {
    let Some(race) = pure(r"Data\Environments\01_Vineta_K\track.vex") else {
        return;
    };
    let arms = race.core.sim.floor_arms();
    assert_eq!(arms, FloorArms::LaunchNormal);
    for (kind, rows, kmh) in [
        (Weapon::Rocket, &PURE_VINETA_ROCKET_0[..], PURE_ROCKET_KMH),
        (Weapon::Rocket, &PURE_VINETA_ROCKET_1[..], PURE_ROCKET_KMH),
        (Weapon::Rocket, &PURE_VINETA_ROCKET_2[..], PURE_ROCKET_KMH),
        (Weapon::Plasma, &PURE_VINETA_PLASMA[..], PURE_PLASMA_KMH),
    ] {
        let flight = chain(&race, "VECTOR", kind, rows, kmh, arms);
        matches_to_the_end(&flight, 0.01, rows.len());
    }
}

/// Over Sebenco Climb's floor, 15 degrees off the Rocket's launch normal, it keeps that normal
/// for 25 updates as the original's does; adopting the floor's ends it on update 15.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn a_pure_rocket_keeps_its_launch_normal_over_sebenco_climbs_tilted_floor() {
    let Some(race) = pure(r"Data\Environments\10_Sebenco_Climb\track.vex") else {
        return;
    };
    let arms = race.core.sim.floor_arms();
    let flight = chain(
        &race,
        "VECTOR",
        Weapon::Rocket,
        &PURE_SEBENCO_ROCKET,
        PURE_ROCKET_KMH,
        arms,
    );
    matches_to_the_end(&flight, 0.01, PURE_SEBENCO_ROCKET.len());
}
