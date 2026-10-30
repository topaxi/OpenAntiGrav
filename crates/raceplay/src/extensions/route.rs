//! `prefer-route`: a module names the route a slot's driver takes at its next fork.
//!
//! Held for one tick (the module re-sends it each tick it should hold) and read where the
//! driver moves onto a route, `Race::steer_branching`: the coin has been thrown by then,
//! so the race's random streams are what they would be without it. Hashed, after the
//! no-module early return, so a race without a module hashes as before.

#[cfg(feature = "extensions")]
use oag_gameplay::MAX_SHIPS;

/// Each slot's preferred route this tick, as the course numbers its routes.
#[cfg(feature = "extensions")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Preferred([Option<u16>; MAX_SHIPS]);

#[cfg(feature = "extensions")]
impl Preferred {
    /// `slot` prefers `route`; two modules for one slot: the later in id order wins.
    pub(super) fn set(&mut self, slot: usize, route: u16) {
        self.0[slot] = Some(route);
    }

    pub(super) fn hash_into(&self, hasher: &mut oag_core::hash::StateHasher) {
        for route in self.0 {
            hasher.write_u32(route.map_or(0, |route| u32::from(route) + 1));
        }
    }
}

impl super::Extensions {
    /// The route (the course's numbering) a module asked `slot`'s driver to take this
    /// tick; `None` in every race without a module.
    pub(crate) fn preferred_route(&self, slot: usize) -> Option<u16> {
        #[cfg(feature = "extensions")]
        {
            self.on.prefer.0.get(slot).copied().flatten()
        }
        #[cfg(not(feature = "extensions"))]
        {
            let _ = slot;
            None
        }
    }
}
