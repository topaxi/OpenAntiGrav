//! The handling a Zone race flies where a title reads a Zone file of its own.
//!
//! Wipeout HD's `Team_LoadHandlingStats` reads `Data\Ships\Zone\handlingstats.xml` in place of
//! the team's file on the Zone modes, and that file authors its blocks directly under `<Stats>`
//! with no `<Class>`: HD's parser fills class 0 from it, and a Zone race reads class 0
//! (`docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md`). Pulse's Zone flies its team's own
//! class (`docs/ghidra/functions/psp-pulse-usa/zone-rest.md`), so it has none of this. Data, not a
//! comparison on a title's name (ADR-0058).

use crate::pre_race::Sourced;

/// One title's Zone handling file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneHandling {
    /// The archive entry the Zone race reads in place of the team's `handlingstats.xml`.
    pub entry: Sourced<&'static str>,
    /// `<Antigrav landing_rebound>` where the file authors none: what the engine's stats block
    /// holds before the file is read.
    pub landing_rebound: Sourced<f32>,
    /// `<Antigrav rebound_jump_time>` where the file authors none, on the same terms.
    pub rebound_jump_time: Sourced<f32>,
}
