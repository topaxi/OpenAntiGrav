//! Which of the ways a craft is put back a title has.
//!
//! The numbers they fire on are a race's own (`oag_race::recovery::ResetTriggers`); what
//! differs between titles is whether a trigger exists. A title that has not read its own
//! runs Pulse's law (`docs/gameplay/leaving-the-track.md`).

use crate::pre_race::Sourced;

/// One enable per trigger, each with where it came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResetEnables {
    /// Touching an authored `Reset` volume.
    pub reset_zone: Sourced<bool>,
    /// No hover probe touching anything for too long.
    pub airborne: Sourced<bool>,
    /// Too far from the located spline sample for too long.
    pub distance: Sourced<bool>,
    /// The collision cage's reset.
    pub cage: Sourced<bool>,
    /// An opponent well under its line, on another road.
    pub beneath: Sourced<bool>,
    /// An opponent stopped while asking to move.
    pub stalled: Sourced<bool>,
}

impl ResetEnables {
    /// Pulse: the original's reset zone, airborne clock, distance dwell and cage are read
    /// off the executable; the beneath and stall rescues are this project's.
    pub const PULSE: Self = Self {
        reset_zone: Sourced::measured(true),
        airborne: Sourced::measured(true),
        distance: Sourced::measured(true),
        cage: Sourced::measured(true),
        beneath: Sourced::chosen(true),
        stalled: Sourced::chosen(true),
    };

    /// The same table, run on another title that has not read its own.
    pub const fn inherited(from: &'static str) -> Self {
        Self {
            reset_zone: Sourced::inherited(true, from),
            airborne: Sourced::inherited(true, from),
            distance: Sourced::inherited(true, from),
            cage: Sourced::inherited(true, from),
            beneath: Sourced::chosen(true),
            stalled: Sourced::chosen(true),
        }
    }
}
