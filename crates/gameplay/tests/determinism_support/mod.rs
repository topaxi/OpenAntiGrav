//! What the two determinism scenarios share: the timestep, the corridor
//! and the invented weapon table. A `mod`, not a crate, so it stays a
//! dev-only fixture; `determinism.rs` and `determinism_volley.rs` both
//! include it.

use oag_physics::{CollisionWorld, Surface, TriangleSoup};

/// Our own fixed timestep. ADR-0007.
pub const TICK: f32 = 1.0 / 60.0;

/// A wall across the flight path, far enough that a rocket flies for a while
/// before reaching it.
pub fn corridor() -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-200.0, -200.0, 400.0],
            [-200.0, 200.0, 400.0],
            [200.0, 200.0, 400.0],
            [200.0, -200.0, 400.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Vec::new(),
        Surface::Wall,
        0,
    ));
    world
}

/// Invented numbers, per ADR-0006 - **not** any ship's or any weapon's.
/// The whole table, since `projectile::step` looks a blast up by weapon.
///
/// **It authors a Rocket and nothing else on purpose.** The scenario below flies
/// a Rocket, and the committed constants are what say the Rocket's flight has not
/// changed; adding a Missile block here would put a second weapon's numbers inside
/// the thing the reference is measuring. A Missile that never flies would move no
/// hash either, but the next person to add a projectile to this scenario should
/// have to think about it rather than find one already half-wired.
pub fn weapon_stats() -> oag_tables::weapons::WeaponStats {
    oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Rocket"><Stats absorb="1" blastforce="20" blastradius="30"
               damage="10" slowdown_time="1" venomspeed="500" flashspeed="600"
               rapierspeed="700" phantomspeed="800" launchSpeed="0" spread="1"/></Weapon>
           </WeaponStats>"#,
    )
    .expect("the fixture parses")
}
