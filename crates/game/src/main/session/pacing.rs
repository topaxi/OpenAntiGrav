//! The frame limiter: which limit is in force for what is on screen, and the
//! schedule that holds the loop to it.

use oag_present::perf;

use super::Session;
use crate::stage::Stage;

impl Session {
    /// How long one frame is allowed to take at the least, or `None` when
    /// nothing is limiting.
    ///
    /// `None` under `vsync = "on"` alone, whatever the limit says: there the
    /// display is deciding, and a second limiter underneath it does not halve
    /// the frame rate, it beats against the refresh and turns an even 60 into
    /// an uneven one. That is why the menu greys the row out under that one
    /// value and not the other two - `off` and `smooth` both leave the loop
    /// free to run ahead, and under `smooth` the limit is the only thing
    /// stopping the GPU rendering frames that are then discarded.
    fn frame_period(&self) -> Option<std::time::Duration> {
        if self.settings.display.vsync.paces_itself() {
            return None;
        }
        self.active_limit().period()
    }

    /// The limit in force for what is on screen: the configured one in a race,
    /// [`perf::FrameLimit::front_end`] everywhere else.
    ///
    /// The launcher, the boot reel, the loading wave, the menus and the pause
    /// menu all count as front end. The pause menu is a `Stage::Menu` over a
    /// held backdrop picture with the race parked in `suspended_race`, so no
    /// race scene is being drawn behind it (**chosen, not measured**). A race
    /// that is still building is already `Stage::Race` and keeps the race limit.
    /// Under `Vsync::On` [`Session::frame_period`] returns before this is read,
    /// so the cap is inert there.
    fn active_limit(&self) -> perf::FrameLimit {
        let limit = self.settings.display.frame_limit;
        match self.stage {
            Stage::Race(_) => limit,
            _ => limit.front_end(),
        }
    }

    /// The earliest the next frame may start, or `None` for as soon as
    /// possible.
    pub(crate) fn next_frame_at(&self) -> Option<web_time::Instant> {
        self.frame_period().map(|_| self.next_frame)
    }

    /// Whether a redraw the browser delivered should be skipped to hold the
    /// limit. A quarter period of slack, because `requestAnimationFrame` lands
    /// on display refreshes and a deadline that falls a hair after one would
    /// otherwise halve a 60 fps cap on a 60 Hz panel.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn too_early_for_frame(&self, now: web_time::Instant) -> bool {
        self.frame_period()
            .is_some_and(|period| now + period / 4 < self.next_frame)
    }

    /// What a frame time is measured against in the overlay: the rate this
    /// build is actually trying to present at.
    ///
    /// **Not the tick rate**, which is what this used to pass and which stopped
    /// being right the moment a frame limiter existed. Against 60 Hz a 4 ms
    /// frame is a 4-pixel sliver and every column is green, so a 240-limited
    /// run - the default - drew a pacing graph that conveyed nothing. The rule
    /// line means "one target frame" and the target has to be the one in force.
    ///
    /// With no limit in force the display is the target: its refresh as
    /// [`Self::display_hz`] knows it, and the simulation's own rate while
    /// nothing says.
    pub(super) fn presentation_hz(&self) -> u32 {
        self.frame_period()
            .and_then(|_| self.active_limit().hz())
            .or_else(|| self.display.hz())
            .unwrap_or_else(|| self.clock.rate().hz())
    }

    /// The rate the frame limiter is actually holding the loop to, or `None`
    /// when nothing is.
    ///
    /// `None` for `FrameLimit::UNLIMITED`. The display is a bound of its own
    /// ([`Self::display_hz`]); neither falls back to the simulation's 60, which
    /// is a fine default for a graph's scale and a bad one for a clamp, because
    /// a 144 Hz panel would silently have its `target_fps` capped at 60. See
    /// [`oag_present::drs::Target::at_most`].
    pub(super) fn limiter_hz(&self) -> Option<u32> {
        // The race's own limit, not `active_limit`: this feeds dynamic
        // resolution's clamp, which is a race concern.
        self.frame_period()
            .and_then(|_| self.settings.display.frame_limit.hz())
    }

    /// The most the output can show, or `None` while nothing says
    /// ([`oag_present::refresh`]).
    pub(super) fn display_hz(&self) -> Option<u32> {
        self.display.hz()
    }

    /// The rate the loop is held to by the limiter or the output, whichever is lower.
    pub(super) fn held_hz(&self) -> Option<u32> {
        match (self.limiter_hz(), self.display_hz()) {
            (Some(limit), Some(shown)) => Some(limit.min(shown)),
            (limit, shown) => limit.or(shown),
        }
    }

    /// One frame's evidence about the output, and the monitor's report asked again about once
    /// a second so a window moved to another monitor follows. Only race frames with a timed
    /// GPU cost count: without the cost a slow GPU cannot be told from a slow display.
    pub(super) fn feed_display(&mut self, frame_seconds: Option<f32>, wait: f32) {
        if self.frame_index.is_multiple_of(60) {
            let monitor = self.gpu.output.window.current_monitor();
            let reported = monitor.as_ref().and_then(|m| m.refresh_rate_millihertz());
            log::trace!("monitor reports {reported:?} mHz");
            self.display.report(reported);
        }
        if matches!(self.stage, Stage::Race(_))
            && let (Some(interval), Some(gpu)) = (frame_seconds, self.governor_gpu)
        {
            self.display.observe(oag_present::refresh::Frame {
                interval,
                wait,
                gpu,
            });
        }
        let shown = self.display.hz();
        if self.logged_display_hz != Some(shown) {
            self.logged_display_hz = Some(shown);
            log::debug!(
                "display refresh in force: {} (target {} held under it)",
                shown.map_or_else(|| "unknown".to_string(), |hz| format!("{hz} Hz")),
                self.render_profile().target_fps
            );
        }
    }

    /// Puts the next frame on the schedule, one period after the last one was
    /// *due* rather than one period after now.
    ///
    /// **This is the difference between a 240 limit delivering 240 and
    /// delivering 220.** A timer wakes at or after its deadline, never before,
    /// and the platform's granularity is around a millisecond - which at a
    /// 4.17 ms period is a quarter of it. Measuring the next deadline from when
    /// the loop actually woke banks that overshoot into every frame and the
    /// error compounds into a systematically low frame rate; measuring it from
    /// the previous deadline puts the frames on a fixed grid, so a late wake-up
    /// is followed by an early-relative one and the *average* is the rate that
    /// was asked for.
    ///
    /// The schedule is resynchronised whenever it is more than one period away
    /// from now, in either direction: behind means the loop stalled on a load
    /// and must not pay it back as a burst of frames, ahead means the limit
    /// itself just changed and the old period is still on the clock.
    pub(super) fn schedule_next_frame(&mut self, now: web_time::Instant) {
        let limit = self.active_limit();
        if self.logged_limit != Some(limit) {
            self.logged_limit = Some(limit);
            log::debug!(
                "frame limit in force: {limit} (configured {})",
                self.settings.display.frame_limit
            );
        }
        let Some(period) = self.frame_period() else {
            self.next_frame = now;
            return;
        };
        self.next_frame += period;
        if self.next_frame < now || self.next_frame > now + period {
            self.next_frame = now + period;
        }
    }
}
