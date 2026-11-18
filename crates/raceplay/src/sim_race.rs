//! [`SimRace`]: the simulation of a race and what it keeps beside its state.
//!
//! [`RaceSim`] is the state a tick reads and writes, hashed and snapshotted whole. Beside it sits
//! the speed-plan job, which only moves work off the frame thread. It is not in
//! [`RaceSim::state_hash`] or in [`Race::snapshot_sim`](crate::Race::snapshot_sim), so a restore
//! leaves it as it stands. `Race` holds one as `core`.

use crate::sim::RaceSim;
use crate::speed_plan::SpeedPlanJob;

/// A race's simulation and what sits beside it; see the module doc comment.
#[derive(Debug)]
pub struct SimRace {
    /// Everything that decides what happens next: the world, the track, the opponents and the
    /// rules. See [`crate::sim`].
    pub sim: RaceSim,
    /// Speed plans being learned on a thread, see [`SimRace::begin_speed_plans`]. Not state:
    /// it only moves work off the frame thread, and its result is adopted only if the race
    /// still matches what it was learned from.
    pub(crate) plans: Option<SpeedPlanJob>,
}
