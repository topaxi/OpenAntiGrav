//! Snapshot and restore: the two operations a network client reconciles with.
//!
//! A server-authoritative client predicts ahead of the server using its own
//! just-captured input, and when an authoritative snapshot arrives tagged with
//! the tick it was computed from, the client puts that snapshot back and
//! replays the inputs it has buffered since. These are the put-back half.
//!
//! The protocol this is the first piece of is designed rather than observed -
//! network play is the one subsystem this project does not reimplement from an
//! original. What makes a prediction worth trusting is
//! [ADR-0007](../../../../docs/architecture/adr/0007-fixed-timestep-vs-original.md)'s
//! fixed 60 Hz step, and what makes N players expressible at all is
//! [ADR-0052](../../../../docs/architecture/adr/0052-world-and-race-tick-widen-to-n-players.md).
//! `race/tests/reconcile.rs` is the loopback client/server that proves the
//! pair works against the real tick.
//!
//! **There is no networking here.** No transport, no wire format, no
//! `oag-net`. A protocol written before this pair was proven would be a
//! protocol written before its own shape is known, which is the mistake
//! `docs/architecture/workspace-layout.md` warns about for `oag-net`
//! specifically.

use super::*;

impl Race {
    /// A restorable copy of everything that decides what happens next.
    ///
    /// The simulation half only. See [`Self::restore_sim`] for why that is the
    /// whole point rather than an omission.
    #[must_use]
    pub fn snapshot_sim(&self) -> RaceSim {
        self.sim.clone()
    }

    /// Puts a snapshot back, leaving the view untouched.
    ///
    /// **The sim, never the whole `Race`.** [`RaceView`] is the camera, the
    /// particles, the exhaust and the sound banks, and a reconciliation that
    /// restored it would snap the camera back every time the server corrected
    /// a prediction - a visible jolt caused by the correction machinery rather
    /// than by anything that happened in the race. Keeping the view out is
    /// what the sim/view split was made for.
    ///
    /// **Restoring does not undo what the replayed ticks will do to the
    /// view.** Re-running ticks after this call advances the chase camera,
    /// draws sparks, shakes on impact and raises sound cues again, because
    /// [`Race::tick`] does both halves in one body. None of that reaches
    /// [`RaceSim::state_hash`], so it cannot desync a race - but a client that
    /// reconciles for real needs a tick path that does not re-fire it, and
    /// this pair does not provide one.
    pub fn restore_sim(&mut self, snapshot: &RaceSim) {
        self.sim = snapshot.clone();
    }
}
