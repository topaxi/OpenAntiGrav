//! What the display can show, learned from the monitor's report and from frames that prove it.
//!
//! A reported refresh cannot always be believed (gamescope's output is a virtual monitor whose
//! mode is the compositor's, not the panel's), and a wall-clock interval alone cannot tell a
//! slow display from a slow GPU. [`Output`] accepts an interval as evidence of the display only
//! when the frame spent most of it waiting in the swapchain **and** the GPU's own timed cost was
//! a small part of it. A frame that fails either test says nothing, so a slow GPU never reads as
//! a slow display; no accepted frames means no answer, and no answer means no clamp
//! (`docs/rendering/dynamic-resolution.md`, "The target is held to what is shown").

/// Gated frames one estimate is drawn from: one and a half seconds at 60 Hz.
pub const WINDOW: usize = 90;
/// The share of the interval the swapchain wait must be at least.
pub const MIN_WAIT_SHARE: f32 = 0.5;
/// The share of the interval the timed GPU cost must be at most.
pub const MAX_GPU_SHARE: f32 = 0.5;
/// Widest `(p90 - p10) / median` of a window still read as a fixed refresh. Wider is a
/// variable-refresh output or a loop that is not output-bound, and gives no answer.
pub const MAX_SPREAD: f32 = 0.08;
/// Gated frames in a row a full window must stay unsteady before a rate is withdrawn: a stutter
/// is not a variable-refresh output, and withdrawing the rate would let the target climb again.
pub const WITHDRAW_AFTER: usize = 2 * WINDOW;
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
}

impl Frame {
    fn proves_the_output(self) -> bool {
        let finite = self.interval.is_finite() && self.wait.is_finite() && self.gpu.is_finite();
        finite
            && self.interval > 0.0
            && self.interval < 1.0 / MIN_HZ
            && self.wait >= self.interval * MIN_WAIT_SHARE
            && self.gpu >= 0.0
            && self.gpu <= self.interval * MAX_GPU_SHARE
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

    /// One frame. Returns the rate when this frame changed what is known.
    ///
    /// A frame that proves nothing is ignored and the last agreed rate stays: once the clamp
    /// holds the scale the GPU fills the budget and the wait shrinks, and dropping the rate then
    /// would let the target climb and the evidence return, forever.
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

/// What the display can show: the monitor's report, if it is believed, and the measurement.
#[derive(Debug, Clone)]
pub struct Display {
    reported_millihertz: Option<u32>,
    trust_report: bool,
    measured: Output,
}

impl Display {
    /// `trust_report` is false where the monitor is virtual (gamescope).
    #[must_use]
    pub fn new(trust_report: bool) -> Self {
        Self {
            reported_millihertz: None,
            trust_report,
            measured: Output::new(),
        }
    }

    /// The current monitor's report, in millihertz, as the platform gave it.
    pub fn report(&mut self, millihertz: Option<u32>) {
        self.reported_millihertz = millihertz.filter(|mhz| *mhz > 0);
    }

    /// Feeds the measurement; see [`Output::observe`].
    pub fn observe(&mut self, frame: Frame) -> Option<Option<u32>> {
        self.measured.observe(frame)
    }

    /// The most the display can show, or `None` when nothing says. The measurement outranks the
    /// report in both directions: it is what the frames saw.
    #[must_use]
    pub fn hz(&self) -> Option<u32> {
        self.measured.hz().or_else(|| {
            self.trust_report
                .then_some(self.reported_millihertz)
                .flatten()
                .map(|mhz| (mhz + 500) / 1000)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drs::Target;

    fn held(hz: f32, gpu_share: f32) -> Frame {
        let interval = 1.0 / hz;
        Frame {
            interval,
            wait: interval * 0.8,
            gpu: interval * gpu_share,
        }
    }

    fn feed(display: &mut Display, frame: Frame, n: usize) {
        for _ in 0..n {
            display.observe(frame);
        }
    }

    fn bound(display: &Display, limiter: Option<u32>) -> Target {
        "90".parse::<Target>()
            .unwrap()
            .at_most(limiter)
            .at_most(display.hz())
    }

    #[test]
    fn a_60_hz_output_with_a_fast_gpu_holds_a_90_target_to_60() {
        let mut display = Display::new(false);
        feed(&mut display, held(60.0, 0.3), WINDOW);
        assert_eq!(display.hz(), Some(60));
        assert_eq!(bound(&display, None).hz(), Some(60));
    }

    #[test]
    fn the_clamp_is_what_holds_it() {
        let display = Display::new(false);
        assert_eq!(bound(&display, None).hz(), Some(90));
    }

    #[test]
    fn a_slow_gpu_with_no_wait_is_no_evidence() {
        let mut display = Display::new(false);
        let slow = Frame {
            interval: 1.0 / 60.0,
            wait: 0.001,
            gpu: 0.0155,
        };
        feed(&mut display, slow, WINDOW * 2);
        assert_eq!(display.hz(), None);
        // A GPU filling the interval is no evidence even when the wait is the long call.
        let backed_up = Frame {
            interval: 1.0 / 60.0,
            wait: 0.014,
            gpu: 0.0155,
        };
        feed(&mut display, backed_up, WINDOW * 2);
        assert_eq!(display.hz(), None);
    }

    #[test]
    fn a_144_hz_panel_keeps_144() {
        let mut display = Display::new(true);
        display.report(Some(143_981));
        assert_eq!(display.hz(), Some(144));
        feed(&mut display, held(144.0, 0.3), WINDOW);
        assert_eq!(display.hz(), Some(144));
        assert_eq!(
            "144".parse::<Target>().unwrap().at_most(display.hz()).hz(),
            Some(144)
        );
    }

    #[test]
    fn a_false_report_of_60_gives_way_to_a_measured_144() {
        let mut display = Display::new(true);
        display.report(Some(60_000));
        assert_eq!(display.hz(), Some(60));
        feed(&mut display, held(144.0, 0.3), WINDOW);
        assert_eq!(display.hz(), Some(144));
    }

    #[test]
    fn a_virtual_monitors_report_is_not_believed() {
        let mut display = Display::new(false);
        display.report(Some(90_000));
        assert_eq!(display.hz(), None);
    }

    #[test]
    fn a_variable_refresh_output_gives_no_answer() {
        let mut display = Display::new(false);
        feed(&mut display, held(60.0, 0.3), WINDOW);
        assert_eq!(display.hz(), Some(60));
        for i in 0..WINDOW * 4 {
            let hz = if i % 2 == 0 { 48.0 } else { 90.0 };
            display.observe(held(hz, 0.3));
        }
        assert_eq!(display.hz(), None);
    }

    #[test]
    fn a_stutter_does_not_withdraw_the_rate() {
        let mut display = Display::new(false);
        feed(&mut display, held(60.0, 0.3), WINDOW);
        for i in 0..WITHDRAW_AFTER - 1 {
            let hz = if i % 2 == 0 { 40.0 } else { 90.0 };
            display.observe(held(hz, 0.3));
        }
        assert_eq!(display.hz(), Some(60));
    }

    #[test]
    fn the_rate_is_kept_when_the_evidence_disappears() {
        let mut display = Display::new(false);
        feed(&mut display, held(60.0, 0.3), WINDOW);
        // The clamp holds the scale; the GPU now fills the interval and the wait is gone.
        let busy = Frame {
            interval: 1.0 / 60.0,
            wait: 0.0005,
            gpu: 0.015,
        };
        feed(&mut display, busy, WINDOW * 3);
        assert_eq!(display.hz(), Some(60));
    }

    #[test]
    fn a_stall_and_a_jittery_few_frames_do_not_move_it() {
        let mut display = Display::new(false);
        feed(&mut display, held(60.0, 0.3), WINDOW);
        display.observe(Frame {
            interval: 0.5,
            wait: 0.45,
            gpu: 0.01,
        });
        display.observe(Frame {
            interval: f32::NAN,
            wait: 0.0,
            gpu: 0.0,
        });
        assert_eq!(display.hz(), Some(60));
    }

    #[test]
    fn a_change_of_mode_lands_on_the_new_rate() {
        let mut display = Display::new(false);
        feed(&mut display, held(60.0, 0.3), WINDOW);
        feed(&mut display, held(120.0, 0.3), WINDOW * 2);
        assert_eq!(display.hz(), Some(120));
    }
}
