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
        let (spline_distance, height_above_spline) =
            self.spline
                .nearest(position)
                .map_or((f32::NAN, f32::NAN), |(_, sample, distance)| {
                    let up = (-Vec3::from_array(sample.down)).normalize_or_zero();
                    (distance, (position - Vec3::from_array(sample.pos)).dot(up))
                });

        Telemetry {
            tick: self.world.tick,
            position,
            speed: ship.physics.body.linear_velocity.length(),
            pickup: ship.pickup.weapon,
            projectiles: self.world.projectiles.live(),
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
        let Some((_, sample, _)) = self.spline.nearest(ship.physics.body.position) else {
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
    /// zero, and [`crate::hud`] omits a widget rather than claiming a value it does
    /// not have. See `docs/ui/hud.md`.
    #[must_use]
    pub fn readout(&self) -> crate::hud::Readout {
        let ship = self.ship();
        let race = &self.world.race;
        // Zero still means "unknown", and a track with no closed ring still has
        // no lap counter - the widget is omitted rather than reading 1 of 3 on a
        // course that cannot tell.
        let counted = self.course.is_some();
        crate::hud::Readout {
            speed_kmh: ship.physics.body.linear_velocity.length()
                * oag_render::exhaust::SPEED_TO_KMH,
            speed_full_kmh: crate::hud::DEFAULT_SPEED_FULL_KMH,
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
            place: if counted && self.world.ship_count > 1 {
                u32::from(self.player_place())
            } else {
                0
            },
            ships: u32::from(self.world.ship_count),
            race_ticks: self.world.tick,
            lap_ticks: if counted {
                race.lap_ticks(self.world.tick)
            } else {
                self.world.tick
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
            // The colour grade lives on `crate::race::Scene`, not on the
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
            sight: Some(self.sight),
            shield_flashing: self.shield_flash_timer > 0.0,
        }
    }

    /// Advances the shield bar's post-hit flash for the next [`Self::readout`].
    ///
    /// Called once a tick, from [`Self::tick`], **after every writer of the
    /// pool has run** - the wall contact `oag_physics::step` itself applies,
    /// the weapon damage `oag_gameplay::projectile::step` applies (which does
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
            self.shield_flash_timer,
            self.shield_flash_prev,
            current,
            self.dt,
        );
        self.shield_flash_timer = timer;
        self.shield_flash_prev = prev;
    }
}

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
    let dropped = current < prev;
    let mut timer = timer;
    if dropped || timer > 0.0 {
        timer += dt;
        if timer >= 1.0 {
            timer = 0.0;
        }
    }
    (timer, current)
}

#[cfg(test)]
mod shield_flash_tests {
    use super::shield_flash_step;

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
