//! Timed motion for the front end, driven by the tick rather than by the clock.
//!
//! # Why a module rather than a `lerp` where it is needed
//!
//! There was no time-based interpolation anywhere in this workspace before
//! this: `fog::lerp` blends colour bands and `vex`'s UV interpolation walks
//! keyframes, and neither is reachable from a draw list. Menu motion needs one
//! shape used the same way in three places - a page change, the cursor, and
//! whatever comes next - so it is worth the module.
//!
//! # The clock rule
//!
//! **Nothing here reads the wall clock**, and the reason is not that menus are
//! part of the simulation - they are not. It is that `--screenshot`, `--until`
//! and `--ticks` have to produce the same picture twice. A tween advanced by
//! `Instant::now` would make a captured frame depend on how fast the machine
//! that captured it happened to be, and two runs of the same command would
//! differ for a reason nothing in the command says. So [`Tween::advance`] takes
//! the same fixed `dt` the stage above it is already stepping with. See
//! `docs/architecture/determinism.md`, whose rules this follows even though it
//! is outside what they bind.

/// A value on its way from one number to another.
///
/// Holds no start and end of its own: a page change animates several quantities
/// at once - scale, alpha, offset - and they share one clock. So this is the
/// clock, and callers read [`Self::eased`] to shape whatever they are moving.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tween {
    elapsed: f32,
    duration: f32,
}

impl Tween {
    /// A tween that will run for `duration` seconds, starting now.
    ///
    /// A duration of zero or less is allowed and lands [`Self::done`]
    /// immediately, which is what the disc's own `transition="0"` screens want:
    /// they cut rather than move, and expressing that as "a transition of no
    /// length" keeps one code path instead of two.
    #[must_use]
    pub fn new(duration: f32) -> Self {
        Self {
            elapsed: 0.0,
            duration: duration.max(0.0),
        }
    }

    /// Moves it on by one tick's worth of time.
    ///
    /// `dt` is the fixed step the caller is already using. Clamped at the
    /// duration rather than allowed to overshoot, so [`Self::progress`] is
    /// always in `0..=1` and a caller cannot read a scale past the one the
    /// transition ends on.
    pub fn advance(&mut self, dt: f32) {
        self.elapsed = (self.elapsed + dt.max(0.0)).min(self.duration);
    }

    /// Whether it has arrived.
    #[must_use]
    pub fn done(&self) -> bool {
        self.elapsed >= self.duration
    }

    /// How far along, linearly, in `0..=1`.
    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    /// How far along, shaped.
    ///
    /// **The curve is invented.** Nothing in the disc's data states one, and a
    /// capture of the original's page change gives only its shape: the outgoing
    /// page's scale grows by 0.026, 0.027, 0.026, 0.026, 0.035, 0.053 over its
    /// first six frames, so the motion *accelerates* rather than running
    /// linearly or easing out. This is the simplest curve with that property,
    /// picked to match the measurement's shape rather than derived from
    /// anything. See `docs/ui/menus-original.md`, which scores it accordingly.
    ///
    /// If someone later reads the real curve out of the executable, this is the
    /// one function to change.
    #[must_use]
    pub fn eased(&self) -> f32 {
        let t = self.progress();
        t * t
    }

    /// `from` to `to`, along [`Self::eased`].
    #[must_use]
    pub fn between(&self, from: f32, to: f32) -> f32 {
        from + (to - from) * self.eased()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 60 Hz step lands exactly on the end rather than a hair short of it.
    #[test]
    fn a_tween_arrives_and_stops() {
        let mut tween = Tween::new(0.5);
        for _ in 0..30 {
            tween.advance(1.0 / 60.0);
        }
        assert!(tween.done(), "half a second of 60 Hz ticks is 0.5 s");
        assert!((tween.progress() - 1.0).abs() < f32::EPSILON);
    }

    /// Overshooting a tick does not overshoot the value, which is what lets a
    /// caller read the scale without clamping at every site.
    #[test]
    fn a_late_tick_cannot_push_it_past_the_end() {
        let mut tween = Tween::new(0.5);
        tween.advance(10.0);
        assert!((tween.progress() - 1.0).abs() < f32::EPSILON);
        assert!((tween.between(1.0, 2.0) - 2.0).abs() < f32::EPSILON);
    }

    /// The endpoints are exact. An eased curve that misses them by a rounding
    /// error leaves a page permanently a fraction of a pixel out of place.
    #[test]
    fn the_ends_are_exact() {
        let tween = Tween::new(0.5);
        assert!((tween.between(4.0, 9.0) - 4.0).abs() < f32::EPSILON);

        let mut arrived = Tween::new(0.5);
        arrived.advance(0.5);
        assert!((arrived.between(4.0, 9.0) - 9.0).abs() < f32::EPSILON);
    }

    /// It only ever moves forwards, which is what a caller reading a scale off
    /// it depends on to avoid a visible stutter.
    #[test]
    fn it_is_monotonic_in_between() {
        let mut tween = Tween::new(0.5);
        let mut last = tween.eased();
        for _ in 0..30 {
            tween.advance(1.0 / 60.0);
            let now = tween.eased();
            assert!(now >= last, "went backwards: {last} then {now}");
            last = now;
        }
    }

    /// A zero-length transition is a cut, not a division by zero. The disc
    /// authors `transition="0"` on several screens.
    #[test]
    fn a_transition_of_no_length_is_already_over() {
        let tween = Tween::new(0.0);
        assert!(tween.done());
        assert!((tween.progress() - 1.0).abs() < f32::EPSILON);
        assert!((tween.between(0.0, 5.0) - 5.0).abs() < f32::EPSILON);
    }

    /// The curve accelerates, which is the one property the capture actually
    /// shows: the outgoing page's scale steps grow rather than shrink.
    #[test]
    fn the_curve_accelerates_the_way_the_capture_does() {
        let mut tween = Tween::new(1.0);
        let mut steps = Vec::new();
        let mut last = tween.eased();
        for _ in 0..10 {
            tween.advance(0.1);
            steps.push(tween.eased() - last);
            last = tween.eased();
        }
        for pair in steps.windows(2) {
            assert!(pair[1] > pair[0], "steps must grow: {steps:?}");
        }
    }
}
