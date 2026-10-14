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
use crate::player_step::Evaluations;

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
    /// view** if they run through [`Race::tick`], which advances the chase
    /// camera, draws sparks, shakes on impact and raises sound cues again.
    /// Replay through [`Self::tick_sim_only`] instead.
    pub fn restore_sim(&mut self, snapshot: &RaceSim) {
        self.sim = snapshot.clone();
    }

    /// One tick of the simulation and nothing else: the replay step of a
    /// reconciliation.
    ///
    /// Advances [`RaceSim`] as [`Race::tick`] does - in every scenario pinned,
    /// the state hash after N of these equals the hash after N ticks from the
    /// same state and the same inputs, with one structural exception below -
    /// and leaves the picture and the sound alone. Not touched:
    /// the chase camera and every other camera, the shake, the particle systems
    /// and every view RNG (`sparks_rng`, `shake_rng`, `stage_rng`, `exhaust_rng`,
    /// `engine_light_rng`, `leach_beam_rng`), the exhausts, trail hits, shield
    /// flashes, airbrake flaps, `boost_kick`, the HUD and the recording. The
    /// cues, announcements and class announcements the tick raised are dropped:
    /// the tick being replayed already raised them once.
    ///
    /// **A mode on the one body, not a second body.** The view writes are
    /// interleaved with the simulation inside the helpers (a craft's step
    /// flashes its shield, a pad crossing lights the plume, a respawn clears the
    /// trail), so a separate entry point would be a second 700-line copy of
    /// `tick` or a split of a dozen helpers. [`Race::tick`] with `quiet` unset
    /// runs the same instructions in the same order as before the mode existed,
    /// so no single-player hash moves by construction. The cues are dropped by
    /// truncating the three queues to their length at entry rather than gating
    /// each of the twenty-odd push sites: a cue's bookkeeping
    /// (`contact_cue_cooldown`) is sim state and must keep running.
    ///
    /// **What still runs although it lives in the view**, because the
    /// simulation or the caller reads it back:
    ///
    /// - `update_sight`, while the primary holds a Missile or a LeachBeam. The
    ///   primary's shot takes its target only once
    ///   [`RaceView::sight`](crate::view::RaceView) reports a lock, and that lock
    ///   is advanced from the chase camera's projection. This is a
    ///   view-to-sim dependency that exists in [`Race::tick`] too; it reads
    ///   the camera and never writes it. **That is the exception to the hash
    ///   guarantee**: the camera a sim-only tick projects through is not
    ///   advanced, and the reticle is not updated on ticks where no lock weapon
    ///   is held (its hold, previous point and lock are left as they were), so a
    ///   replay in which the primary fires a lock weapon is only as exact as the
    ///   reticle it starts from. Pinned equal in a countdown, in a moving pack
    ///   and with the lock forming inside the window, not proven in general. The
    ///   client's reticle and the server's can also disagree about a lock without
    ///   any misprediction. "Camera untouched" and "hash always equal" cannot
    ///   both hold while the lock reads the camera.
    /// - `capture_results`, which writes the scoreboard once, on the first tick
    ///   the race is finished.
    /// - `wreck_ended_tick`, which `Race::runs_on_after_the_wreck` reads.
    ///
    /// **The recording does not hear a replayed tick.** A client recording its
    /// race keeps the inputs it predicted, not the corrected ones; correcting a
    /// recording is its own piece of work.
    pub fn tick_sim_only(&mut self, inputs: &oag_gameplay::PlayerInputs) -> Evaluations {
        let queued = (
            self.sim.cues.len(),
            self.sim.announcements.len(),
            self.sim.class_announcements.len(),
        );
        let was_quiet = std::mem::replace(&mut self.quiet, true);
        let evaluations = self.tick(inputs);
        self.quiet = was_quiet;
        self.sim.cues.truncate(queued.0);
        self.sim.announcements.truncate(queued.1);
        self.sim.class_announcements.truncate(queued.2);
        evaluations
    }

    /// Whether this tick draws and sounds, as opposed to only simulating.
    /// False only inside [`Self::tick_sim_only`].
    pub(super) fn draws(&self) -> bool {
        !self.quiet
    }

    /// A shell bulge on `slot`, if this tick is drawn. View-only.
    pub(super) fn shield_hit(&mut self, slot: usize) {
        if self.draws()
            && let Some(shell) = self.view.shield.get_mut(slot)
        {
            shell.hit();
        }
    }

    /// `slot`'s shield coming up, if this tick is drawn. View-only.
    pub(super) fn shield_activate(&mut self, slot: usize) {
        if self.draws() {
            self.view.shield[slot].activate();
        }
    }

    /// `slot`'s boost plume lighting, if this tick is drawn. View-only.
    pub(super) fn boost_plume(&mut self, slot: usize) {
        if self.draws() {
            self.view.exhaust[slot].boost(exhaust::BOOST_SECONDS);
        }
    }
}
