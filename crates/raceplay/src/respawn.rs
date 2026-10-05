//! Putting a craft back on the track: the two ways one is judged lost, and the
//! respawn itself.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/respawn.rs`.

use super::*;
use log::warn;

/// Which of the four triggers put a craft back, for a caller measuring where
/// craft leave the track.
///
/// Bookkeeping only, outside [`Race::state_hash`](super::Race): it records
/// what already fired and changes nothing about whether it does. Each variant
/// names one guard in this file, so a survey can tell a craft that fell
/// through authored `Reset` geometry from one that one of this project's own
/// invented dwells (see [`RESCUE_HALF_WIDTHS`], [`PLAYER_RESCUE_HALF_WIDTHS`],
/// [`STALL_SPEED`]) caught.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespawnCause {
    /// An authored `Reset` volume was touched. The original's own mechanism.
    ResetZone,
    /// An opponent stayed far from its driver's sample - invented.
    LostCircuit,
    /// An opponent stayed on the ground well below its driver's sample, on a
    /// road under the circuit - invented, see [`BENEATH_LINE`].
    Beneath,
    /// An opponent stayed stopped while asking to move - invented.
    Stalled,
    /// The player stayed far from the nearest spline sample - invented.
    OffTrack,
    /// No hover probe touched anything for [`oag_race::recovery::AIRBORNE_RESET_SECONDS`]:
    /// the original's own rescue for a craft beached upside down or on its
    /// flank, `FUN_088418e0` at `0x08841d30`.
    Airborne,
    /// The craft was wrecked and is being brought back by the destroyed-craft
    /// pass, not rescued from anywhere: a shield running out is not a fall.
    Destroyed,
}

impl RespawnCause {
    /// A short lowercase label for a survey line.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::ResetZone => "reset",
            Self::LostCircuit => "lost",
            Self::Beneath => "beneath",
            Self::Stalled => "stalled",
            Self::OffTrack => "offtrack",
            Self::Airborne => "airborne",
            Self::Destroyed => "wrecked",
        }
    }
}

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
        let index = self.sim.world.ships[slot].driver.index as usize;
        let away = self.sim.world.ships[slot]
            .physics
            .body
            .position
            .distance(self.line_of(slot).point(index))
            > self.sim.rescue_distance
            || self.beneath_the_line(slot);
        self.sim.lost_ticks[slot] = if away {
            self.sim.lost_ticks[slot].saturating_add(1)
        } else {
            0
        };
        // An empty line puts every point at the origin, so a track with no
        // spline would read every craft as lost and respawn it onto nothing.
        //
        // The same two guards `reset_zone_touched` applies, applied *after* the
        // counter so the dwell is still measured while they hold.
        !self.sim.racing_line.is_empty()
            && !self.sim.respawn_disabled[slot]
            && self.sim.respawn_cooldown[slot] == 0
            && self.sim.lost_ticks[slot] >= RESCUE_TICKS
    }

    /// Whether this opponent is on the ground more than [`BENEATH_LINE`] below
    /// its driver's line point, measured along that sample's own up.
    pub(super) fn beneath_the_line(&self, slot: usize) -> bool {
        let ship = &self.sim.world.ships[slot];
        let index = ship.driver.index as usize;
        // Off the ground nothing is "on" a road, and over a gap the line arcs
        // through the air: a craft that comes down early lands beneath it on
        // the road it was always going to land on (`25_Track` reversed,
        // samples 690-740, 77 ticks in the census).
        let line = self.line_of(slot);
        if ship.physics.time_airborne > 0.0 || line.is_unsupported(index) {
            return false;
        }
        let Some(sample) = self.ai_sample_for(slot, index) else {
            return false;
        };
        let up = -Vec3::from_array(sample.down).normalize_or_zero();
        (line.point(index) - ship.physics.body.position).dot(up) > BENEATH_LINE
    }

    /// Whether this opponent has been stopped, while asking to move, for long
    /// enough to count as beached.
    ///
    /// The other half of [`Self::lost_off_the_circuit`], and deliberately a
    /// separate dwell: that one asks whether a craft is *far from* its line, which
    /// a craft wedged against the scenery is not. See [`STALL_SPEED`] for the
    /// measurement the two constants come from.
    ///
    /// Advances the dwell counter as a side effect, on the same terms and for the
    /// same reason [`Self::lost_off_the_circuit`] does.
    ///
    /// **Three conditions, and each excludes a craft that is stopped legitimately.**
    /// `thrust` above zero excludes one held on the grid before the lights.
    /// `!standing.finished()` excludes one coasting to a halt after it has taken
    /// the flag - measured at 2,242 consecutive ticks on `05_Track` at ace, which
    /// is the longest apparent stall on the whole disc and is not a stall at all.
    /// And the speed itself excludes one merely cornering slowly.
    pub(super) fn stalled(&mut self, slot: usize) -> bool {
        let ship = &self.sim.world.ships[slot];
        // `ShipState::thrust` is the commanded throttle on the original's `0..=100`
        // scale, written by `oag_physics::controls::update` from the controls the
        // driver returned this tick - so this reads what the craft asked for, not
        // what it got. `> 0.0` and not `>= 100.0`: `Driver`'s caution axis
        // multiplies the throttle by an arbitrary fraction, and an exact
        // comparison against a scaled throttle is the mistake that silently
        // stopped opponents boosting once.
        let stopped = ship.physics.thrust > 0.0
            && !ship.standing.finished()
            && ship.physics.body.linear_velocity.length() < STALL_SPEED;
        self.sim.stalled_ticks[slot] = if stopped {
            self.sim.stalled_ticks[slot].saturating_add(1)
        } else {
            0
        };
        // The same two guards the other two triggers apply, applied *after* the
        // counter so the dwell is still measured while they hold. No emptiness
        // check on the racing line here: unlike `lost_off_the_circuit` this
        // measures nothing that comes off it - but the respawn it leads to does,
        // so the guard stays where it is needed.
        !self.sim.racing_line.is_empty()
            && !self.sim.respawn_disabled[slot]
            && self.sim.respawn_cooldown[slot] == 0
            && self.sim.stalled_ticks[slot] >= STALL_TICKS
    }

    /// Whether the **player** has been off the track for long enough to be put
    /// back.
    ///
    /// `distance` is the player's distance to the nearest spline sample, measured
    /// where the tick already measures it - `None` on a track with no samples,
    /// which is a track this cannot recover anybody onto.
    ///
    /// # Why the player needs a trigger of their own
    ///
    /// The authored `Reset` volumes catch a craft that falls *through* the floor.
    /// They do not catch one that leaves the circuit sideways into open space -
    /// [`RESCUE_HALF_WIDTHS`] records the measurement, and the player reaches that
    /// state the same way an opponent does: `docs/gameplay/ai.md` has a
    /// hand-driven lap of `06_Track` going through the same drop the AI does,
    /// with no AI involved. Measured again for this: an autopiloted craft on
    /// `05_Track` left at tick 591 and was 7,983 units away by the end of the run.
    ///
    /// [`Self::lost_off_the_circuit`] cannot be reused for it. That one asks
    /// whether the craft and *its driver's* idea of where it is have come apart,
    /// and slot 0 has no driver unless `Self::set_autopilot` is on - its
    /// `driver.index` would sit wherever it was left, which is sample zero on an
    /// ordinary race.
    ///
    /// **The stall rescue is deliberately not extended to the player.** Being
    /// teleported off a wall one is scraping along, while holding the throttle, is
    /// a thing done *to* a player rather than for them; a human who is wedged can
    /// see it and back off, which is exactly what an opponent cannot do.
    ///
    /// Advances the dwell counter as a side effect, so it must be called once per
    /// tick and not conditionally, for the reason [`Self::lost_off_the_circuit`]
    /// gives.
    pub(super) fn lost_off_the_track(&mut self, slot: usize, distance: Option<f32>) -> bool {
        let away = distance.is_some_and(|distance| distance > self.sim.player_rescue_distance);
        self.sim.lost_ticks[slot] = if away {
            self.sim.lost_ticks[slot].saturating_add(1)
        } else {
            0
        };
        // A circuit that authors no width at all states no scale for "off the
        // track", so it gets no threshold rather than one at zero. The same
        // shape as the racing-line emptiness guard next door, and applied after
        // the counter for the same reason.
        self.sim.player_rescue_distance > 0.0
            && !self.sim.respawn_disabled[slot]
            && self.sim.respawn_cooldown[slot] == 0
            && self.sim.lost_ticks[slot] >= PLAYER_RESCUE_TICKS
    }

    /// Whether this tick ended in contact with `Reset` geometry.
    ///
    /// Suppressed while the cooldown runs and once respawning has given up, so
    /// the two guards live in one place rather than at the call site.
    pub(super) fn reset_zone_touched(&self, slot: usize, env: &Environment, before: Vec3) -> bool {
        if self.sim.respawn_disabled[slot] || self.sim.respawn_cooldown[slot] > 0 {
            return false;
        }
        let ship = &self.sim.world.ships[slot];
        // The same `env` the force law just ran with, so the self-collider
        // exclusion cannot differ between the two.
        oag_physics::reset::contact(
            &ship.physics,
            &ship.handling,
            env,
            &self.sim.collision,
            before,
        )
        .is_some()
    }

    /// Whether this craft has gone longer without a hover contact than the
    /// original allows, `FUN_088418e0`'s `*(craft+0x94)+0x284 > 4.0` at
    /// `0x08841d30`. See [`oag_race::recovery::AIRBORNE_RESET_SECONDS`].
    ///
    /// No dwell counter of its own: the clock is the physics state's, and
    /// [`Self::respawn`] rebuilds that state, so a craft put back starts again
    /// from zero.
    pub(super) fn airborne_too_long(&self, slot: usize) -> bool {
        !self.sim.respawn_disabled[slot]
            && self.sim.world.ships[slot].physics.time_airborne
                > oag_race::recovery::AIRBORNE_RESET_SECONDS
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
            .and_then(|index| self.sim.spline.sample(index))
            .or_else(|| self.ai_sample(0))
            .or_else(|| self.sim.spline.start());
        let Some(sample) = sample.copied() else {
            return;
        };

        let ship = &mut self.sim.world.ships[slot];
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
        ship.driver.index =
            self.sim
                .racing_line
                .nearest(pose.position, 0, self.sim.racing_line.len()) as u32;
        // Put back on the ring, so off any route. See `oag_ai::branch::Branching::on_ring`.
        ship.driver.branching = ship.driver.branching.on_ring();

        self.flash_player_reset(slot);
        self.sim.respawns[slot] += 1;
        self.sim.respawns_in_a_row[slot] += 1;
        self.sim.respawn_cooldown[slot] = RESPAWN_COOLDOWN_TICKS;
        // The craft is back on its line by definition, so the dwell starts
        // again rather than carrying over and rescuing it a second time on the
        // tick the cooldown expires. Both dwells, for the same reason: a craft
        // put back is also moving again shortly, and a stall counter that
        // survived the teleport would fire once more the moment the cooldown ran
        // out.
        self.sim.lost_ticks[slot] = 0;
        self.sim.stalled_ticks[slot] = 0;

        // Otherwise the ribbon spans the teleport: ten samples of history from
        // wherever the craft fell off, stretched across the track to where it was
        // put back. The camera is snapped for the same reason, and only for the
        // player, who is the only craft one is flown from.
        self.view.exhaust[slot].clear_trail();
        self.view.hd_trail[slot].clear();

        if self.sim.respawns_in_a_row[slot] >= RESPAWN_GIVE_UP {
            self.sim.respawn_disabled[slot] = true;
            // **Two causes, and naming only one sent people to the wrong place.**
            // This message used to say the pose was probably inside a `Reset`
            // volume, which is the right guess when the reset trigger is what
            // fired. Since the stall rescue landed there is a second, measured
            // one: the recovery pose is on the racing line, and some circuits
            // author a line that runs *above* their own collision surface -
            // `05_Track` for 134 samples - so a craft put back there comes off
            // again in the same place, however many times it is rescued.
            warn!(
                "reset: craft {slot} respawned {} times in a row without getting \
                 clear, giving up on it. Either the recovery pose is inside a \
                 Reset volume, or the racing line there runs above the collision \
                 surface; see Race::respawn and docs/gameplay/ai.md.",
                self.sim.respawns_in_a_row[slot]
            );
        }
    }
}
