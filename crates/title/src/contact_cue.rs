//! How a title decides a craft's contact is heard.
//!
//! Pulse re-arms one `.COLLISIONS` per craft by time. Wipeout HD/Fury starts its collision and
//! grind cues from the size of the frame's largest contact impulse, with a floor, a re-arm and a
//! volume that all follow from it
//! (`docs/ghidra/functions/ps3-hdfury-eu/collision-cues.md`, "`Ship_UpdateCollisionCues`").

use crate::pre_race::Sourced;

/// How a title decides a craft's contact is heard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContactCues {
    /// One `.COLLISIONS` per craft on a wall impact or a hit spark, re-armed by time
    /// (`oag_gameplay::cue::COLLISION_COOLDOWN`): Pulse's law.
    Rearmed,
    /// The frame's largest contact impulse against floors and steps.
    Impulse(Sourced<ImpulseLaw>),
}

/// The numbers of the impulse law: when a contact is heard, how loud, which size, and how soon
/// it may be heard again.
///
/// A hit is `.COLLISIONS` (or `.COLLSHIELD`) and every craft plays it; a grind is `.GRIND` (or
/// `.GRINDSHIELD`) and only a craft flown by a person plays it. Both read the same impulse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpulseLaw {
    /// A contact is nose-on when `|dot(craft forward, contact normal)|` is over this. A grind is
    /// never nose-on and a hit under [`Self::hit_floor_side`] needs it.
    pub nose_on_dot: f32,
    /// A hit is heard only over this impulse.
    pub hit_floor: f32,
    /// The floor for a hit that is not nose-on.
    pub hit_floor_side: f32,
    /// A hit over this impulse is at least medium.
    pub hit_medium: f32,
    /// A hit over this impulse is large.
    pub hit_large: f32,
    /// Ticks after a hit before the next one may start.
    pub hit_wait: u8,
    /// A hit's volume grows by this per unit of impulse.
    pub hit_volume_per_impulse: f32,
    /// A hit's volume at no impulse.
    pub hit_volume_base: f32,
    /// A grind is heard only over this impulse.
    pub grind_floor: f32,
    /// A grind over this impulse is at least medium.
    pub grind_medium: f32,
    /// A grind over this impulse is large.
    pub grind_large: f32,
    /// Ticks after a grind before the next one may start.
    pub grind_wait: u8,
    /// A grind's volume grows by this per unit of impulse.
    pub grind_volume_per_impulse: f32,
    /// A grind's volume at no impulse.
    pub grind_volume_base: f32,
}
