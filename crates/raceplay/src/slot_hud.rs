//! The HUD state one craft's screen animates from, kept per slot.
//!
//! A full-screen HUD reads one craft's, the primary slot's. A split-screen race
//! draws a HUD per view, and each of those needs its own flash timers and bar
//! chases, so the state is an array indexed by slot rather than a handful of
//! `RaceView` fields. Every field is render-side and out of the hash.
//!
//! The primary slot's entry is exactly what the fields it replaced were, advanced
//! at the same point of the tick; the other slots' entries are advanced only while
//! a split is drawn ([`Race::advance_slot_huds`]), so a race with one view does
//! what it always did.

use super::*;

/// The shield bar's and the other bars' memory, plus a split view's reticle.
#[derive(Clone, Debug, Default)]
pub(super) struct SlotHud {
    /// `Hud_UpdateEnergyBar`'s `hud+0x118`: the shield percentage as of the
    /// previous tick. Seeded `0.0`.
    pub(super) shield_flash_prev: f32,
    /// `Hud_UpdateEnergyBar`'s `hud+0x11c`, the post-hit flash accumulator:
    /// `0.0` is "not flashing", it counts up while running and wraps to `0.0` at
    /// `1.0`. See [`shield_flash_step`].
    pub(super) shield_flash_timer: f32,
    /// `Hud_UpdateEnergyBar`'s `hud+0x1dc`, the blink accumulator: freezes
    /// rather than resets when nothing blinks. See [`shield_blink_step`].
    pub(super) shield_blink_timer: f32,
    /// HD's `hud+0x10c`: the previous tick's percentage, for [`Self::shield_flash_timer_whole`].
    pub(super) shield_flash_prev_whole: f32,
    /// HD's `hud+0x110`, the post-hit timer, armed by a drop of the truncated
    /// whole percentage. See [`shield_flash_step_whole`].
    pub(super) shield_flash_timer_whole: f32,
    /// HD's `hud+0x1e8`, the flash accumulator gated on
    /// [`Self::shield_flash_timer_whole`].
    pub(super) shield_blink_timer_whole: f32,
    /// 2048's `EnergyBarDelay`, the lagging shield fraction
    /// ([`oag_hud::Readout::energy_bar_delay_fraction`]).
    pub(super) energy_bar_delay_fraction: f32,
    /// 2048's `ThrustBar` chase in percent
    /// ([`oag_hud::Readout::thrust_chase_percent`]).
    pub(super) thrust_chase_percent: f32,
    /// Pilot Assist's HUD indicator timers, off this slot's own assist state.
    pub(super) assist_indicator: oag_hud::assist::Indicator,
    /// The reticle of a split-screen view on a slot that is not the primary.
    /// **Display only**: the primary's sight is `RaceView::sight`, and the
    /// weapons code reads that one, so a second person's fire does not wait on
    /// this lock. Unused in the primary's entry.
    pub(super) sight: sight::Sight,
    /// What [`Self::sight`] said last tick.
    pub(super) sight_state: sight::State,
    /// This view's HUD message lines. One per slot so that each person's own
    /// perfect-lap, lap-record and final-lap lines show in their own view; a
    /// slot nobody flies never has one raised. Render-side, out of the hash.
    pub(super) messages: oag_hud::messages::MessageBoard,
    /// This slot's lap-crossing lines, or `None` for a title with no message
    /// slots. Set for every slot by `Race::watch_laps`, fed only for the slots a
    /// person on this machine flies.
    pub(super) lap_watch: Option<super::lap_messages::LapWatch>,
}

/// 2048's `EnergyBarDelay` factor, `Hud_UpdateEnergyBar`'s own `0.1`, applied
/// once per tick and not `dt`-scaled: a literal per-tick port at the fixed 60 Hz.
const ENERGY_BAR_DELAY_RATE: f32 = 0.1;

/// 2048's `ThrustBar` rise rate: `100 * 1.4` percent per second, the
/// `0x3fb33333` the HUD update multiplies its `dt * 100` by.
const THRUST_CHASE_RISE_PER_SECOND: f32 = 140.0;

/// 2048's `ThrustBar` fall rate, percent per second: the bare `dt * 100`.
const THRUST_CHASE_FALL_PER_SECOND: f32 = 100.0;

impl SlotHud {
    /// One tick of every bar, in the order the original's update runs them:
    /// the flash, then the blink that reads it, HD's twin of the pair, 2048's
    /// lag and then its thrust chase.
    ///
    /// `percent` is the shield pool as a percentage, `fraction` the same as
    /// `0..=1`, `thrust` the craft's thrust state clamped to `0..=100`, and
    /// `absorbing` whether the pickup-absorb window is open. **After every writer
    /// of the shield pool this tick has run**, so a hit flashes on the tick it
    /// lands rather than one late.
    ///
    /// The thrust chase's target is **chosen, not measured**; its rates are the
    /// original's. See [`oag_hud::Readout::thrust_chase_percent`].
    pub(super) fn advance(
        &mut self,
        percent: f32,
        fraction: f32,
        thrust: f32,
        absorbing: bool,
        dt: f32,
    ) {
        let (timer, prev) =
            shield_flash_step(self.shield_flash_timer, self.shield_flash_prev, percent, dt);
        self.shield_flash_timer = timer;
        self.shield_flash_prev = prev;

        let blinking = percent <= oag_physics::damage::CRITICAL_PERCENT || timer > 0.0 || absorbing;
        self.shield_blink_timer = shield_blink_step(self.shield_blink_timer, blinking, dt);

        let (timer, prev) = shield_flash_step_whole(
            self.shield_flash_timer_whole,
            self.shield_flash_prev_whole,
            percent,
            dt,
        );
        self.shield_flash_timer_whole = timer;
        self.shield_flash_prev_whole = prev;
        let blinking = percent <= oag_physics::damage::CRITICAL_PERCENT || timer > 0.0 || absorbing;
        self.shield_blink_timer_whole =
            shield_blink_step(self.shield_blink_timer_whole, blinking, dt);

        self.energy_bar_delay_fraction +=
            (fraction - self.energy_bar_delay_fraction) * ENERGY_BAR_DELAY_RATE;

        let tick = 1.0 / 60.0;
        let value = self.thrust_chase_percent;
        self.thrust_chase_percent = if value < thrust {
            (value + THRUST_CHASE_RISE_PER_SECOND * tick).min(thrust)
        } else {
            (value - THRUST_CHASE_FALL_PER_SECOND * tick).max(thrust)
        };
    }
}

impl Race {
    /// One tick of the HUD bars of every slot a view is drawn for: the primary
    /// always, and with a split the other viewed slots too.
    ///
    /// Called from [`Self::tick`] at the place the primary's four separate
    /// advances used to be, after the shield pool's last writer and after
    /// [`Self::advance_absorb_bursts`].
    pub(super) fn advance_slot_huds(&mut self) {
        let primary = self.player_slot();
        let viewed = self.viewport_count();
        for slot in 0..MAX_SHIPS {
            if slot != primary && slot >= viewed {
                continue;
            }
            let ship = &self.sim.world.ships[slot];
            let pool = ship.physics.shield;
            let max = ship.handling.dimensions.shield;
            let percent = oag_physics::damage::percent(pool, max);
            let fraction = oag_hud::shield_fraction(pool, max);
            let thrust = ship.physics.thrust.clamp(0.0, 100.0);
            let absorbing = self.absorb_window_active(slot);
            let acting = ship.physics.pilot_assist.acting;
            let (shown, dt) = (self.pilot_assist_shown(), self.sim.dt);
            let hud = &mut self.view.huds[slot];
            hud.advance(percent, fraction, thrust, absorbing, dt);
            hud.assist_indicator.advance(shown, acting, dt);
        }
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
pub(super) fn shield_flash_step(timer: f32, prev: f32, current: f32, dt: f32) -> (f32, f32) {
    flash_step(timer, current, dt, current < prev)
}

/// [`shield_flash_step`] with HD's drop test: the percentages compared after
/// truncation to whole numbers, as `Hud_UpdateShieldReadout`'s `fctiwz` pair
/// does. A fall inside one whole percent arms nothing.
pub(super) fn shield_flash_step_whole(timer: f32, prev: f32, current: f32, dt: f32) -> (f32, f32) {
    flash_step(timer, current, dt, current.trunc() < prev.trunc())
}

pub(super) fn flash_step(timer: f32, current: f32, dt: f32, dropped: bool) -> (f32, f32) {
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
pub(super) fn shield_blink_step(timer: f32, blinking: bool, dt: f32) -> f32 {
    if !blinking {
        return timer;
    }
    let timer = timer + dt;
    if timer > 1.0 { 0.0 } else { timer }
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
