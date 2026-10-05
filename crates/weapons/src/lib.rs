//! Weapons: what a craft holds, what it fires, what lands, and what that costs.
//!
//! The pickup a craft carries ([`pickup`]), the projectiles in flight and the
//! blasts, beams, waves and rings they raise ([`projectile`]), the disruption a
//! bolt leaves on its victim ([`disruption`]) and the slowdown a blast credits
//! ([`slowdown`]). Simulation code, bound by the same determinism rules as
//! `oag-physics`: `f32` only, no `mul_add`, no clock, randomness through the
//! seeded `oag_core::Rng`.
//!
//! **This crate sits below `oag-gameplay`.** `World` and `Ship` embed
//! [`projectile::Projectiles`], [`pickup::Held`] and [`disruption::Disruption`],
//! so the dependency has to run that way; the weapons code in turn reaches a craft
//! only through the [`Craft`] trait, which `oag_gameplay::Ship` implements. Every
//! function is generic over it rather than over `dyn`, so the arithmetic is the
//! same instructions it was when it named `Ship` directly.

pub mod craft;
pub mod disruption;
pub mod pickup;
pub mod projectile;
pub mod slowdown;

pub use craft::Craft;

/// The most ships a race can hold.
///
/// Eight, which is what Pulse grids. A hard array bound rather than a `Vec`
/// capacity: the limit is real, and making it visible is what keeps the world
/// snapshot a fixed size. It lives here, in the lowest crate that needs it,
/// because the quake wave and the repulser rings carry a per-craft `hit` array;
/// `oag_gameplay::MAX_SHIPS` is this constant, not a second eight.
pub const MAX_SHIPS: usize = 8;

#[cfg(test)]
mod test_craft;
