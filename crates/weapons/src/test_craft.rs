//! A stand-in for `oag_gameplay::Ship` and `World`, for this crate's own tests.
//!
//! The real ones live in a crate above this one, which cannot be a
//! dev-dependency without compiling this crate twice. These carry the same
//! field names and defaults as the parts of `Ship` and `World` a weapon reads,
//! so a test reads the way it did when it named the real ones.

use oag_physics::params::Dimensions;
use oag_physics::{Handling, ShipState};

use crate::disruption::Disruption;
use crate::projectile::Projectiles;
use crate::{Craft, MAX_SHIPS};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ship {
    pub physics: ShipState,
    pub handling: Handling,
    pub standing: oag_race::Standing,
    pub pending_slowdown: f32,
    pub pending_thrust_scale: f32,
    pub disruption: Disruption,
    pub active: bool,
}

impl Default for Ship {
    fn default() -> Self {
        Self {
            physics: ShipState::default(),
            handling: Handling::ZERO,
            standing: oag_race::Standing::default(),
            pending_slowdown: 0.0,
            pending_thrust_scale: 1.0,
            disruption: Disruption::default(),
            active: false,
        }
    }
}

impl Craft for Ship {
    fn active(&self) -> bool {
        self.active
    }
    fn physics(&self) -> &ShipState {
        &self.physics
    }
    fn physics_mut(&mut self) -> &mut ShipState {
        &mut self.physics
    }
    fn dimensions(&self) -> Dimensions {
        self.handling.dimensions
    }
    fn pending_slowdown(&self) -> f32 {
        self.pending_slowdown
    }
    fn pending_slowdown_mut(&mut self) -> &mut f32 {
        &mut self.pending_slowdown
    }
    fn pending_thrust_scale_mut(&mut self) -> &mut f32 {
        &mut self.pending_thrust_scale
    }
    fn disruption(&self) -> &Disruption {
        &self.disruption
    }
    fn disruption_mut(&mut self) -> &mut Disruption {
        &mut self.disruption
    }
    fn progress(&self) -> Option<f32> {
        self.standing.progress
    }
    fn course_index(&self) -> Option<u32> {
        self.standing.course_index
    }
}

/// The field and the projectile pool, as `oag_gameplay::World` holds them.
pub(crate) struct World {
    pub ship_count: u8,
    pub ships: [Ship; MAX_SHIPS],
    pub projectiles: Projectiles,
}

impl World {
    pub fn new(_seed: u64) -> Self {
        Self {
            ship_count: 0,
            ships: [Ship::default(); MAX_SHIPS],
            projectiles: Projectiles::new(),
        }
    }
}
