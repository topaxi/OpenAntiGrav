//! A Rocket volley on Talon's Junction, flown from the pose Pulse PSP's own was
//! measured at, against the numbers measured there.
//!
//! **`#[ignore]`d and never run in CI**: it needs `data/images/pulse-psp-usa.chd`.
//!
//! The original's volley (Venom, Assegai, Time Trial on `16_Track`, fired 120
//! frames after GO) was read off `Rocket_Update` on PPSSPP, see
//! `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`'s 2026-10-01 section:
//! spawned at the craft's own position `(124.5, -47.9, -197.0)`, heading
//! `+X` with a 0.036 climb, cruising at 222.22 units/s, ending at 51, 61 and 70
//! frames and 185, 221 and 252 units from the spawn (centre, then the two
//! fanned shots, in the pool's own order).
//!
//! **What matches and what does not**, measured 2026-10-01. Ours detonates at
//! ticks 49/60/69 and 177/218/251 units for the left, centre and right shot
//! against the original's frames 51/61/70 and 185/221/252, and the centre shot's
//! last position is within about four units of the original's. The original's
//! rocket flies the first 2-3 updates on its fall arm (`Collision_SweepSegment`
//! returned `0x7f` with the floor four units below, cause unrecovered), a slow
//! 166.67 units/s phase ours does not reproduce: ours finds the floor on the
//! first update. That is why the tolerances are three ticks and twelve units.
//!
//! Our own start lands the craft at z -199.4 heading 1.7 degrees toward -Z
//! rather than that pose, so a volley from *our* craft cannot be compared with
//! the original's range: the road curves away from the line it flies. This
//! places the craft at the original's pose and fires through
//! [`oag_gameplay::projectile::fire_rocket`], the call both the player and an
//! opponent make.

use oag_core::math::{Mat3, Quat, Vec3};
use oag_game::race;
use oag_gameplay::projectile;

const TRACK: &str = "Data\\Environments\\16_Track\\track.vex";

/// The craft's own position at the fire, measured (`rocket-visuals.md`).
/// The class speed, 800 km/h over 3.6.
const CRUISE: f32 = 800.0 / 3.6;

const ORIGINAL_SPAWN: Vec3 = Vec3::new(124.48, -47.91, -196.95);

fn start() -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(TRACK.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(race::Race::start(loaded.setup))
}

/// A body whose forward is `forward` and whose up is as close to `up` as it
/// can be.
fn pose(forward: Vec3, up: Vec3) -> Quat {
    let forward = forward.normalize();
    let right = forward.cross(up).normalize();
    let up = right.cross(forward);
    Quat::from_mat3(&Mat3::from_cols(right, up, -forward))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_volley_from_the_originals_pose_is_laid_at_the_craft_and_cruises_at_the_class_speed() {
    let Some(mut race) = start() else {
        return;
    };
    let dt = race.dt();
    {
        let body = &mut race.sim.world.ships[0].physics.body;
        body.position = ORIGINAL_SPAWN;
        body.orientation = pose(Vec3::new(2.74, 0.10, -0.02), Vec3::new(-0.03, 1.0, 0.05));
        body.linear_velocity = Vec3::ZERO;
    }
    let stats = race.rocket_stats().expect("the disc authors a Rocket");
    let class = "VENOM".to_string();
    // A copy of the world to step, so the race's collision soup can be borrowed
    // beside it.
    let mut world = race.sim.world.clone();
    let fired = projectile::fire_rocket(
        &mut world.projectiles,
        &world.ships[0].physics,
        &stats,
        &class,
        0,
    )
    .expect("Venom is authored");
    assert_eq!(fired, projectile::ROCKET_SHOTS);
    for slot in 0..projectile::ROCKET_SHOTS {
        assert_eq!(
            world.projectiles.slots[slot].position, ORIGINAL_SPAWN,
            "a rocket is laid at the craft's own position"
        );
    }

    let mut ended: [Option<(u32, f32)>; projectile::ROCKET_SHOTS] = [None; 3];
    for tick in 1..=200u32 {
        let before: Vec<Vec3> = (0..3)
            .map(|s| world.projectiles.slots[s].position)
            .collect();
        let impacts = projectile::step(
            &mut world,
            dt,
            race.collision(),
            None,
            &class,
            oag_physics::DamageRules::default(),
            &mut [],
        );
        for slot in 0..3 {
            let p = world.projectiles.slots[slot];
            if p.kind.is_some() {
                assert!(
                    (p.velocity.length() - CRUISE).abs() < 0.05,
                    "slot {slot} tick {tick}: speed {} is not the class speed {CRUISE}",
                    p.velocity.length()
                );
            } else if ended[slot].is_none() && impacts[slot].is_some() {
                ended[slot] = Some((tick, (before[slot] - ORIGINAL_SPAWN).length()));
            }
        }
    }
    println!("ended (tick, units from the spawn) by slot: {ended:?}");
    // The original's, by slot (our slot 1 is its -Z-side shot, slot 0 the
    // centre, slot 2 the +Z-side shot): detonation frame and range.
    let original = [(61u32, 221.0f32), (51, 185.0), (70, 252.0)];
    for (slot, (frame, range)) in original.into_iter().enumerate() {
        let (tick, units) = ended[slot].unwrap_or_else(|| panic!("slot {slot} never detonated"));
        assert!(
            tick.abs_diff(frame) <= 3,
            "slot {slot} detonated at tick {tick}, the original at frame {frame}"
        );
        assert!(
            (units - range).abs() <= 12.0,
            "slot {slot} detonated {units} units out, the original {range}"
        );
    }
}
