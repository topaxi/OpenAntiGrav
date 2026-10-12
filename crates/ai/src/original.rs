//! The original's own opponent, for the campaign: Wipeout Pulse's driver as
//! recovered in `docs/gameplay/ai.md`, "The original driver, recovered", with
//! the instruction-level evidence on
//! `docs/ghidra/functions/psp-pulse-usa/ai-controls.md`.
//!
//! **This is the one place this crate reads the player.** [`crate::Driver`]
//! never does; the original's opponent indexes its throttle by the player's
//! place, holds a formation around the player and rubber-bands field-wide on
//! the player's lead. The maintainer's rule (2026-10-11) gives it to the
//! campaign only, chosen by the game mode and never by a player setting.
//!
//! It writes the same controls a player's pad does and flies the same physics.
//! The one exception is the rail ([`Rail`]): a deterministic write to the
//! body's position and velocity, which the original makes every tick.
//!
//! Every tuning value is read from the disc at runtime ([`ClassRace`],
//! [`Controller`]); the literals in these modules are the executable's code
//! constants, each cited to the evidence page.
//!
//! [`ClassRace`]: oag_tables::ai_race_stats::ClassRace
//! [`Controller`]: oag_tables::ai_control_stats::Controller

mod cruise;
mod field;
mod lane;
mod laws;
mod pack;
mod rail;
mod sibling;
mod steer;
mod thrust;
mod track;

pub use cruise::{
    CruiseDraws, CruiseState, cruise_thrust, finish_thrust, gap_along_track, zone_multiplier,
    zone_speed,
};
pub use field::{Field, OriginalDriver, Output, SLOTS, Standings, Tick};
pub use lane::{PlanInput, Planner, Ring};
pub use laws::{Laws, RING_RATE, event_ring_rate};
pub use pack::{Wander, race_fraction, shuffle};
pub use rail::{Rail, RailCorrection};
pub use sibling::SingleLane;
pub use steer::{Controller, Steering, airborne, lookahead_distance, pitch, steer};
pub use thrust::{Racer, Thrust, ThrustInput, opponent_thrust};
pub use track::{Pose, TrackPoint};
