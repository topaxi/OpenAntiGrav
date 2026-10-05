//! What a running race says about itself: one tick's telemetry, the wrong-way
//! test, and the HUD's readout.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. The
//! [`Telemetry`] struct itself followed it out of the root on 2026-08-23, for
//! the same reason and to the module it was always named for.

use super::*;

/// One tick's worth of what the simulation did, for a log or an overlay.
///
/// Everything here is read out of the state *after* a step; nothing in it is an
/// input to the next one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Telemetry {
    /// Ticks elapsed since the race began.
    pub tick: u64,
    /// Where the ship is.
    pub position: Vec3,
    /// How fast it is going, in world units per second.
    pub speed: f32,
    /// Probes in contact over two: `0.0`, `0.5` or `1.0`.
    pub grounded: f32,
    /// Distance to the nearest spline sample.
    pub spline_distance: f32,
    /// Height above that sample along its own up axis. Negative is below the
    /// surface line.
    pub height_above_spline: f32,
    /// What the craft is carrying, if anything.
    ///
    /// Here so `--race --screenshot`'s own log answers "did the pad grant
    /// anything" without a debugger. It is the question every weapon capture
    /// starts with, and a HUD icon in a screenshot is a poor way to ask it.
    pub pickup: Option<oag_tables::weapons::Weapon>,
    /// How many projectiles are in the air.
    pub projectiles: usize,
}

impl Race {
    /// What the ship did, as of the last tick.
    #[must_use]
    pub fn telemetry(&self) -> Telemetry {
        let ship = self.ship();
        let position = ship.physics.body.position;
        let (spline_distance, height_above_spline) = self.sim.spline.nearest(position).map_or(
            (f32::NAN, f32::NAN),
            |(_, sample, distance)| {
                let up = (-Vec3::from_array(sample.down)).normalize_or_zero();
                (distance, (position - Vec3::from_array(sample.pos)).dot(up))
            },
        );

        Telemetry {
            tick: self.sim.world.tick,
            position,
            speed: ship.physics.body.linear_velocity.length(),
            pickup: ship.pickup.weapon,
            projectiles: self.sim.world.projectiles.live(),
            grounded: ship.physics.grounded,
            spline_distance,
            height_above_spline,
        }
    }

    /// Whether the ship is pointing back down the track.
    ///
    /// `dot(craft_forward, sample.tangent) < 0`. **The tangent, not the sample
    /// index** - `HANDOVER.md` records index order as unusable for this, because
    /// `Spline::from_track` concatenates paths in file order rather than travel
    /// order, and on a slow capture most windows step by zero. The dot product has
    /// neither failure mode and reads `+0.9999` against `-0.9999`.
    ///
    /// [`ai_order`] did not change that. It gives the *drivers* a travel-ordered
    /// view; this reads `Spline::nearest`, which is still the whole table in
    /// file order, and has to be - the player can be anywhere the track is,
    /// including on a branch the lap never drives.
    ///
    /// `false` when the ship is not near the spline at all, which is the safe way
    /// round: a spurious warning is worse than a missing one.
    #[must_use]
    pub fn wrong_way(&self) -> bool {
        let ship = self.ship();
        let Some((_, sample, _)) = self.sim.spline.nearest(ship.physics.body.position) else {
            return false;
        };
        let tangent = Vec3::from_array(sample.tangent).normalize_or_zero();
        ship.physics.body.forward().dot(tangent) < 0.0
    }

    /// What the HUD shows, as of the last tick.
    ///
    /// Separate from [`Telemetry`], which is a log line and carries diagnostics
    /// no player sees. The fields with no source yet are left at their "unknown"
    /// value rather than filled with a plausible number - `lap` and `place` read
    /// zero, and [`oag_hud`] omits a widget rather than claiming a value it does
    /// not have. See `docs/ui/hud.md`.
    #[must_use]
    pub fn readout(&self) -> oag_hud::Readout {
        let ship = self.ship();
        let race = self.sim.world.primary_race();
        // Zero still means "unknown", and a track with no closed ring still has
        // no lap counter - the widget is omitted rather than reading 1 of 3 on a
        // course that cannot tell.
        let counted = self.sim.course.is_some();
        oag_hud::Readout {
            speed_kmh: ship.physics.body.linear_velocity.length() * oag_fx::exhaust::SPEED_TO_KMH,
            speed_full_kmh: oag_hud::DEFAULT_SPEED_FULL_KMH,
            // The ship's own pool, which wall contact spends and absorbing a
            // pickup pays back into - `oag_physics::damage`. (This comment said
            // "nothing depletes this yet" until 2026-08-11, three subsystems
            // after it stopped being true.)
            shield: ship.physics.shield,
            shield_max: ship.handling.dimensions.shield,
            lap: if counted { race.lap } else { 0 },
            // A speed lap and a Zone run have no lap target. Zero is what the HUD
            // already reads as "unknown" and it omits the "of N" half.
            laps: race.laps_target.filter(|_| counted).unwrap_or(0),
            // The player's place in the field, from the same table the results
            // will read - [`Self::player_place`], slot 0 of [`Self::places`].
            //
            // Two gates, both meaning "there is no place to report" rather than
            // "the place is one". `counted`, because with no closed ring there is
            // no distance round the circuit to order the field by and
            // [`Self::places`] answers "everyone is first" - honest as a return
            // value and a lie on screen. And a field of **more than one craft**,
            // because a place is a position among opponents: a time trial, a speed
            // lap and a Zone run all grid the player alone, and so does a single
            // race on a track with no authored `Start Position` node. `1 / 1` is
            // arithmetic rather than a standing, and the widget group is omitted
            // the same way the lap group is on a track that cannot count.
            place: if counted && self.sim.world.ship_count > 1 {
                u32::from(self.player_place())
            } else {
                0
            },
            ships: u32::from(self.sim.world.ship_count),
            // The racing clock, not the raw tick: the original's clocks read
            // zero through the whole start-line countdown and start on the
            // release. `oag_race::race_clock_ticks` has the measurement.
            // The gates and animations that need the raw tick read
            // `world.tick` themselves.
            race_ticks: oag_race::race_clock_ticks(self.sim.world.tick),
            lap_ticks: if counted {
                race.lap_ticks(self.sim.world.tick)
            } else {
                oag_race::race_clock_ticks(self.sim.world.tick)
            },
            best_lap_ticks: race.best_lap_ticks,
            // Gated the same as `lap`/`laps` above: a track with no closed ring
            // has no lap history either.
            lap_splits: if counted {
                race.lap_splits
            } else {
                [None; oag_race::MAX_RECORDED_LAPS]
            },
            wrong_way: self.wrong_way(),
            zone: race.zone.into(),
            score: race.score,
            // The colour grade lives on `crate::Scene`, not on the
            // simulation, so the two draw sites that have both fill this in -
            // see `Scene::zone_stage`. Zero here means "no grade", which is what
            // rung zero already reads as: unnamed.
            zone_stage: 0,
            zone_next_in: None,
            pickup: ship.pickup.weapon,
            // The reticle, whatever it is doing. `Sight::visible` is what
            // decides whether anything reaches the screen, so this is passed
            // unconditionally rather than gated here - the brackets are watched
            // opening again after a target is lost, which a gate on "has a
            // target" would cut off.
            sight: Some(self.view.sight),
            shield_flashing: self.view.shield_flash_timer > 0.0,
            shield_absorbing: self.absorb_window_active(self.player_slot()),
            shield_blink_phase: self.view.shield_blink_timer,
            shield_flashing_whole: self.view.shield_flash_timer_whole > 0.0,
            shield_blink_phase_whole: self.view.shield_blink_timer_whole,
            energy_bar_delay_fraction: self.view.energy_bar_delay_fraction,
            thrust_chase_percent: self.view.thrust_chase_percent,
            pilot_assist: false,
            mode: self.sim.world.mode(),
            // Not this struct's to know: it needs the campaign cell and the
            // stored best, which live on `RaceStage`, one level up -
            // `RaceStage::draw_hud` fills it in after this method returns,
            // the same seam
            // `zone_stage`/`zone_next_in` already use above.
            time_trial_pace: None,
            head_to_head: self.head_to_head(),
            kill_tags: self.kill_tags(),
            kill_target: if self.sim.world.mode() == oag_race::Mode::Eliminator {
                self.sim.eliminator_kill_target
            } else {
                0
            },
        }
    }

    /// The Eliminator's kill column, top row first, in the order the original
    /// draws it - see [`oag_hud::kill_tags`]. Empty in every other mode, and
    /// for a slot with no team on record (a `Setup` built by hand).
    fn kill_tags(&self) -> Vec<oag_hud::KillTag> {
        let world = &self.sim.world;
        if world.mode() != oag_race::Mode::Eliminator {
            return Vec::new();
        }
        let slots: Vec<usize> = (0..world.ship_count as usize)
            .filter(|&slot| world.ships[slot].active)
            .collect();
        let kills: Vec<u32> = slots
            .iter()
            .map(|&slot| world.ships[slot].standing.kills)
            .collect();
        let player = self.player_slot();
        oag_hud::kill_tags::ranked(&kills)
            .into_iter()
            .filter_map(|row| {
                let slot = slots[row];
                Some(oag_hud::KillTag {
                    team: self.view.slot_teams.get(slot)?.clone(),
                    kills: kills[row],
                    player: slot == player,
                })
            })
            .collect()
    }

    /// Head2Head's gap readout: the unwrapped-progress difference between the
    /// player and the one opponent, and who leads. `None` outside Head2Head.
    ///
    /// The original takes `|craft+0xad0 - craft+0xad0|` over the pair
    /// (`FUN_0881d458`), and [`oag_race::Standing::distance`] is this
    /// engine's `craft+0xad0` with the same lap-1 wrap fix, in the same
    /// arc-length units. See `docs/ghidra/functions/psp-pulse-usa/head2head.md`.
    fn head_to_head(&self) -> Option<oag_hud::HeadToHead> {
        if self.sim.world.mode() != oag_race::Mode::Head2Head {
            return None;
        }
        let course = self.sim.course.as_ref()?;
        let world = &self.sim.world;
        let player = self.player_slot();
        let opponent = (0..world.ship_count as usize)
            .find(|&slot| slot != player && world.ships[slot].active)?;
        let gap = (world.ships[player].standing.distance(course)
            - world.ships[opponent].standing.distance(course))
        .abs();
        Some(oag_hud::HeadToHead {
            gap,
            player_leads: self.player_place() == 1,
        })
    }

    /// Advances the shield bar's post-hit flash for the next [`Self::readout`].
    ///
    /// Called once a tick, from [`Self::tick`], **after every writer of the
    /// pool has run** - the wall contact `oag_physics::step` itself applies,
    /// the weapon damage `oag_weapons::projectile::step` applies (which does
    /// not surface through `evaluated.shield`, unlike the contact case), and
    /// the perfect-zone recharge. Sampling any earlier would flash a weapon
    /// hit a tick late. Comparing the percentage itself, rather than keying
    /// off one writer's own edge, is also what makes this catch every source
    /// of a drop at once - the same thing the original's HUD-side comparison
    /// does, since `Hud_UpdateEnergyBar` has no idea what dropped the pool
    /// either.
    pub(super) fn advance_shield_flash(&mut self) {
        let ship = self.ship();
        let current =
            oag_physics::damage::percent(ship.physics.shield, ship.handling.dimensions.shield);
        let (timer, prev) = shield_flash_step(
            self.view.shield_flash_timer,
            self.view.shield_flash_prev,
            current,
            self.sim.dt,
        );
        self.view.shield_flash_timer = timer;
        self.view.shield_flash_prev = prev;
    }

    /// Advances the shield bar's blink accumulator for the next
    /// [`Self::readout`].
    ///
    /// Called once a tick, from [`Self::tick`], **after both
    /// [`Self::advance_shield_flash`] and [`Self::advance_absorb_bursts`]**
    /// have run - the blink gate reads both of their outputs
    /// ([`super::view::View::shield_flash_timer`] and
    /// [`Self::absorb_window_active`]) for this same tick, and reading
    /// either one stale would blink a tick early or late relative to the
    /// state the bar itself is drawn from.
    pub(super) fn advance_shield_blink(&mut self) {
        let ship = self.ship();
        let current =
            oag_physics::damage::percent(ship.physics.shield, ship.handling.dimensions.shield);
        let blinking = current <= oag_physics::damage::CRITICAL_PERCENT
            || self.view.shield_flash_timer > 0.0
            || self.absorb_window_active(self.player_slot());
        self.view.shield_blink_timer =
            shield_blink_step(self.view.shield_blink_timer, blinking, self.sim.dt);
    }

    /// Advances HD's twin of [`Self::advance_shield_flash`] and
    /// [`Self::advance_shield_blink`] for the next [`Self::readout`].
    ///
    /// `Hud_UpdateShieldReadout` (`0x000866c8`) is the same machine as
    /// Pulse's `Hud_UpdateEnergyBar` with one difference measured on the
    /// running original: the post-hit timer arms on a drop of the
    /// **truncated whole** percentage ([`shield_flash_step_whole`]). It then
    /// blinks on `percent <= 20`, the running timer, or the absorb window -
    /// `0x000cf490` is "within a second of `ship+0x6a80`", the stamp the pickup
    /// handler writes as the absorb feedback starts (hud-readouts.md). The
    /// second half of that condition, `ship+0x6958`, is not wired.
    pub(super) fn advance_shield_flash_whole(&mut self) {
        let ship = self.ship();
        let current =
            oag_physics::damage::percent(ship.physics.shield, ship.handling.dimensions.shield);
        let (timer, prev) = shield_flash_step_whole(
            self.view.shield_flash_timer_whole,
            self.view.shield_flash_prev_whole,
            current,
            self.sim.dt,
        );
        self.view.shield_flash_timer_whole = timer;
        self.view.shield_flash_prev_whole = prev;
        let blinking = current <= oag_physics::damage::CRITICAL_PERCENT
            || timer > 0.0
            || self.absorb_window_active(self.player_slot());
        self.view.shield_blink_timer_whole =
            shield_blink_step(self.view.shield_blink_timer_whole, blinking, self.sim.dt);
    }

    /// Advances 2048's `EnergyBarDelay` trail for the next [`Self::readout`].
    ///
    /// Called once a tick, from [`Self::tick`]. **Not `dt`-scaled** -
    /// `Hud_UpdateEnergyBar` applies a fixed `0.1` factor once per its own
    /// update, and this project runs a fixed 60 Hz tick throughout
    /// (`docs/architecture/determinism.md`), so a literal per-tick port is
    /// the closest reproduction available without a recovered original
    /// frame rate for this function specifically. See
    /// [`oag_hud::Readout::energy_bar_delay_fraction`] for the formula
    /// and the decompile it is read off.
    pub(super) fn advance_energy_bar_delay(&mut self) {
        let target = self.readout().shield_fraction();
        self.view.energy_bar_delay_fraction +=
            (target - self.view.energy_bar_delay_fraction) * ENERGY_BAR_DELAY_RATE;
    }

    /// Advances 2048's `ThrustBar` chase for the next [`Self::readout`].
    ///
    /// Called once a tick, from [`Self::tick`], with the fixed 60 Hz `dt`.
    /// The rates and the clamp are the original's; the target (the ship's
    /// thrust state, already `0..=100`) is **chosen, not measured** - see
    /// [`oag_hud::Readout::thrust_chase_percent`].
    pub(super) fn advance_thrust_chase(&mut self) {
        let target = self.ship().physics.thrust.clamp(0.0, 100.0);
        let dt = 1.0 / 60.0;
        let value = self.view.thrust_chase_percent;
        self.view.thrust_chase_percent = if value < target {
            (value + THRUST_CHASE_RISE_PER_SECOND * dt).min(target)
        } else {
            (value - THRUST_CHASE_FALL_PER_SECOND * dt).max(target)
        };
    }
}

/// `Hud_UpdateEnergyBar`'s own `0.1` - see
/// [`Race::advance_energy_bar_delay`].
const ENERGY_BAR_DELAY_RATE: f32 = 0.1;

/// 2048's `ThrustBar` rise rate: `100 * 1.4` percent per second, the
/// `0x3fb33333` the HUD update multiplies its `dt * 100` by.
const THRUST_CHASE_RISE_PER_SECOND: f32 = 140.0;

/// 2048's `ThrustBar` fall rate, percent per second: the bare `dt * 100`.
const THRUST_CHASE_FALL_PER_SECOND: f32 = 100.0;

/// The shield bar's post-hit flash, one tick of `Hud_UpdateEnergyBar`'s own
/// `hud+0x118`/`hud+0x11c` pair.
///
/// **Ported literally as an up-counter that wraps, not rewritten as a
/// down-counting timer.** The decompiled branch is `if (dropped ||
/// timer_running) { timer += dt; if (timer >= 1.0) timer = 0.0; }` - so once
/// the timer is already running, a *further* drop during the window changes
/// nothing, because `timer_running` alone already satisfies the `||`.
/// [shield.md]'s own prose calls this "re-armed by any further drop before it
/// expires", but that describes the *outcome* under sustained contact (every
/// tick re-tests the drop condition, so the window effectively never lapses
/// while the pool keeps falling), not a literal reset - the branch itself
/// never re-arms an already-running timer early. This follows the branch.
///
/// Returns `(next_timer, next_prev)`. `next_timer > 0.0` is "flashing this
/// frame"; `next_prev` is this tick's percentage, threaded back in as next
/// tick's "previous".
///
/// [shield.md]: ../../../../docs/ghidra/functions/psp-pulse-usa/shield.md#hud_updateenergybar-0x0881c638-tints-the-bar-from-a-20-threshold-not-a-gradient
fn shield_flash_step(timer: f32, prev: f32, current: f32, dt: f32) -> (f32, f32) {
    flash_step(timer, current, dt, current < prev)
}

/// [`shield_flash_step`] with HD's drop test: the percentages compared after
/// truncation to whole numbers, as `Hud_UpdateShieldReadout`'s `fctiwz` pair
/// does. A fall inside one whole percent arms nothing.
fn shield_flash_step_whole(timer: f32, prev: f32, current: f32, dt: f32) -> (f32, f32) {
    flash_step(timer, current, dt, current.trunc() < prev.trunc())
}

fn flash_step(timer: f32, current: f32, dt: f32, dropped: bool) -> (f32, f32) {
    let mut timer = timer;
    if dropped || timer > 0.0 {
        timer += dt;
        if timer >= 1.0 {
            timer = 0.0;
        }
    }
    (timer, current)
}

/// The shield bar's blink accumulator, one tick of `Hud_UpdateEnergyBar`'s
/// own `hud+0x1dc`.
///
/// **Freezes rather than resets when `blinking` is false** - the decompiled
/// branch only advances `hud+0x1dc` inside the blink condition at all, so a
/// tick where nothing blinks leaves the accumulator exactly where it was,
/// and the next blink resumes from that phase rather than starting fresh.
/// Wraps to `0.0` once it exceeds `1.0`, the same up-counting shape
/// [`shield_flash_step`] uses for `hud+0x11c`. See
/// `docs/ghidra/functions/psp-pulse-usa/shield.md#hud_updateenergybar-the-absorb-flash-2026-09-25`.
fn shield_blink_step(timer: f32, blinking: bool, dt: f32) -> f32 {
    if !blinking {
        return timer;
    }
    let timer = timer + dt;
    if timer > 1.0 { 0.0 } else { timer }
}

/// One telemetry line, for a log or a report.
#[must_use]
pub fn describe(telemetry: &Telemetry) -> String {
    format!(
        "tick {:>5}  speed {:>8.2}  grounded {:>3.1}  spline {:>8.2}  height {:>8.2}  at {:.1}\
         {}",
        telemetry.tick,
        telemetry.speed,
        telemetry.grounded,
        telemetry.spline_distance,
        telemetry.height_above_spline,
        telemetry.position,
        match (telemetry.pickup, telemetry.projectiles) {
            (None, 0) => String::new(),
            (held, count) => {
                let held = held.map_or("-", oag_tables::weapons::Weapon::as_type);
                format!("  holding {held}  in the air {count}")
            }
        },
    )
}

#[cfg(test)]
mod shield_flash_tests {
    use super::{shield_flash_step, shield_flash_step_whole};

    /// One 60 Hz tick, the same fixed step [`super::Race`] runs at.
    const DT: f32 = 1.0 / 60.0;

    /// A drop arms the flash on the very tick it happens.
    #[test]
    fn a_drop_arms_the_flash_immediately() {
        let (timer, prev) = shield_flash_step(0.0, 100.0, 60.0, DT);
        assert!(timer > 0.0);
        assert_eq!(prev, 60.0);
    }

    /// No drop, and no flash already running, leaves the timer at zero - a
    /// rising or level percentage never arms it.
    #[test]
    fn holding_or_rising_does_not_arm_it() {
        let (timer, _) = shield_flash_step(0.0, 60.0, 60.0, DT);
        assert_eq!(timer, 0.0);
        let (timer, _) = shield_flash_step(0.0, 60.0, 80.0, DT);
        assert_eq!(timer, 0.0);
    }

    /// The window is roughly a second: still flashing at half a second in,
    /// expired past 1.1 - bounds rather than an exact tick count, since
    /// accumulating `1.0 / 60.0` sixty times in `f32` may land either side of
    /// `1.0`.
    #[test]
    fn the_window_is_about_one_second() {
        // Tick 0 is the drop itself; every tick after holds the percentage
        // level, so only the timer's own "already running" arm keeps it
        // going - exactly what a single isolated hit looks like.
        let (mut timer, mut prev) = shield_flash_step(0.0, 100.0, 60.0, DT);
        for _ in 0..29 {
            (timer, prev) = shield_flash_step(timer, prev, prev, DT);
        }
        assert!(timer > 0.0, "still flashing half a second after the drop");

        for _ in 0..40 {
            (timer, prev) = shield_flash_step(timer, prev, prev, DT);
        }
        assert_eq!(
            timer, 0.0,
            "expired: 1.1s have passed since the drop with no further one"
        );
    }

    /// HD arms on a drop of the truncated whole percentage: 60.9 to 60.2 is a
    /// fall inside one whole percent and arms nothing, 60.2 to 59.9 crosses
    /// one and does. Pulse's step arms on both. Measured on the running
    /// original through the HUD's own timer, three reps each.
    #[test]
    fn hd_arms_only_on_a_whole_percent_drop() {
        let (timer, _) = shield_flash_step_whole(0.0, 60.9, 60.2, DT);
        assert_eq!(timer, 0.0, "inside one whole percent");
        let (timer, _) = shield_flash_step_whole(0.0, 60.2, 59.9, DT);
        assert!(timer > 0.0, "across one");
        let (timer, _) = shield_flash_step(0.0, 60.9, 60.2, DT);
        assert!(timer > 0.0, "Pulse's float compare arms on the same fall");
        let (timer, _) = shield_flash_step_whole(0.0, 30.0, 40.0, DT);
        assert_eq!(timer, 0.0, "a rise arms nothing");
    }

    /// A further drop mid-window changes nothing, per the branch rather than
    /// the prose - see [`shield_flash_step`]'s own doc comment.
    #[test]
    fn a_further_drop_mid_window_does_not_reset_it() {
        let (timer_a, prev_a) = shield_flash_step(0.0, 100.0, 60.0, DT);
        let (timer_b, _) = shield_flash_step(timer_a, prev_a, 40.0, DT);
        // Both branches of the `||` are satisfied on this second tick, and
        // the accumulation is the same either way: one `dt` further on from
        // `timer_a`, not reset to it.
        assert_eq!(timer_b, timer_a + DT);
    }
}

#[cfg(test)]
mod shield_blink_tests {
    use super::shield_blink_step;

    /// One 60 Hz tick, the same fixed step [`super::Race`] runs at.
    const DT: f32 = 1.0 / 60.0;

    /// Not blinking: the accumulator does not move at all.
    #[test]
    fn not_blinking_leaves_the_timer_alone() {
        assert_eq!(shield_blink_step(0.4, false, DT), 0.4);
    }

    /// Blinking: the accumulator advances by `dt`, same as the flash's own.
    #[test]
    fn blinking_advances_by_dt() {
        assert_eq!(shield_blink_step(0.4, true, DT), 0.4 + DT);
    }

    /// Freezes rather than resets: a run that stops blinking mid-cycle and
    /// starts again later resumes from where it left off, not from zero -
    /// the discriminating behaviour against [`shield_flash_step`], which
    /// this shares the up-counting shape with but not the reset rule.
    #[test]
    fn a_gap_in_blinking_does_not_reset_the_phase() {
        let mut timer = 0.0;
        for _ in 0..10 {
            timer = shield_blink_step(timer, true, DT);
        }
        let frozen = timer;
        for _ in 0..20 {
            timer = shield_blink_step(timer, false, DT);
        }
        assert_eq!(timer, frozen, "not blinking must not move the accumulator");
        timer = shield_blink_step(timer, true, DT);
        assert_eq!(
            timer,
            frozen + DT,
            "resumes from the frozen phase, not zero"
        );
    }

    /// Wraps to zero once it exceeds one second, not at exactly one second -
    /// `1.0 < accum`, not `<=`, in the decompile. A `0.3` step lands the
    /// crossing well clear of `f32` rounding at the boundary, unlike
    /// accumulating sixty `1.0 / 60.0` ticks would.
    #[test]
    fn wraps_past_one_second() {
        let mut timer = 0.0;
        for _ in 0..3 {
            timer = shield_blink_step(timer, true, 0.3);
        }
        assert!((0.89..0.91).contains(&timer), "0.9s in: {timer}");
        timer = shield_blink_step(timer, true, 0.3);
        assert_eq!(timer, 0.0, "1.2s crosses 1.0 and wraps: {timer}");
    }
}
