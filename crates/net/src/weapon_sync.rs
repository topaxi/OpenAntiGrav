//! What a prediction gets wrong about weapons, and how a client notices.
//!
//! A craft's position is a continuous quantity the client tolerates being a little off
//! (ADR-0069). A weapon event is discrete: a missile exists or it does not, a pickup is held or
//! spent, a hit landed or it did not. The client's present predicts every other person on their
//! last held input, which carries no button edges, so **a remote person's fire is never
//! predicted**: it enters this client's present only when a correction replays the confirmed
//! tick. A correction triggered by craft position alone would wait for the missile to move
//! something by a unit, which it may never do. So the client also compares a [`digest`] of the
//! discrete weapon state, predicted against confirmed, at every confirmed tick.
//!
//! Chosen, not measured: [`SHIELD_STEP`], the coarseness at which a shield change counts as a
//! hit rather than as the small wear of contact.

use oag_gameplay::MAX_SHIPS;
use oag_raceplay::RaceSim;

/// A shield change smaller than this does not count as a hit. Chosen, not measured.
pub const SHIELD_STEP: f32 = 1.0;

/// A fingerprint of the discrete weapon state of `sim`, blind to where things are: the
/// projectiles in the air (kind, owner, target, as a set, so the slot a shot landed in does not
/// matter), the Quake, the LeachBeam, the Repulsers, each craft's held pickup and its shield in
/// whole steps of [`SHIELD_STEP`]. Not the hash: a position never enters it.
#[must_use]
pub fn digest(sim: &RaceSim) -> u64 {
    let world = &sim.world;
    let mut shots = 0_u64;
    for p in world.projectiles.slots.iter().filter(|p| p.kind.is_some()) {
        let mut h = oag_core::hash::StateHasher::new();
        h.write_u8(p.kind.map_or(0, |k| k as u8 + 1));
        h.write_u8(p.owner);
        h.write_u8(p.target.map_or(255, |t| t));
        shots = shots.wrapping_add(h.finish());
    }
    let mut h = oag_core::hash::StateHasher::new();
    h.write_u64(shots);
    h.write_u8(u8::from(world.quake.is_some()));
    if let Some(beam) = &world.leach_beam {
        h.write_u8(1 + beam.owner);
        h.write_u8(beam.target);
    }
    h.write_u8(u8::try_from(world.repulsers.iter().flatten().count()).unwrap_or(255));
    for ship in world.ships.iter().take(usize::from(world.ship_count).min(MAX_SHIPS)) {
        h.write_u8(ship.pickup.weapon.map_or(0, |w| w as u8 + 1));
        h.write_i32((ship.physics.shield / SHIELD_STEP).floor() as i32);
    }
    h.finish()
}

/// A shot in the air, as a pop-in measurement sees it: what and whose, and how old.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shot {
    /// The weapon, by its index.
    pub kind: u8,
    /// The firing craft's slot.
    pub owner: u8,
    /// Ticks in the air.
    pub age_ticks: u32,
}

/// Every projectile in the air in `sim`.
#[must_use]
pub fn shots(sim: &RaceSim) -> Vec<Shot> {
    sim.world
        .projectiles
        .slots
        .iter()
        .filter_map(|p| {
            Some(Shot {
                kind: p.kind? as u8,
                owner: p.owner,
                age_ticks: (p.age * 60.0).round().max(0.0) as u32,
            })
        })
        .collect()
}

/// What a correction changed about the weapons, between the present before it and after it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Change {
    /// Shots in the present after the correction that were not there before (a missile
    /// popping into view).
    pub appeared: u32,
    /// Shots in the present before that are not there after (a missile undone).
    pub vanished: u32,
    /// The greatest age, in ticks, of a shot that appeared: how far into its flight a missile
    /// is when the player first sees it.
    pub oldest_appeared: u32,
    /// Whether the client's own craft lost shield in the correction (a dodge undone).
    pub own_hit_added: bool,
    /// Whether the client's own craft gained shield back (a predicted hit undone).
    pub own_hit_removed: bool,
}

/// Compares two presents, `before` a correction and `after` it, for the craft in `seat`.
#[must_use]
pub fn change(before: &RaceSim, after: &RaceSim, seat: usize) -> Change {
    let had = shots(before);
    let has = shots(after);
    let mut out = Change::default();
    let unmatched = |from: &[Shot], other: &[Shot]| -> Vec<Shot> {
        let mut pool: Vec<Shot> = other.to_vec();
        from.iter()
            .filter(|s| {
                if let Some(i) = pool
                    .iter()
                    .position(|o| o.kind == s.kind && o.owner == s.owner)
                {
                    pool.swap_remove(i);
                    false
                } else {
                    true
                }
            })
            .copied()
            .collect()
    };
    let appeared = unmatched(&has, &had);
    out.appeared = u32::try_from(appeared.len()).unwrap_or(u32::MAX);
    out.oldest_appeared = appeared.iter().map(|s| s.age_ticks).max().unwrap_or(0);
    out.vanished = u32::try_from(unmatched(&had, &has).len()).unwrap_or(u32::MAX);
    let drop = before.world.ships[seat].physics.shield - after.world.ships[seat].physics.shield;
    out.own_hit_added = drop >= SHIELD_STEP;
    out.own_hit_removed = drop <= -SHIELD_STEP;
    out
}
