//! [`SimRace`]: the simulation of a race and what it keeps beside its state.
//!
//! [`RaceSim`] is the state a tick reads and writes, hashed and snapshotted whole. Beside it sit
//! the race's [`Outcome`] (the results board, the wreck's end tick, the run's tallies, who sits
//! where), which the tick writes and the screens read, and the speed-plan job, which only moves
//! work off the frame thread. Neither is in [`RaceSim::state_hash`] or in
//! [`Race::snapshot_sim`](crate::Race::snapshot_sim), so a restore leaves both as they stand.
//! `Race` holds one as `core`.

use crate::outcome::Outcome;
use crate::sim::RaceSim;
use crate::speed_plan::SpeedPlanJob;

/// A race's simulation and what sits beside it; see the module doc comment.
#[derive(Debug)]
pub struct SimRace {
    /// Everything that decides what happens next: the world, the track, the opponents and the
    /// rules. See [`crate::sim`].
    pub sim: RaceSim,
    /// What the race decided or was told that is not state: the results board, the wreck's
    /// end tick, the run's tallies, the slot teams, the seat and the people's names.
    pub(crate) outcome: Outcome,
    /// Speed plans being learned on a thread, see [`SimRace::begin_speed_plans`]. Not state:
    /// it only moves work off the frame thread, and its result is adopted only if the race
    /// still matches what it was learned from.
    pub(crate) plans: Option<SpeedPlanJob>,
}
