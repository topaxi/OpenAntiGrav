//! Putting a craft back on the track: the two ways one is judged lost, and the
//! respawn itself.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/respawn.rs`.

use super::*;

impl Race {
    /// Whether this opponent has been away from its own driver's idea of where
    /// it is for long enough to count as lost.
    ///
    /// Advances the dwell counter as a side effect, so it must be called once
    /// per craft per tick and not conditionally - a caller that skipped it
    /// while a cooldown ran would let a craft bank dwell it never spent.
    ///
    /// See [`RESCUE_HALF_WIDTHS`] for why this exists next to the reset volumes
    /// rather than instead of them.
    pub(super) fn lost_off_the_circuit(&mut self, slot: usize) -> bool {
        let ship = &self.world.ships[slot];
        let index = ship.driver.index as usize;
        let away = ship
            .physics
            .body
            .position
            .distance(self.racing_line.point(index))
            > self.rescue_distance;
        self.lost_ticks[slot] = if away {
            self.lost_ticks[slot].saturating_add(1)
        } else {
            0
        };
        // An empty line puts every point at the origin, so a track with no
        // spline would read every craft as lost and respawn it onto nothing.
        //
        // The same two guards `reset_zone_touched` applies, applied *after* the
        // counter so the dwell is still measured while they hold.
        !self.racing_line.is_empty()
            && !self.respawn_disabled[slot]
            && self.respawn_cooldown[slot] == 0
            && self.lost_ticks[slot] >= RESCUE_TICKS
    }

    /// Whether this tick ended in contact with `Reset` geometry.
    ///
    /// Suppressed while the cooldown runs and once respawning has given up, so
    /// the two guards live in one place rather than at the call site.
    pub(super) fn reset_zone_touched(&self, slot: usize, env: &Environment, before: Vec3) -> bool {
        if self.respawn_disabled[slot] || self.respawn_cooldown[slot] > 0 {
            return false;
        }
        let ship = &self.world.ships[slot];
        // The same `env` the force law just ran with, so the self-collider
        // exclusion cannot differ between the two.
        oag_physics::reset::contact(&ship.physics, &ship.handling, env, &self.collision, before)
            .is_some()
    }

    /// Puts the ship back on the track after a `Reset` contact.
    ///
    /// # This pose is a guess, not a reading
    ///
    /// `docs/ghidra/functions/psp-pulse-usa/collision.md` records **that** a `Reset`
    /// contact respawns the ship, at confidence 86. **Where it respawns it is not
    /// recorded anywhere**, and searching the RE tree for it found nothing - which
    /// is itself the finding. So this reuses the initial spawn: the racing line at
    /// [`spawn_height`], on the spline sample nearest where the ship was before
    /// the tick that triggered the reset.
    ///
    /// Confidence **40**. That is deliberately low, and the number matters: this
    /// is a placeholder chosen because it reuses code that is already correct for
    /// the race start, not because anything says the original does it. The likelier
    /// real mechanism is a last-passed checkpoint or track section - [`Ship::segment`]
    /// is the field that would hold it, and nothing populates it meaningfully today.
    /// Whoever recovers that should replace this outright rather than tune it.
    ///
    /// The velocity, orientation and every control state go to zero, because
    /// [`Ship::place_at`] resets the whole physics state and keeps only mass and
    /// inertia. Whether the original preserves any speed through a respawn is also
    /// unrecorded.
    pub(super) fn respawn(&mut self, slot: usize, sample_index: Option<usize>) {
        // **The fallback is the lap's first sample, not the table's.** They are
        // the same on every circuit the disc ships, and this arm fires exactly
        // when the caller had no index to give - which for an opponent means
        // `ai_order` was empty, the one case where "sample zero" has no
        // relationship to any lap at all.
        let sample = sample_index
            .and_then(|index| self.spline.sample(index))
            .or_else(|| self.ai_sample(0))
            .or_else(|| self.spline.start());
        let Some(sample) = sample.copied() else {
            return;
        };

        let ship = &mut self.world.ships[slot];
        let height = spawn_height(&ship.handling);
        let pose = Pose::from_sample(&sample, sample.racing_line, height);
        ship.place_at(pose);

        // **The driver has to be told where it has been put.** `Driver::drive`
        // locates a craft in a 48-sample window around its last index, so a
        // teleport of more than that leaves the driver steering at the piece of
        // track the craft fell off - the same failure the grid had when
        // `Driver::index` started at zero, except mid-race and invisible,
        // because a craft that recovered and then drove into the scenery reads
        // as bad driving rather than as a lost index.
        ship.driver.index = self
            .racing_line
            .nearest(pose.position, 0, self.racing_line.len()) as u32;

        self.respawns[slot] += 1;
        self.respawns_in_a_row[slot] += 1;
        self.respawn_cooldown[slot] = RESPAWN_COOLDOWN_TICKS;
        // The craft is back on its line by definition, so the dwell starts
        // again rather than carrying over and rescuing it a second time on the
        // tick the cooldown expires.
        self.lost_ticks[slot] = 0;

        // Otherwise the ribbon spans the teleport: ten samples of history from
        // wherever the craft fell off, stretched across the track to where it was
        // put back. The camera is snapped for the same reason, and only for the
        // player, who is the only craft one is flown from.
        self.exhaust[slot].clear_trail();

        if self.respawns_in_a_row[slot] >= RESPAWN_GIVE_UP {
            self.respawn_disabled[slot] = true;
            eprintln!(
                "reset: craft {slot} respawned {} times in a row without getting \
                 clear, giving up on it. The recovery pose is probably inside a \
                 Reset volume; see Race::respawn.",
                self.respawns_in_a_row[slot]
            );
        }
    }
}
