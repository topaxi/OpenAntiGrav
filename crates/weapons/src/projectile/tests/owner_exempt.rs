//! The owner's exemption from its own Mine or Bomb is a half-second window,
//! `Bomb_UpdateTrigger` / `Mine_SweepCraftTrigger`; it used to be permanent.

use super::*;
use crate::projectile::mine::{NO_FUSE, OWNER_EXEMPT_SECONDS};

const DT: f32 = 1.0 / 60.0;

/// Ticks until a charge laid on the craft that laid it goes off, or `None`
/// if it never does inside `limit` ticks. `ships` holds the owner at slot 0.
fn ticks_to_trip(kind: Weapon, owner_at_slot: u8, limit: usize) -> Option<(usize, Impact)> {
    let mut projectiles = Projectiles::new();
    assert!(projectiles.lay(kind, Vec3::ZERO, owner_at_slot, NO_FUSE, Quat::IDENTITY));
    let world = empty_world();
    let radii = TriggerRadii {
        mine: Some(5.0),
        bomb: Some(5.0),
    };
    // One craft, standing on the charge.
    let field = ships(&[(true, Vec3::ZERO)]);
    for tick in 1..=limit {
        let impacts = projectiles.advance(DT, &world, &field, None, None, None, radii, "VENOM");
        if let Some(impact) = impacts.iter().flatten().next() {
            return Some((tick, *impact));
        }
    }
    None
}

#[test]
fn the_layer_trips_its_own_bomb_once_the_window_is_over() {
    let (tick, impact) = ticks_to_trip(Weapon::Bomb, 0, 120).expect("the owner trips its own bomb");
    let seconds = tick as f32 * DT;
    assert!(
        (OWNER_EXEMPT_SECONDS..OWNER_EXEMPT_SECONDS + 2.0 * DT).contains(&seconds),
        "tripped at {seconds} s, not at the 0.5 s window's end"
    );
    assert_eq!(impact.struck, Some(0));
    assert!(impact.blast);
}

#[test]
fn the_layer_trips_its_own_mine_once_the_window_is_over() {
    let (tick, impact) = ticks_to_trip(Weapon::Mine, 0, 120).expect("the owner trips its own mine");
    let seconds = tick as f32 * DT;
    assert!(
        (OWNER_EXEMPT_SECONDS..OWNER_EXEMPT_SECONDS + 2.0 * DT).contains(&seconds),
        "tripped at {seconds} s, not at the 0.5 s window's end"
    );
    assert_eq!(impact.struck, Some(0));
}

#[test]
fn another_craft_trips_it_on_the_first_tick() {
    for kind in [Weapon::Mine, Weapon::Bomb] {
        let (tick, impact) = ticks_to_trip(kind, 1, 120).expect("a stranger trips it");
        assert_eq!(tick, 1, "{kind:?}: exemption must not shield other craft");
        assert_eq!(impact.struck, Some(0));
    }
}

#[test]
fn the_exemption_is_a_pure_function_of_age() {
    let at = Vec3::ZERO;
    assert!(!mine::triggered_by(at, 0, 0.49, 0, at, 5.0));
    assert!(mine::triggered_by(at, 0, 0.5, 0, at, 5.0));
    assert!(mine::triggered_by(at, 0, 0.0, 1, at, 5.0));
}
