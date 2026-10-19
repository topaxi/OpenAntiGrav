//! The player step of [`Race::tick`], once per slot a person flies.
//!
//! ADR-0052 widened the world to N players and left the tick running one player
//! step, for [`oag_gameplay::World::primary_slot`]. A second `Local` or `Remote`
//! slot got its [`oag_gameplay::InputSnapshot`] in [`PlayerInputs`] and drove
//! nothing: [`Race::step_opponents`] flew its craft with the AI. This module is
//! the pieces of that step that are per craft, so the tick can run them over
//! [`oag_gameplay::World::person_flown`] and an AI slot stays an AI slot.
//!
//! What stays once-per-tick, because it is the field's rather than a craft's:
//! the slowdown drain, mine laying, cannons, the projectile pool, the
//! destroyed-craft pass, the standings. What stays on the primary slot because
//! it is the *view's* (camera, HUD, messages, shake, run statistics) is listed
//! in the handover thread, since split screen owns the per-viewport version.
//!
//! [`PlayerInputs`]: oag_gameplay::PlayerInputs

use super::*;
use oag_gameplay::MAX_PLAYERS;

/// What one tick's player step produced for each slot a person flies.
///
/// A fixed array indexed by slot - the same indexing as
/// [`oag_gameplay::World::race`] and `World::controllers` - and `None` for a slot
/// no person flies, which is every AI slot. [`Self::primary`] is the
/// single-player view: the craft [`oag_gameplay::World::primary_slot`] names, which is
/// what the whole return value used to be.
#[derive(Debug, Clone, Copy)]
pub struct Evaluations {
    slots: [Option<Evaluated>; MAX_PLAYERS],
    primary: usize,
}

impl Evaluations {
    pub(super) fn new(steps: &[Option<PlayerStep>; MAX_PLAYERS], primary: usize) -> Self {
        Self {
            slots: std::array::from_fn(|slot| steps[slot].as_ref().map(|step| step.evaluated)),
            primary,
        }
    }

    /// What slot `slot`'s step produced, or `None` if no person flew it.
    #[must_use]
    pub fn get(&self, slot: usize) -> Option<&Evaluated> {
        self.slots.get(slot)?.as_ref()
    }

    /// The primary slot's step: the craft the camera and the full-screen HUD
    /// follow. Always present - the primary slot is stepped as a player whoever
    /// the controllers say flies it.
    #[must_use]
    pub fn primary(&self) -> &Evaluated {
        self.slots[self.primary]
            .as_ref()
            .expect("the primary slot is always stepped as a player")
    }

    /// Every slot that was stepped, in slot order.
    pub fn flown(&self) -> impl Iterator<Item = (usize, &Evaluated)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(slot, evaluated)| evaluated.as_ref().map(|e| (slot, e)))
    }
}

/// The single-player reading: `evaluations.wall` is the primary slot's `wall`,
/// so a caller that flew one craft and read its [`Evaluated`] off the tick
/// keeps compiling and keeps meaning the same thing. A caller that wants
/// another slot says so with [`Evaluations::get`].
impl std::ops::Deref for Evaluations {
    type Target = Evaluated;

    fn deref(&self) -> &Evaluated {
        self.primary()
    }
}

/// One slot's step, kept for the second half of the tick.
///
/// The respawn trigger and the race rules run after the projectile pool has
/// moved, and want the environment and the position the force law ran against.
pub(super) struct PlayerStep {
    pub(super) evaluated: Evaluated,
    env: Environment,
    before: Vec3,
    spline_distance: Option<f32>,
}

impl Race {
    /// What slot `slot` asks of its craft this tick: the snapshot mapped
    /// through the control scheme, or the driver's answer when the craft is
    /// being flown for the player, then the finish line's cap, a Disruptor's
    /// filter and the start-line thrust gate.
    ///
    /// Flown for the player by either route - the operator's `--autopilot` or
    /// the pickup - and then the snapshot is not consulted for steering at all.
    /// See [`Race::flown_for_the_player`].
    pub(super) fn player_controls(
        &mut self,
        slot: usize,
        snapshot: &InputSnapshot,
    ) -> oag_physics::ShipControls {
        let flown = self.flown_for_the_player(slot);
        let mut controls = if flown {
            self.autopilot_controls(slot)
        } else {
            self.release_autopilot(slot);
            ship_controls(snapshot, self.sim.scheme)
        };
        // Past the line the original's own law sets the pace - see
        // `race::finished_thrust`.
        if let Some(cap) = self.finished_thrust_cap(slot) {
            controls.thrust = controls.thrust.min(cap);
        }
        // A Disruptor hit filters whatever the pilot - human or driver -
        // asked for: no thrust, no airbrakes, a mirrored yaw, or an
        // autopilot's thrust scale. Before the start-line gate below, so a
        // stalled craft on the line is still gated and not doubly so. See
        // `oag_weapons::disruption`.
        controls = self.disrupted_controls(slot, controls, flown);
        // The start-line countdown: measured, not authored - see
        // `RaceState::thrust_gated` for the live capture this reproduces. Only
        // thrust was held and recorded, so only thrust is gated; steering stays
        // live through the countdown, which is unmeasured either way but costs
        // nothing to leave alone. Autopilot is gated too - the capture measured
        // the engine's own gate, not a distinction between input sources.
        if RaceState::thrust_gated(self.sim.world.tick) {
            controls.thrust = 0.0;
        }
        controls
    }

    /// Steps slot `slot`'s craft: locate it on the track, run the pads, and
    /// run the force law with `controls`.
    pub(super) fn step_player(
        &mut self,
        slot: usize,
        controls: &oag_physics::ShipControls,
    ) -> PlayerStep {
        // The two spline samples the magstrip hold reads. In the original these are
        // `AiTrack_LocatePosition`'s two output records on the ship entity; here
        // they are the nearest table entry and the one after it, which is the same
        // "where am I and what is next" pair at this crate's resolution. Off a
        // magstrip nothing consumes them, so a track without one is unaffected.
        let nearest = self
            .sim
            .spline
            .nearest(self.sim.world.ships[slot].physics.body.position);
        let index = nearest.map(|(index, _, _)| index);
        // The same scan, read a third way: how far off the sample table the craft
        // is, which is the player's own off-track trigger. Taken from here
        // rather than scanned again after the step - the table is ~3,400 samples
        // and a second fold would double the tick's hottest loop to move a
        // measurement one tick, under a dwell of [`PLAYER_RESCUE_TICKS`].
        let spline_distance = nearest.map(|(_, _, distance)| distance);
        let track_sample = nearest.map(|(_, sample, _)| Spline::track_sample(sample));
        let crumb_flags = nearest.map(|(_, sample, _)| sample.flags);
        // The neighbour is the next entry in table order, which at the end of a
        // path is the *next path's* first sample rather than the geometric
        // successor through a junction. The original follows the junction graph;
        // this is one sample out of 34,000 per lap and is recorded rather than
        // pretended away.
        //
        // Still table order, deliberately: the player's locator works in sample
        // space, where every path the track authors is reachable. An opponent's
        // equivalent goes through [`Self::ai_sample`] instead, because a driver
        // is asking a lap question. See [`ai_order`].
        let track_sample_next = index
            .and_then(|index| self.sim.spline.sample(index + 1))
            .map(Spline::track_sample);

        // **Latched on the distance itself**, not on the counters later in the
        // tick, so where the craft is put back cannot depend on the order the
        // two are updated in. See [`RaceSim::last_on_track`].
        if let (Some(index), Some(distance)) = (index, spline_distance)
            && distance <= self.sim.player_rescue_distance
        {
            // Zero on an index that will not fit, matching `Race::start` - the
            // arm is unreachable on any real table, and the value it writes is
            // read back as a sample index, so it has to be a *valid* one. A
            // saturating `u32::MAX` here would make `Race::respawn` find no
            // sample and return having recovered nothing, silently.
            self.sim.last_on_track[slot] = u32::try_from(index).unwrap_or(0);
        }

        if let Some(index) = index {
            // An index into this module's own sample table, which is *not* what
            // `Ship::segment` documents: that field means a per-path control-point
            // segment. Nothing reads it today, and it is written because a locator
            // that keeps no note of where it was is the thing that has to be replaced
            // when the brute-force scan does. Converting the one into the other needs
            // a lap-counting convention nobody has recovered.
            self.sim.world.ships[slot].segment = u16::try_from(index).unwrap_or(u16::MAX);
        }

        // Zone's auto-speed, from the zone the run has reached. Read before the
        // step, from the zone the last tick left behind, because that is the
        // order the original runs in: `Zone_Update` assigns the counter into the
        // craft and `Ship_UpdateEngine` reads it on the following craft update.
        let auto_speed = self.sim.zone.map(|zone| {
            oag_race::zone::thrust(zone.start, zone.increment, self.sim.world.race[slot].zone)
        });
        let before = self.sim.world.ships[slot].physics.body.position;
        // Step 15's input, measured before the step because that is when the
        // original measures it: `Ship_ApplySpeedupPad` runs inside the same craft
        // update as the other fourteen terms, all of them against the position the
        // tick started at, and the integrator moves the body afterwards.
        let (moved, sweep) = self.pad_sweep(slot, before);
        let pad_hit = self.test_speedup_pads(slot, before, moved, &sweep);
        // After the speed pad, sharing its sweep. The two are independent - a
        // track can author a weapon pad on top of a speed pad and both fire -
        // and this one returns nothing, because a pickup is an event rather than
        // a per-tick force.
        self.test_weapon_pads(slot, before, moved, &sweep);
        // The mode's rules, read before the craft is borrowed: `World::mode`
        // asks the whole world and the borrow checker will not have both.
        let env_damage_rules = self.sim.damage_rules();
        let pilot_assist = self.pilot_assist_input(slot);
        let ship = &mut self.sim.world.ships[slot];
        // `Ship_UpdateEngine`'s read-and-reset of `craft+0x31c`: the beam's
        // throttle armed last tick is consumed by this step and nothing after
        // it. See `Ship::pending_thrust_scale`. One asymmetry: the original's
        // engine early-returns *above* that line for a stunned or slowed
        // craft and so keeps the armed value for a later tick, where this
        // discards it either way. A beam re-arms it every tick the link
        // holds, so the two differ only on the tick a stun ends after the
        // link has already broken - one tick of throttle, not chased.
        let thrust_scale = std::mem::replace(&mut ship.pending_thrust_scale, 1.0);
        let env = Environment {
            track_sample,
            track_sample_next,
            auto_speed,
            pad_hit,
            thrust_scale,
            class_gravity_scale: self.sim.class_gravity_scale,
            start_boost: self.sim.start_boost,
            // The mode's own `Weapons`/`Damage` defaults, which decide both what
            // a wall costs the energy pool and whether it recovers - a time trial
            // and a speed lap run with both off, so their pool floors at 20 and
            // regenerates, exactly as the disc's manual text describes. See
            // `oag_gameplay::damage_rules`.
            damage_rules: env_damage_rules,
            pilot_assist,
            barrel_rolls: self.sim.barrel_rolls,
            ..Environment::default()
        };
        // The autopilot's `Ai_SnapToLane` write, between its controls and
        // the step, as an opponent's (`Race::step_opponents`).
        if let Some(rail) = self.sim.autopilot_rail.get_mut(slot).and_then(Option::take) {
            oag_physics::rail::apply(&mut ship.physics.body, rail.velocity, rail.position);
        }
        let evaluated = oag_physics::step(
            &mut ship.physics,
            controls,
            &ship.handling,
            &env,
            &self.sim.collision,
            self.sim.dt,
        );
        self.lay_crumb(slot, crumb_flags);
        // The contact half of the same edge as the blast one, and the one a
        // player meets first: scraping a wall behind a shield costs nothing
        // and bulges the shell. The original's contact loop takes exactly this
        // branch instead of its hull-damage call.
        if evaluated.shield.absorbed {
            self.shield_hit(slot);
        }
        PlayerStep {
            evaluated,
            env,
            before,
            spline_distance,
        }
    }

    /// The player's off-track, airborne and reset-zone rescue for slot `slot`,
    /// after the tick's craft have all moved.
    pub(super) fn rescue_player(&mut self, slot: usize, step: &PlayerStep) {
        self.sim.respawn_cooldown[slot] = self.sim.respawn_cooldown[slot].saturating_sub(1);
        // Unconditionally and before the `||`, so the dwell sees every tick -
        // the same argument `step_opponents` makes for its two counters.
        let off_the_track = self.lost_off_the_track(slot, step.spline_distance);
        // **Not a wreck**, the same gate `step_opponents` applies: in an
        // Eliminator the player sits `Eliminated` for the respawn dwell, and
        // a rescue firing inside it would rebuild the craft as a racing one
        // with an empty pool. In a single race the ending has already fired.
        let alive =
            self.sim.world.ships[slot].physics.craft_state == oag_physics::CraftState::Racing;
        let reset = self.reset_zone_touched(slot, &step.env, step.before);
        let airborne = self.airborne_too_long(slot);
        if alive && (off_the_track || airborne || reset) {
            let cause = if off_the_track {
                respawn::RespawnCause::OffTrack
            } else if airborne {
                respawn::RespawnCause::Airborne
            } else {
                respawn::RespawnCause::ResetZone
            };
            self.sim.last_respawn_cause[slot] = Some(cause);
            // The last sample the craft was *on the track* at, which for a reset
            // contact is where it was a tick or two ago and for the off-track
            // trigger is where it left. The nearest sample to wherever the
            // craft is now would be that same place for the first and a
            // sample on some other part of the circuit for the second.
            let last_on_track = Some(self.sim.last_on_track[slot] as usize);
            if cause.is_the_originals() {
                self.reset(slot, last_on_track);
            } else {
                self.respawn(slot, last_on_track);
            }
        } else if self.sim.respawn_cooldown[slot] == 0 {
            // Clear of the trigger with the cooldown expired: whatever run of
            // back-to-back respawns was happening is over.
            self.sim.respawns_in_a_row[slot] = 0;
        }
    }

    /// Ends slot `slot`'s race when its craft finished blowing up this tick, and
    /// says whether it did.
    ///
    /// Checked every tick because [`RaceState::eliminate`] is what makes it
    /// idempotent. See the comment at the call for what reaches it.
    pub(super) fn eliminate_if_wrecked(&mut self, slot: usize) -> bool {
        self.sim.world.mode() != Mode::Eliminator
            && self.sim.world.ships[slot].physics.craft_state == oag_physics::CraftState::Eliminated
            && self.sim.world.race[slot].eliminate()
    }

    /// Runs the race rules on slot `slot`'s own [`RaceState`], from the position
    /// its step produced, and pays what they earn: Zone's shield recharge, the
    /// free Time Trial turbo, the Eliminator's lap refill.
    ///
    /// **The view's half stays on the primary slot.** The announcer, the
    /// lap messages, the perfect-zone statistic and the run statistics are one
    /// screen's, and a second person's screen is split screen's to build. A
    /// non-primary slot's rules run on `race[slot]` all the same, so its lap
    /// clock, its zone counter and its finish are the simulation's.
    pub(super) fn advance_player_rules(&mut self, slot: usize, evaluated: &Evaluated) {
        let primary = slot == self.sim.world.primary_slot();
        if primary && self.draws() {
            let body = &self.sim.world.ships[slot].physics.body;
            self.view
                .run_stats
                .observe(body.linear_velocity.dot(body.forward()).abs());
        }

        let Some(course) = &self.sim.course else {
            return;
        };
        let position = self.sim.world.ships[slot].physics.body.position;
        // The same flag the collision sparks fire on, so "the HUD says that
        // zone was not clean" and "sparks came off the hull" cannot disagree.
        let contact = evaluated.wall.impact;
        let tick = self.sim.world.tick;
        let outcome =
            self.sim.world.race[slot].update(course, position, tick, self.sim.dt, contact);

        // A zone survived without touching anything pays shield back, clamped
        // to the ship's own pool. `Ship_SetShield` (`0x0883e6f4`) does the
        // same clamp against the stat block's maximum, so a full ship gains
        // nothing and the bar cannot overfill.
        if primary && outcome.perfect_zone && self.draws() {
            self.view.run_stats.perfect_zones += 1;
        }
        if outcome.perfect_zone
            && let Some(zone) = self.sim.zone
        {
            let ship = &mut self.sim.world.ships[slot];
            let max = ship.handling.dimensions.shield;
            ship.physics.shield = (ship.physics.shield + zone.recharge).min(max);
        }

        // The announcer's own trigger: raised on every zone step, whether
        // or not this title's ladder names that particular number - the
        // same "raise the edge, let the audio layer decide if it has
        // anything to say" split `raise_contact_cue` and `Banks::pick`
        // already keep. See `oag_title::ZoneAnnouncer`.
        if primary && outcome.zone_advanced {
            self.push_announcement(self.sim.world.race[slot].zone);
        }

        // The speed-class announcer's own trigger, on a title whose
        // zone-to-stage ladder is recovered: the zone counter just
        // stepped by exactly one (`RaceState::advance_zone`'s own
        // `saturating_add(1)`), so the zone before this tick is the new
        // value minus one. Fires only when that step also crosses a
        // stage boundary - most zone steps do not, since the ladder's
        // bands are wider than one zone - which is what keeps this from
        // announcing a class on every zone survived. See
        // `oag_title::ZoneStages` and `oag_title::ZoneClassAnnouncer`.
        if primary
            && outcome.zone_advanced
            && let Some(stages) = self.sim.zone_stages
        {
            let after = self.sim.world.race[slot].zone;
            let before = after.saturating_sub(1);
            if let Some(stage_after) = stages.stage_for(after)
                && Some(stage_after) != stages.stage_for(before)
            {
                self.push_class_announcement(stage_after);
            }
        }

        // The free Time Trial/Speed Lap Turbo, on every tick the player
        // crosses the line forwards - the first crossing included - as the
        // original's state-2 handlers do (`0x0882ddd8`, `0x0882d578`,
        // `0x08823270`: `craft+0x911` set -> held = 4). Never at the
        // release: a craft that has not reached the line holds nothing.
        if outcome.lap_completed || outcome.first_crossing {
            self.grant_free_turbo(slot);
        }
        self.observe_lap(slot, &outcome, evaluated);
        if outcome.lap_completed {
            // Eliminator's own per-lap mechanic, a no-op on every other
            // mode - see `Race::eliminator_lap_health_refill`.
            self.eliminator_lap_health_refill(slot);
        }
    }
}

impl Race {
    #[cfg(feature = "view")]
    /// Whether this machine grades `slot`'s laps: the player's slot, not the
    /// world's lowest person slot (in a network race a client flying seat 1
    /// would otherwise grade seat 0's), and in a local race every person's own
    /// slot, so each seat of a split or multi-window race gets its own lines.
    fn watches_laps_of(&self, slot: usize) -> bool {
        slot == self.player_slot()
            || (self.view.seat.is_none() && self.sim.world.person_flown(slot))
    }

    /// Feeds each local person's craft to its own lap watch. A replayed tick
    /// (`tick_sim_only`) does not feed it, because the predicted tick it re-runs
    /// already did. **Chosen, not measured.**
    #[cfg_attr(not(feature = "view"), allow(unused_variables))]
    pub(super) fn observe_lap(
        &mut self,
        slot: usize,
        outcome: &oag_race::Outcome,
        evaluated: &Evaluated,
    ) {
        #[cfg(feature = "view")]
        let pair = std::mem::take(&mut self.view.pair_spoil[slot]);
        #[cfg(feature = "view")]
        if self.draws() && self.watches_laps_of(slot) {
            self.raise_lap_messages(slot, outcome, pair || lap_spoiling_contact(evaluated));
        }
    }
}

#[cfg(feature = "view")]
/// Whether this tick's contact sets the original's `0x20` bit of `craft+0x860`
/// (`contact-response.md`, reaction #2): the hull-damage branch of
/// `FUN_088418e0`'s loop, taken for **every** contact record whose combined
/// friction is positive, whatever the sign of its normal speed or the size of
/// its impulse, and not taken while a shield absorbs it (`shield-pickup.md`).
/// So a sustained scrape counts, not only the inbound edge `wall.impact` marks.
/// A craft-to-craft contact counts too, through `RaceView::pair_spoil`.
fn lap_spoiling_contact(evaluated: &Evaluated) -> bool {
    evaluated.wall.resolved.is_some() && !evaluated.shield.absorbed
}
