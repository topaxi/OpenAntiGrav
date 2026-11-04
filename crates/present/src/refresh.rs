//! What the display can show, learned from the monitor's report and from frames that prove it.
//!
//! A reported refresh can be missing (X11 under Xvfb), and a wall-clock interval alone cannot tell
//! a slow display from a slow GPU. [`Output`] accepts an interval as evidence of the display only
//! when the present mode holds frames back to the refresh **and** the busy work (the loop's own
//! plus the GPU's timed cost, the rest being the swapchain wait) would have fit in under half of
//! it: a loop that is busy for more can miss every second vblank and read as a
//! display at half the rate. A frame that fails a test says nothing; no accepted frames means no
//! answer, and no answer means no clamp (`docs/rendering/dynamic-resolution.md`, "The target is
//! held to what is shown").

/// Gated frames one estimate is drawn from: one and a half seconds at 60 Hz.
pub const WINDOW: usize = 90;
/// Widest `(p90 - p10) / median` of a window still read as a fixed refresh. Wider is a
/// variable-refresh output or a loop that is not output-bound, and gives no answer.
pub const MAX_SPREAD: f32 = 0.08;
/// Gated frames in a row a full window must stay unsteady before a rate is withdrawn: a stutter
/// is not a variable-refresh output, and withdrawing the rate would let the target climb again.
pub const WITHDRAW_AFTER: usize = 2 * WINDOW;
/// The share of the interval the busy work (`interval - wait + gpu`) must be at most. Work that
/// fits in under half of it would have made the earlier vblank, so the interval is not a doubled
/// one; the margin below a half is for GPU work the timer does not see. It also means the frame
/// spent over half of it waiting, and the GPU was not the long pole.
pub const MAX_BUSY_SHARE: f32 = 0.4;
/// Below this an interval is not a refresh any display has: a stalled frame.
const MIN_HZ: f32 = 20.0;

/// One frame as the policy sees it, in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Wall clock since the previous frame.
    pub interval: f32,
    /// Acquire plus present, the calls a held-back output blocks in.
    pub wait: f32,
    /// The GPU's own timed cost for the frame.
    pub gpu: f32,
    /// The present mode holds a frame back until the next refresh (`Fifo`). Under any other mode
    /// an interval is the loop's own pacing, not the output's.
    pub held_to_refresh: bool,
}

impl Frame {
    fn proves_the_output(self) -> bool {
        let finite = self.interval.is_finite() && self.wait.is_finite() && self.gpu.is_finite();
        finite
            && self.held_to_refresh
            && self.interval > 0.0
            && self.interval < 1.0 / MIN_HZ
            && self.wait >= 0.0
            && self.gpu >= 0.0
            && (self.interval - self.wait) + self.gpu <= self.interval * MAX_BUSY_SHARE
    }
}

/// The measured refresh: a window of gated intervals and the last rate they agreed on.
#[derive(Debug, Clone)]
pub struct Output {
    ring: [f32; WINDOW],
    filled: usize,
    next: usize,
    unsteady: usize,
    hz: Option<u32>,
}

impl Default for Output {
    fn default() -> Self {
        Self::new()
    }
}

impl Output {
    /// Nothing measured.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ring: [0.0; WINDOW],
            filled: 0,
            next: 0,
            unsteady: 0,
            hz: None,
        }
    }

    /// Forgets everything: the evidence belongs to a state that no longer holds (another monitor,
    /// a reconfigured surface, a new present mode, a new race).
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// One frame. Returns the rate when this frame changed what is known.
    ///
    /// A frame that proves nothing is ignored and the last agreed rate stays: once the clamp
    /// holds the scale the GPU fills the budget and the wait shrinks, and dropping the rate then
    /// would let the target climb and the evidence return, forever. [`Output::reset`] is what
    /// ends a rate, when the state it was measured in changes.
    pub fn observe(&mut self, frame: Frame) -> Option<Option<u32>> {
        if !frame.proves_the_output() {
            return None;
        }
        self.ring[self.next] = frame.interval;
        self.next = (self.next + 1) % WINDOW;
        self.filled = (self.filled + 1).min(WINDOW);
        if self.filled < WINDOW {
            return None;
        }
        let Some(agreed) = self.agreed() else {
            self.unsteady += 1;
            if self.unsteady >= WITHDRAW_AFTER && self.hz.is_some() {
                self.hz = None;
                return Some(None);
            }
            return None;
        };
        self.unsteady = 0;
        if self.hz == Some(agreed) {
            return None;
        }
        self.hz = Some(agreed);
        Some(Some(agreed))
    }

    /// The refresh the gated frames agree on, or `None`.
    #[must_use]
    pub fn hz(&self) -> Option<u32> {
        self.hz
    }

    fn agreed(&self) -> Option<u32> {
        let mut sorted = self.ring;
        sorted.sort_by(f32::total_cmp);
        let at = |share: f32| sorted[((WINDOW - 1) as f32 * share) as usize];
        let median = at(0.5);
        if (at(0.9) - at(0.1)) / median > MAX_SPREAD {
            return None;
        }
        Some((1.0 / median).round() as u32)
    }
}

/// What the display can show: the monitor's report and the measurement.
#[derive(Debug, Clone, Default)]
pub struct Display {
    reported_millihertz: Option<u32>,
    measured: Output,
}

impl Display {
    /// Nothing reported, nothing measured.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The current monitor's report, in millihertz, as the platform gave it. A report no display
    /// has (a driver's 0 or 1 for "hardware default") is no report. Any change of the report
    /// drops the measurement: it was taken on another output or mode.
    pub fn report(&mut self, millihertz: Option<u32>) {
        let plausible = millihertz.filter(|mhz| ((mhz + 500) / 1000) as f32 >= MIN_HZ);
        if plausible != self.reported_millihertz {
            self.reported_millihertz = plausible;
            self.measured.reset();
        }
    }

    /// Drops the measurement; see [`Output::reset`]. The report stays.
    pub fn reset(&mut self) {
        self.measured.reset();
    }

    /// Feeds the measurement; see [`Output::observe`].
    pub fn observe(&mut self, frame: Frame) -> Option<Option<u32>> {
        self.measured.observe(frame)
    }

    /// The most the display can show, or `None` when nothing says.
    ///
    /// The report is a floor the measurement may raise but not lower: a measurement below a
    /// report is a loop that missed vblanks or a limiter outside the game, and believing it would
    /// hold a faster output to the lower rate for the session. A wrong high report costs what an
    /// unclamped target costs today; a wrong low clamp costs pixels.
    #[must_use]
    pub fn hz(&self) -> Option<u32> {
        let reported = self.reported_millihertz.map(|mhz| (mhz + 500) / 1000);
        match (self.measured.hz(), reported) {
            (Some(m), Some(r)) => Some(m.max(r)),
            (m, r) => m.or(r),
        }
    }
}

/// The rate the loop is held to by the frame limiter or the output, whichever is lower.
#[must_use]
pub fn held_hz(limiter: Option<u32>, display: &Display) -> Option<u32> {
    match (limiter, display.hz()) {
        (Some(limit), Some(shown)) => Some(limit.min(shown)),
        (limit, shown) => limit.or(shown),
    }
}

/// The resolution target held under the limiter and the output: aiming above a rate the loop is
/// not allowed to produce, or the display cannot show, would drop the resolution for frames that
/// are never presented.
#[must_use]
pub fn held_target(
    target: crate::drs::Target,
    limiter: Option<u32>,
    display: &Display,
) -> crate::drs::Target {
    target.at_most(held_hz(limiter, display))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drs::Target;

    fn frame(interval: f32, wait: f32, gpu: f32) -> Frame {
        Frame {
            interval,
            wait,
            gpu,
            held_to_refresh: true,
        }
    }

    /// A loop the output holds back: most of the interval waiting, a quarter busy.
    fn held(hz: f32) -> Frame {
        let interval = 1.0 / hz;
        frame(interval, interval * 0.8, interval * 0.15)
    }

    fn feed(display: &mut Display, frame: Frame, n: usize) {
        for _ in 0..n {
            display.observe(frame);
        }
    }

    fn bound(display: &Display, limiter: Option<u32>) -> Target {
        held_target("90".parse::<Target>().unwrap(), limiter, display)
    }

    #[test]
    fn the_lower_of_the_limiter_and_the_output_holds_the_loop() {
        let mut display = Display::new();
        assert_eq!(held_hz(None, &display), None);
        assert_eq!(held_hz(Some(120), &display), Some(120));
        display.report(Some(59_980));
        assert_eq!(held_hz(None, &display), Some(60));
        assert_eq!(held_hz(Some(120), &display), Some(60));
        assert_eq!(held_hz(Some(30), &display), Some(30));
        assert_eq!(bound(&display, Some(30)).hz(), Some(30));
    }

    #[test]
    fn a_60_hz_output_with_a_fast_gpu_holds_a_90_target_to_60() {
        let mut display = Display::new();
        feed(&mut display, held(60.0), WINDOW);
        assert_eq!(display.hz(), Some(60));
        assert_eq!(bound(&display, None).hz(), Some(60));
    }

    #[test]
    fn the_clamp_is_what_holds_it() {
        let display = Display::new();
        assert_eq!(bound(&display, None).hz(), Some(90));
    }

    #[test]
    fn a_limiter_held_loop_is_no_evidence() {
        // The game's own limiter sleeps outside the swapchain calls: wait 2 %, GPU 10 %.
        let mut display = Display::new();
        feed(&mut display, frame(1.0 / 60.0, 0.0003, 0.0017), WINDOW * 2);
        assert_eq!(display.hz(), None);
    }

    #[test]
    fn a_slow_gpu_is_no_evidence() {
        let mut display = Display::new();
        feed(&mut display, frame(1.0 / 60.0, 0.001, 0.0155), WINDOW * 2);
        assert_eq!(display.hz(), None);
        // A GPU filling the interval is no evidence even when the wait is the long call.
        feed(&mut display, frame(1.0 / 60.0, 0.014, 0.0155), WINDOW * 2);
        assert_eq!(display.hz(), None);
    }

    #[test]
    fn a_loop_that_misses_every_second_vblank_does_not_read_as_a_slow_display() {
        // 144 Hz panel, CPU 5 ms and GPU 5 ms: 13.9 ms intervals, each wait 8.9 ms.
        let p = 1.0 / 144.0;
        let mut display = Display::new();
        display.report(Some(143_981));
        feed(
            &mut display,
            frame(2.0 * p, 2.0 * p - 0.005, 0.005),
            WINDOW * 2,
        );
        assert_eq!(display.hz(), Some(144));
        // Without a report it gives no answer at all.
        let mut bare = Display::new();
        feed(
            &mut bare,
            frame(2.0 * p, 2.0 * p - 0.005, 0.005),
            WINDOW * 2,
        );
        assert_eq!(bare.hz(), None);
    }

    #[test]
    fn a_run_of_stalls_is_not_a_5_hz_display() {
        let mut display = Display::new();
        feed(&mut display, frame(0.2, 0.19, 0.005), WINDOW * 2);
        assert_eq!(display.hz(), None);
    }

    #[test]
    fn a_present_mode_that_does_not_hold_frames_back_is_no_evidence() {
        // Mailbox on a 60 Hz panel runs at 240 with a short acquire wait.
        let mut display = Display::new();
        display.report(Some(59_980));
        let mailbox = Frame {
            held_to_refresh: false,
            ..frame(1.0 / 240.0, 0.0033, 0.0004)
        };
        feed(&mut display, mailbox, WINDOW * 2);
        assert_eq!(display.hz(), Some(60));
        assert_eq!(bound(&display, None).hz(), Some(60));
    }

    #[test]
    fn a_144_hz_panel_keeps_144() {
        let mut display = Display::new();
        display.report(Some(143_981));
        assert_eq!(display.hz(), Some(144));
        feed(&mut display, held(144.0), WINDOW);
        assert_eq!(display.hz(), Some(144));
        assert_eq!(bound(&display, None).hz(), Some(90));
        // A target above the panel is the case that is held.
        assert_eq!(
            "165".parse::<Target>().unwrap().at_most(display.hz()).hz(),
            Some(144)
        );
    }

    #[test]
    fn the_measurement_raises_a_report_but_never_lowers_it() {
        let mut display = Display::new();
        display.report(Some(60_000));
        feed(&mut display, held(144.0), WINDOW);
        assert_eq!(display.hz(), Some(144));
        let mut lower = Display::new();
        lower.report(Some(90_000));
        feed(&mut lower, held(60.0), WINDOW);
        assert_eq!(lower.hz(), Some(90));
    }

    #[test]
    fn a_report_alone_is_the_rate() {
        let mut display = Display::new();
        display.report(Some(90_000));
        assert_eq!(display.hz(), Some(90));
        display.report(None);
        assert_eq!(display.hz(), None);
    }

    #[test]
    fn an_implausible_report_is_no_report() {
        for mhz in [0, 400, 1000, 19_000] {
            let mut display = Display::new();
            display.report(Some(mhz));
            assert_eq!(display.hz(), None, "{mhz} mHz");
            assert_eq!(bound(&display, None).hz(), Some(90));
        }
    }

    #[test]
    fn a_window_moved_to_a_faster_monitor_follows_its_report() {
        let mut display = Display::new();
        display.report(Some(60_000));
        feed(&mut display, held(60.0), WINDOW);
        assert_eq!(display.hz(), Some(60));
        display.report(Some(144_000));
        assert_eq!(display.hz(), Some(144));
    }

    #[test]
    fn a_report_change_drops_a_rate_measured_on_the_old_output() {
        let mut display = Display::new();
        feed(&mut display, held(60.0), WINDOW);
        assert_eq!(display.hz(), Some(60));
        display.report(Some(90_000));
        display.report(None);
        assert_eq!(display.hz(), None);
    }

    #[test]
    fn a_reset_forgets_the_measurement_and_keeps_the_report() {
        let mut display = Display::new();
        display.report(Some(30_000));
        feed(&mut display, held(60.0), WINDOW);
        assert_eq!(display.hz(), Some(60));
        display.reset();
        assert_eq!(display.hz(), Some(30));
    }

    #[test]
    fn a_variable_refresh_output_gives_no_answer() {
        let mut display = Display::new();
        feed(&mut display, held(60.0), WINDOW);
        assert_eq!(display.hz(), Some(60));
        for i in 0..WINDOW * 4 {
            display.observe(held(if i % 2 == 0 { 48.0 } else { 90.0 }));
        }
        assert_eq!(display.hz(), None);
    }

    #[test]
    fn a_stutter_does_not_withdraw_the_rate() {
        let mut display = Display::new();
        feed(&mut display, held(60.0), WINDOW);
        for i in 0..WITHDRAW_AFTER - 1 {
            display.observe(held(if i % 2 == 0 { 40.0 } else { 90.0 }));
        }
        assert_eq!(display.hz(), Some(60));
    }

    #[test]
    fn the_rate_is_kept_when_the_evidence_disappears() {
        let mut display = Display::new();
        feed(&mut display, held(60.0), WINDOW);
        // The clamp holds the scale; the GPU now fills the interval and the wait is gone.
        feed(&mut display, frame(1.0 / 60.0, 0.0005, 0.015), WINDOW * 3);
        assert_eq!(display.hz(), Some(60));
    }

    #[test]
    fn a_non_finite_frame_does_not_move_it() {
        let mut display = Display::new();
        feed(&mut display, held(60.0), WINDOW);
        display.observe(frame(f32::NAN, 0.0, 0.0));
        display.observe(frame(0.5, 0.45, 0.01));
        assert_eq!(display.hz(), Some(60));
    }

    #[test]
    fn a_change_of_mode_lands_on_the_new_rate() {
        let mut display = Display::new();
        feed(&mut display, held(60.0), WINDOW);
        feed(&mut display, held(120.0), WINDOW * 2);
        assert_eq!(display.hz(), Some(120));
    }
}
