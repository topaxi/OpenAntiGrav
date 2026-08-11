//! What a `Weapon Pad` hands out, and what a craft does with it.
//!
//! The pad's *trigger* is recovered and lives in the composition root beside the
//! speed pad's, because it needs the track's volumes. What is here is the two
//! halves that are gameplay rather than geometry: **which** weapon a crossing
//! grants, and the inventory that holds it.
//!
//! # What is recovered and what is ours
//!
//! Stated at the top because this module is unusually mixed, and
//! `docs/gameplay/pickups.md` carries the same split with its evidence.
//!
//! **Recovered.** The odds themselves - `<Pickupodds class="...">` weights every
//! weapon per speed class and separately for `ai`, `human`, `front` and `back`
//! of the grid, at confidence 92 (`docs/formats/weapon-stats.md`). That table
//! *is* the pickup design, and this module spends it rather than inventing a
//! distribution.
//!
//! **Ours, and there is no way for it not to be.**
//!
//! - **The draw.** How the original turns four weights into one weapon is
//!   unread, and its PRNG is an open question on the roadmap
//!   (`docs/overview/roadmap.md`), so the *sequence* this produces cannot match
//!   the original's even in principle. Only the *distribution* can be checked,
//!   which is what [`tests`] does. A weighted walk over the authored weights is
//!   the obvious reading of a table of weights; it is not a recovered algorithm.
//! - **The grant.** `WeaponPads_TestCraft` (`0x0888727c`) stamps the pad's
//!   refresh timer on a hit and **no pickup-grant call site has been found** -
//!   see `docs/ghidra/functions/psp-pulse-usa/pads.md`. So that a crossing
//!   grants anything at all is this project's reading of what a weapon pad is
//!   for, not a ported branch.
//! - **The inventory.** The original keeps it in a flag word at
//!   `*(entity+0x4c) + 0x1b8` whose bits are unread bar one. [`Held`] is a
//!   single slot instead.
//!
//! # Only what has an effect is handed out
//!
//! [`IMPLEMENTED`] is the pool a pad draws from: Turbo, Shield and Rocket.
//!
//! Of the ten still out, nine need a lock, a beam, a mechanic nothing has read
//! or a second flight model - Missile needs the lock distances its `<Stats>`
//! authors, Quake needs track deformation, LeachBeam needs a beam and a victim,
//! and most of the rest need the slowdown mechanic behind
//! `<Global slowdown_limit>`. Autopilot is the AI's own controller taking over
//! (`Ai_Construct` names the local player's input source the literal
//! `"autopilot input"`), so it is AI work rather than pickup work.
//!
//! **What Shield and Rocket do is ours**, more so than the Turbo's effect was:
//! the Turbo at least has a recovered magnitude in `<Engine turbo>`, while
//! Shield's `time` joins to no recovered code path at all and no
//! projectile-flight call site has been found anywhere. The durations, speeds,
//! radii and damage are the disc's; what they drive is this project's reading.
//! See [`oag_physics::ShipState::shield_pickup_timer`], [`crate::projectile`]
//! and `docs/gameplay/pickups.md`, which carries the split.
//!
//! **A rocket has almost nothing to hit today**, and that is a property of the
//! race rather than of the weapon: `oag_race::Mode::has_opponents` is
//! unconditionally `false`, so a race fields one craft. Track geometry and the
//! parked grid the `--opponents` verification flag spawns are what a rocket can
//! reach until the AI lands.
//!
//! **This is a departure and it is deliberate**: the authored table weights
//! thirteen weapons and this draws from a subset, so the distribution a player
//! sees is the authored one *conditioned on* the implemented set. It narrows to
//! nothing as weapons land - adding one to [`IMPLEMENTED`] is the whole change -
//! and it is preferred to handing out a mine that cannot be dropped.

use oag_core::Rng;
use oag_formats::weapons::{PickupTable, Weapon, WeaponStats};

/// The weapons a pad in this engine can hand out.
///
/// See the module docs: the shipped table weights thirteen and this is the
/// subset with an effect. A slice rather than a fixed-size array precisely
/// because it is expected to grow, which is the opposite of
/// [`oag_race::Mode::ALL`]'s reason for being one.
pub const IMPLEMENTED: &[Weapon] = &[Weapon::Turbo, Weapon::Shield, Weapon::Rocket];

/// Which column of `<Pickupodds>` a craft draws from.
///
/// The shipped table carries four - `ai`, `human`, `front` and `back` - and this
/// enum offers two. **The missing two are grid position**, which needs race
/// positions, which needs opponents that move: see
/// `oag_race::Mode::has_opponents`. Once an AI exists the front/back weights
/// become reachable and this type is what grows a third case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Driver {
    /// The player.
    #[default]
    Human,
    /// An AI-driven craft. Live since 2026-08-11: `oag_game::race` draws from
    /// this column for every slot but the player's. See `docs/gameplay/ai.md`.
    Ai,
}

impl Driver {
    /// This driver's weight for one weapon, from one class's table.
    ///
    /// Zero for a weapon the class authors no odds for, which is not an error:
    /// a table need not weight every weapon.
    #[must_use]
    fn weight(self, table: &PickupTable, weapon: Weapon) -> f32 {
        let Some(odds) = table.get(weapon) else {
            return 0.0;
        };
        match self {
            Self::Human => odds.human,
            Self::Ai => odds.ai,
        }
    }
}

/// What a craft is carrying.
///
/// One slot. The original's inventory is a flag word with room for more, and
/// whether it can hold two at once is unread - `SubWeapon` exists as a HUD
/// widget, which suggests it can, and no code has been read that fills it. One
/// slot is the conservative reading and the one the HUD can draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Held {
    /// The weapon in the slot, or `None`.
    pub weapon: Option<Weapon>,
}

impl Held {
    /// Nothing held.
    #[must_use]
    pub const fn empty() -> Self {
        Self { weapon: None }
    }

    /// Whether the slot can take a pickup.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.weapon.is_none()
    }

    /// Takes what is held, leaving the slot empty.
    pub const fn take(&mut self) -> Option<Weapon> {
        self.weapon.take()
    }
}

/// Draws one weapon from a class's authored odds, restricted to [`IMPLEMENTED`].
///
/// `None` when the table weights none of the implemented weapons above zero,
/// which is a real state rather than a failure: a class that authored no Turbo
/// odds would hand out nothing until a second weapon lands.
///
/// The walk is over [`IMPLEMENTED`]'s order rather than the table's document
/// order, so the sequence depends only on this crate and the seed - a table that
/// reordered its `<Weapon>` elements would otherwise change every draw.
#[must_use]
pub fn draw(rng: &mut Rng, table: &PickupTable, driver: Driver) -> Option<Weapon> {
    let total: f32 = IMPLEMENTED
        .iter()
        .map(|&weapon| driver.weight(table, weapon).max(0.0))
        .sum();
    if total <= 0.0 || !total.is_finite() {
        return None;
    }

    let mut roll = rng.next_f32() * total;
    for &weapon in IMPLEMENTED {
        let weight = driver.weight(table, weapon).max(0.0);
        if weight <= 0.0 {
            continue;
        }
        roll -= weight;
        if roll < 0.0 {
            return Some(weapon);
        }
    }
    // Only reachable when `next_f32` returns something that rounds the walk past
    // the end - it is half-open on `1.0`, so this is float slack rather than a
    // logic hole. The last weighted weapon is the right answer either way.
    IMPLEMENTED
        .iter()
        .rev()
        .copied()
        .find(|&weapon| driver.weight(table, weapon) > 0.0)
}

/// The class's own table out of a whole weapon file.
///
/// # The two files spell a speed class differently, and this is where that lands
///
/// `HandlingStats.xml` authors `<GlobalClass name="VENOM">` and
/// `WeaponStats_Race.xml` authors `<Pickupodds class="Venom">` - measured on the
/// shipped USA disc, all four classes, both files. So a caller holding a
/// [`SpeedClass`] cannot reach the pickup table through
/// [`WeaponStats::pickups_for`] and [`SpeedClass::as_str`] together, because the
/// latter is the *handling* file's spelling.
///
/// Matched case-insensitively here rather than by adding a second spelling to
/// [`SpeedClass`]: the difference is between two documents, not between two
/// concepts, and the parser deliberately keeps `PickupTable::class` a `String`
/// because it knows a fifth name the data never uses.
///
/// [`SpeedClass`]: oag_formats::handling::SpeedClass
/// [`SpeedClass::as_str`]: oag_formats::handling::SpeedClass::as_str
#[must_use]
pub fn table_for(
    stats: &WeaponStats,
    class: oag_formats::handling::SpeedClass,
) -> Option<&PickupTable> {
    stats
        .pickups
        .iter()
        .find(|table| table.class.eq_ignore_ascii_case(class.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_formats::weapons::PickupOdds;

    fn table(odds: &[(Weapon, f32, f32)]) -> PickupTable {
        PickupTable {
            class: "VENOM".to_string(),
            odds: odds
                .iter()
                .map(|&(weapon, human, ai)| {
                    (
                        weapon,
                        PickupOdds {
                            human,
                            ai,
                            front: 0.0,
                            back: 0.0,
                        },
                    )
                })
                .collect(),
        }
    }

    /// The stand-in for "weighted, and not implemented". **It has to be a weapon
    /// that stays out of [`IMPLEMENTED`]**, or the tests below invert silently
    /// the day it lands - which is exactly what happened to the `Rocket` these
    /// two used before. Quake needs track deformation and is a long way off.
    const UNIMPLEMENTED: Weapon = Weapon::Quake;

    #[test]
    fn a_class_that_weights_nothing_implemented_hands_out_nothing() {
        // A quake is weighted and a quake cannot be handed out, so this is the
        // real shape of the restriction rather than an empty table.
        let quakes_only = table(&[(UNIMPLEMENTED, 1.0, 1.0)]);
        let mut rng = Rng::new(1);
        assert_eq!(draw(&mut rng, &quakes_only, Driver::Human), None);
    }

    #[test]
    fn a_zero_weight_is_never_drawn() {
        let no_turbo = table(&[(Weapon::Turbo, 0.0, 1.0)]);
        let mut rng = Rng::new(1);
        for _ in 0..100 {
            assert_eq!(draw(&mut rng, &no_turbo, Driver::Human), None);
        }
    }

    /// The draw reads the *driver's own* column. With Turbo weighted for an AI
    /// and not for a human, the two answers have to differ - a reader that took
    /// whichever column came first would pass every other test here.
    #[test]
    fn the_two_drivers_read_their_own_columns() {
        let ai_only = table(&[(Weapon::Turbo, 0.0, 1.0)]);
        let mut rng = Rng::new(7);
        assert_eq!(draw(&mut rng, &ai_only, Driver::Ai), Some(Weapon::Turbo));
        assert_eq!(draw(&mut rng, &ai_only, Driver::Human), None);
    }

    /// **The only thing about the draw that can be checked against the
    /// original's data**, and it is a distribution rather than a sequence - see
    /// the module docs.
    ///
    /// Two implemented weapons weighted 3:1 against each other, plus an
    /// unimplemented one weighted far above both. The second half is the part
    /// that catches a real mistake: a walk that summed the *table's* total
    /// rather than the implemented subset's would spend most of its rolls past
    /// the end of the live weights and fall through to the trailing `find`,
    /// which returns the last implemented weapon every time. That reads as
    /// "mostly Shield" rather than as an error.
    #[test]
    fn the_walk_visits_a_weight_in_proportion_to_it() {
        let weighted = table(&[
            (Weapon::Turbo, 3.0, 3.0),
            (Weapon::Shield, 1.0, 1.0),
            (UNIMPLEMENTED, 96.0, 96.0),
        ]);
        let mut rng = Rng::new(99);
        let (mut turbos, mut shields) = (0, 0);
        for _ in 0..10_000 {
            match draw(&mut rng, &weighted, Driver::Human) {
                Some(Weapon::Turbo) => turbos += 1,
                Some(Weapon::Shield) => shields += 1,
                other => panic!("drew {other:?}, which is not implemented"),
            }
        }
        assert_eq!(turbos + shields, 10_000, "every draw must land somewhere");
        // 3:1 over 10,000 draws puts Turbo at 7,500 with a standard deviation of
        // about 43. A 500-wide window is roughly 11 sigma - wide enough that no
        // seed makes this flaky, narrow enough to reject an even split.
        assert!(
            (7_000..=8_000).contains(&turbos),
            "3:1 odds drew {turbos} turbos and {shields} shields"
        );
    }

    /// The property the determinism gate needs from this: same seed, same
    /// sequence. It is the one guarantee that survives the original's PRNG being
    /// unrecovered.
    #[test]
    fn the_same_seed_draws_the_same_sequence() {
        let weighted = table(&[(Weapon::Turbo, 1.0, 1.0)]);
        let sequence = |seed| {
            let mut rng = Rng::new(seed);
            (0..50)
                .map(|_| draw(&mut rng, &weighted, Driver::Human))
                .collect::<Vec<_>>()
        };
        assert_eq!(sequence(4), sequence(4));
    }

    #[test]
    fn a_held_slot_gives_up_what_it_holds_exactly_once() {
        let mut held = Held::empty();
        assert!(held.is_empty());
        held.weapon = Some(Weapon::Turbo);
        assert!(!held.is_empty());
        assert_eq!(held.take(), Some(Weapon::Turbo));
        assert_eq!(held.take(), None);
    }
}
