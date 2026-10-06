//! What a fired weapon *shows*: the flare gate, the bounce gate and the
//! Missile's own orbiting-anchor geometry - the pure functions
//! `crate::weapons::visuals` exposes for exactly this, testable with
//! no disc.
//!
//! Split out of `race/tests/weapons.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// Only the Rocket, the Missile, the Plasma and the Shuriken ride a flare, and
/// each rides its **own**: four separate authored files, not one generic
/// "something is in the air" marker.
///
/// **Regression test for a real bug, not a speculative one.** This gate used
/// to be `kind.is_none()`, true for *every* live projectile, which attached
/// the Rocket's looping flare to a Mine or a Bomb the instant it was laid -
/// a fire riding a charge from the moment it landed, reported from play
/// against real Pulse and impossible to reproduce headlessly because the
/// test harness's `psys::Library` carries no loaded effects (see
/// `Race::advance_projectile_flares`'s doc comment for why the gate itself,
/// rather than the attach, is what is tested here).
#[test]
fn each_projectile_rides_only_its_own_flare() {
    use oag_tables::weapons::Weapon;

    for weapon in Weapon::ALL {
        let expected = match weapon {
            Weapon::Rocket => Some(Trigger::RocketFlare),
            Weapon::Missile => Some(Trigger::MissileFlare),
            Weapon::Plasma => Some(Trigger::PlasmaFlare),
            Weapon::Shuriken => Some(Trigger::ShurikenFlare),
            _ => None,
        };
        assert_eq!(
            crate::weapons::flare_effect_for(Some(weapon)),
            expected,
            "{weapon:?} rides the wrong flare"
        );
    }
    assert_eq!(
        crate::weapons::flare_effect_for(None),
        None,
        "an empty slot rides nothing"
    );
}

/// Only a live Missile whose own counter just went up counts as a bounce -
/// not another kind sharing the same nonzero value, and not a Missile whose
/// counter fell (a fresh spawn landing in a slot the old one's `bounces`
/// snapshot was taken from).
#[test]
fn only_a_missiles_own_rising_counter_is_a_bounce() {
    use crate::weapons::bounced_this_tick;
    use oag_tables::weapons::Weapon;

    assert!(
        bounced_this_tick(Some(Weapon::Missile), 1, 2),
        "the counter rose on a live Missile"
    );
    assert!(
        !bounced_this_tick(Some(Weapon::Missile), 2, 2),
        "an unchanged counter is not a new bounce"
    );
    assert!(
        !bounced_this_tick(Some(Weapon::Missile), 3, 0),
        "a falling counter is a fresh spawn in the slot, never a bounce"
    );
    assert!(
        !bounced_this_tick(Some(Weapon::Rocket), 1, 2),
        "a Rocket never bounces, whatever a stray counter says"
    );
    assert!(
        !bounced_this_tick(None, 1, 2),
        "an empty slot cannot have bounced"
    );
}

/// At the instant of launch (`age == 0`), the primary anchor sits exactly on
/// the missile - `sin(0) == 0` and `vertical == 0` zero out every offset
/// term - and the second sits one `lateral` unit out along the missile's own
/// right vector. Both are read off `Missile_Init`'s two `Psys_Spawn_q` calls
/// via `Missile_Update`'s formula; see `missile_flare_anchors`'s doc comment.
#[test]
fn the_missiles_flare_anchors_coincide_with_it_at_the_instant_of_launch() {
    use crate::weapons::missile_flare_anchors;

    let position = Vec3::new(10.0, 2.0, -5.0);
    let velocity = Vec3::new(0.0, 0.0, -1.0);
    let (a, b) = missile_flare_anchors(position, velocity, 0.0);

    assert!(
        (a - position).length() < 1e-5,
        "the primary anchor must start exactly on the missile: {a:?}"
    );
    assert!(
        ((b - position).length() - 1.5).abs() < 1e-5,
        "the second anchor must start 1.5 units out along the right vector: {b:?}"
    );
}

/// The two anchors are always the same distance apart from `position` summed
/// together, whatever `θ` (the flight time) is - `sin(θ) + (1 - sin(θ))` and
/// `cos(θ) + (1 - cos(θ))` are both `1` for any `θ`. A rotation invariant
/// that holds without pinning a single trig value, so it exercises the whole
/// formula rather than one sampled angle.
#[test]
fn the_two_anchors_sum_to_a_theta_independent_point() {
    use crate::weapons::missile_flare_anchors;

    let position = Vec3::new(-4.0, 1.0, 8.0);
    let velocity = Vec3::new(3.0, 0.5, -2.0);
    let lateral = 1.5;

    // Recomputed the same way the function does, so this checks the
    // invariant against the function's own (unexported) geometry rather
    // than hardcoding a basis of the test's own.
    let back = (-velocity).try_normalize().unwrap();
    let up = (Vec3::Y - back * Vec3::Y.dot(back))
        .try_normalize()
        .unwrap();
    let right = up.cross(back);

    for age in [0.0_f32, 0.05, 0.3, 1.0, 2.7] {
        let (a, b) = missile_flare_anchors(position, velocity, age);
        let vertical = (age * 6.0).min(3.0);
        // a + b = 2*position + right*lateral + up*vertical, independent of θ.
        let sum = a + b;
        let expected = position * 2.0 + right * lateral + up * vertical;
        assert!(
            (sum - expected).length() < 1e-4,
            "age {age}: anchors summed to {sum:?}, expected {expected:?}"
        );
    }
}

/// A missile flying exactly parallel to world up must not produce a NaN
/// basis - the primary degenerate case `missile_flare_anchors` names in its
/// own doc comment.
#[test]
fn a_missile_flying_straight_up_gets_a_finite_basis() {
    use crate::weapons::missile_flare_anchors;

    let (a, b) = missile_flare_anchors(Vec3::ZERO, Vec3::Y, 0.4);
    assert!(a.is_finite(), "primary anchor went non-finite: {a:?}");
    assert!(b.is_finite(), "second anchor went non-finite: {b:?}");
}

/// Two weapons bounce and **each plays its own burst**, which is the same rule
/// the flares follow and is here for the same reason: reusing one weapon's
/// authored file for another is invention wearing a real asset.
#[test]
fn each_bouncing_weapon_plays_its_own_burst() {
    use crate::weapons::bounce_effect_for;
    use oag_tables::weapons::Weapon;

    for weapon in Weapon::ALL {
        let expected = match weapon {
            Weapon::Missile => Some(Trigger::MissileBounce),
            Weapon::Shuriken => Some(Trigger::ShurikenBounce),
            _ => None,
        };
        assert_eq!(
            bounce_effect_for(Some(weapon)),
            expected,
            "{weapon:?} plays the wrong bounce burst"
        );
    }
    assert_eq!(bounce_effect_for(None), None, "an empty slot plays nothing");
}

/// A rising counter on a weapon that cannot bounce is not a bounce.
///
/// The gate generalised from "is it a Missile" to "does this weapon bounce at
/// all" when the Shuriken landed, and the failure mode that generalisation
/// could introduce is a Rocket - whose counter is always zero - being read as
/// bouncing if the test ever became `now > before` alone.
#[test]
fn a_weapon_that_cannot_bounce_never_reads_as_bouncing() {
    use crate::weapons::bounced_this_tick;
    use oag_tables::weapons::Weapon;

    assert!(bounced_this_tick(Some(Weapon::Shuriken), 3, 4));
    assert!(!bounced_this_tick(Some(Weapon::Shuriken), 4, 4));
    assert!(!bounced_this_tick(Some(Weapon::Rocket), 0, 1));
    assert!(!bounced_this_tick(Some(Weapon::Plasma), 0, 1));
}

/// The riding flare's scale follows the charge for a Plasma alone -
/// `Plasma_UpdateCharge`'s `(1.0 - remaining) * 0.75`, ported in
/// `plasma_flare_scale` - and sits at the stage's neutral `1.0` for every
/// other weapon.
///
/// **A flying bolt is `0.75`, not `1.0`.** `Plasma_Launch`'s recovered body
/// (`plasma.md`) writes no severity field at all, so nothing resets the
/// value `Plasma_UpdateCharge` left behind when `charge` hit zero - see
/// `plasma_flare_scale`'s own doc comment for the screenshot evidence that
/// caught the earlier `1.0` guess as a visible, unexplained pop at release.
///
/// `cockpit` is `false` throughout this test - the external-view reading,
/// which is also every existing assertion's own expected value from before
/// the flag was ported, so this test still pins the un-halved shape.
/// [`the_plasma_flare_scale_halves_in_the_cockpit`] is the `cockpit: true` half.
#[test]
fn the_plasma_flare_scale_follows_the_charge() {
    use crate::weapons::plasma_flare_scale;
    use oag_tables::weapons::Weapon;
    use oag_weapons::projectile::plasma::CHARGE_SECONDS;

    let half_charged = plasma_flare_scale(Some(Weapon::Plasma), CHARGE_SECONDS / 2.0, false);
    assert!(
        (half_charged - 0.375).abs() < 1e-6,
        "half-charged plasma flare should scale to 0.375, got {half_charged}"
    );

    let flying = plasma_flare_scale(Some(Weapon::Plasma), 0.0, false);
    assert!(
        (flying - 0.75).abs() < 1e-6,
        "a launched bolt's flare freezes at the wind-up's own maximum, got {flying}"
    );

    let just_pressed = plasma_flare_scale(Some(Weapon::Plasma), CHARGE_SECONDS, false);
    assert_eq!(
        just_pressed, 0.0,
        "the instant of the press should scale the flare to nothing yet"
    );

    for weapon in [Weapon::Rocket, Weapon::Missile, Weapon::Shuriken] {
        assert_eq!(
            plasma_flare_scale(Some(weapon), CHARGE_SECONDS / 2.0, false),
            1.0,
            "{weapon:?} never ramps, whatever charge it is handed"
        );
    }
    assert_eq!(
        plasma_flare_scale(None, CHARGE_SECONDS / 2.0, false),
        1.0,
        "an empty slot rides at neutral scale"
    );
}

/// The cockpit-view half of `craft+0x6d`'s reading: `Plasma_UpdateCharge`
/// halves the glow again when the firing craft's own flag is set, and this
/// engine's `cockpit` parameter is the caller's `!Race::draws_own_ship()` -
/// see `plasma_flare_scale`'s own doc comment for the two independent
/// consumers (`camera.md`, `shield-pickup.md`) that put the reading at
/// confidence 82.
///
/// **Never for a non-Plasma or an empty slot**, the same as the ramp itself -
/// `cockpit: true` on either is still neutral `1.0`, not `0.5`, since the
/// `kind != Plasma` guard returns before `cockpit` is ever read.
#[test]
fn the_plasma_flare_scale_halves_in_the_cockpit() {
    use crate::weapons::plasma_flare_scale;
    use oag_tables::weapons::Weapon;
    use oag_weapons::projectile::plasma::CHARGE_SECONDS;

    let half_charged = plasma_flare_scale(Some(Weapon::Plasma), CHARGE_SECONDS / 2.0, true);
    assert!(
        (half_charged - 0.1875).abs() < 1e-6,
        "half-charged, in the cockpit, should scale to half of 0.375 = 0.1875, got {half_charged}"
    );

    let flying = plasma_flare_scale(Some(Weapon::Plasma), 0.0, true);
    assert!(
        (flying - 0.375).abs() < 1e-6,
        "a launched bolt's flare freezes at half the wind-up's own maximum, got {flying}"
    );

    assert_eq!(
        plasma_flare_scale(Some(Weapon::Rocket), CHARGE_SECONDS / 2.0, true),
        1.0,
        "a non-Plasma stays neutral even with cockpit set"
    );
    assert_eq!(
        plasma_flare_scale(None, CHARGE_SECONDS / 2.0, true),
        1.0,
        "an empty slot stays neutral even with cockpit set"
    );
}

/// Every weapon's detonation starts the flash `ScreenFlash_Start`'s caller
/// passes, and a Rocket that hit the track, a Cannon round and the Quake start
/// none from the impact path.
///
/// Drop a row from `flash_for` and the matching arm here fails: this is the
/// wiring's own guard, since the flash is render-only and nothing else in the
/// suite would notice one going quiet.
#[test]
fn each_detonation_starts_its_own_screen_flash() {
    use crate::weapons::flash_for;
    use oag_fx::flash;
    use oag_tables::weapons::Weapon;

    for weapon in Weapon::ALL {
        for struck in [false, true] {
            let expected = match (weapon, struck) {
                (Weapon::Rocket, true) => Some(flash::BLAST),
                (Weapon::Rocket, false) => None,
                (Weapon::Missile | Weapon::Shuriken, _) => Some(flash::BLAST),
                (Weapon::Plasma, _) => Some(flash::PLASMA),
                (Weapon::Bomb, _) => Some(flash::BOMB),
                (Weapon::Mine, _) => Some(flash::MINE),
                _ => None,
            };
            assert_eq!(
                flash_for(weapon, struck),
                expected,
                "{weapon:?} (struck: {struck}) starts the wrong flash"
            );
        }
    }
}
