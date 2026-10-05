//! `Ship` as `oag-weapons` sees it.
//!
//! The weapons crate sits below this one and reaches a craft only through
//! [`oag_weapons::Craft`]; this is the one impl, and every method is a plain
//! field access, so a weapon reads and writes exactly the fields it did when it
//! named `Ship` directly.

use oag_physics::{ShipState, params::Dimensions};
use oag_weapons::Craft;
use oag_weapons::disruption::Disruption;

use crate::world::Ship;

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
