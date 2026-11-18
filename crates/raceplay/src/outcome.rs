//! [`Outcome`]: what a race decided or was told that is not simulation state.
//!
//! The results board, the tick a wreck ended the race on, the run's tallies, the grid's teams
//! and who sits where. Each is written once by the tick or a setter and read by the end screens,
//! the camera and the caller; none is in [`RaceSim::state_hash`](crate::RaceSim::state_hash) or
//! in [`Race::snapshot_sim`](crate::Race::snapshot_sim), so a restore leaves them standing.
//!
//! **Beside [`RaceSim`](crate::RaceSim), not in it**, in both builds: a race with no view still
//! reports them (`oag-server` prints the results, the slot teams and the seat). ADR-0087 decision
//! 9 refuses a take-over after the finish or while the seat is wrecked because the results
//! capture and the wreck's end tick live here and a fresh race would not have them.

/// What a race decided or was told that is not simulation state; see the module doc comment.
/// Held as [`SimRace::outcome`](crate::SimRace::outcome).
#[derive(Debug, Default)]
pub struct Outcome {
    /// The grid slot this machine's person flies in a network race, set by
    /// [`crate::Race::set_seat`]. Not in the world: in a network race every person's slot is
    /// [`oag_gameplay::Controller::Remote`] on every peer, because the controllers are hashed,
    /// so "which craft is mine" cannot live in the world.
    pub(crate) seat: Option<usize>,
    /// The name of the person flying each slot, by slot, empty for a slot with none
    /// ([`crate::Race::set_person_names`]): the lobby's in a network race, the stored player name
    /// for seat 1 in a local one. The end screens and the multiplayer tags name people.
    pub(crate) person_names: Vec<String>,
    /// The tick the player's wreck ended the race on, `None` until it does.
    /// Bookkeeping for [`crate::Race::runs_on_after_the_end`] and the spectator director's
    /// clock; nothing [`RaceSim`](crate::RaceSim) reads.
    pub(crate) wreck_ended_tick: Option<u64>,
    /// The team id each slot flies - [`Setup::slot_teams`](crate::Setup::slot_teams).
    pub(crate) slot_teams: Vec<String>,
    /// The results table, taken on the tick the race reached its finish
    /// condition, and `None` before that.
    ///
    /// **A snapshot rather than a live query**: "the race is over" has to mean
    /// one table, not one that moves under a reader at every tick. The field is
    /// still moving when the player crosses - `oag_race::places` ranks whoever
    /// has not finished by distance covered - so the board is replaced only by
    /// [`crate::Race::refresh_results`], when the world runs on behind the end screens,
    /// and each replacement bumps [`Self::results_revision`].
    ///
    /// Not in `World`: it is derived from state the world already holds, so hashing it would
    /// hash the same facts twice, and a replay reproduces it by reaching the same tick.
    pub(crate) results: Option<crate::scoreboard::Board>,
    /// Boards taken or replaced so far, for [`crate::Race::results_revision`].
    pub(crate) results_revision: u32,
    /// The tallies the Zone results screen reports - see
    /// [`RunStats`](crate::results::RunStats).
    pub(crate) run_stats: crate::results::RunStats,
}
