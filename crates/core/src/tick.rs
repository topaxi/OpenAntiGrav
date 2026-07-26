//! Fixed-timestep simulation clock.
//!
//! The simulation advances in whole ticks of a fixed duration. Rendering may
//! run at any rate and interpolates between the last two states; the simulation
//! never sees a variable delta. A variable timestep would make replays
//! frame-rate dependent, which defeats every form of verification we rely on.
//!
//! # The tick rate is a decision, not yet a measurement
//!
//! [`TickRate::DEFAULT`] is 60 Hz. That is a project decision, taken so work
//! can proceed, and **not** something confirmed from the original binary.
//!
//! [`TickRate`] stays a parameter rather than becoming a constant so that
//! confirming a different rate in M2 is a one-line change at the call site
//! instead of a re-tuning of every subsystem built on top. Nothing downstream
//! should assume 60.
//!
//! See `docs/reverse-engineering/methodology.md`.

/// How many simulation ticks elapse per second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickRate {
    hz: u32,
}

impl TickRate {
    /// The project's default simulation rate, 60 Hz.
    ///
    /// A decision taken so work can proceed, not a measurement. Wipeout
    /// Pulse's actual rate has not been read from the binary yet. Do not cite
    /// this as a discovered fact about the original.
    pub const DEFAULT: Self = Self { hz: 60 };

    /// Creates a tick rate.
    ///
    /// # Panics
    ///
    /// Panics if `hz` is zero.
    #[must_use]
    pub const fn from_hz(hz: u32) -> Self {
        assert!(hz > 0, "tick rate must be positive");
        Self { hz }
    }

    /// Ticks per second.
    #[must_use]
    pub const fn hz(self) -> u32 {
        self.hz
    }

    /// Duration of one tick in seconds.
    ///
    /// Derived from the integer rate on every call rather than being stored as
    /// a float, so the value cannot drift and is identical everywhere.
    #[must_use]
    pub fn dt(self) -> f32 {
        1.0 / self.hz as f32
    }

    /// Duration of one tick in nanoseconds, rounded down.
    #[must_use]
    pub const fn dt_nanos(self) -> u64 {
        1_000_000_000 / self.hz as u64
    }
}

/// Accumulates real elapsed time and hands out whole ticks.
///
/// Time is carried in integer nanoseconds. A float accumulator would lose
/// precision over a long session and make the tick boundary depend on how long
/// the process had been running.
#[derive(Debug, Clone)]
pub struct TickClock {
    rate: TickRate,
    accumulated_nanos: u64,
    tick: u64,
    max_ticks_per_step: u32,
}

impl TickClock {
    /// Creates a clock at the given rate.
    #[must_use]
    pub fn new(rate: TickRate) -> Self {
        Self {
            rate,
            accumulated_nanos: 0,
            tick: 0,
            // Bounds the catch-up burst after a stall (breakpoint, window drag,
            // disk hitch). Without a cap, one long pause makes the sim run a
            // huge batch of ticks, which looks like a freeze and can cascade.
            max_ticks_per_step: 8,
        }
    }

    /// The tick rate.
    #[must_use]
    pub fn rate(&self) -> TickRate {
        self.rate
    }

    /// Number of ticks simulated so far.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Feeds in elapsed real time and returns how many ticks to run now.
    ///
    /// Surplus time beyond the catch-up cap is discarded rather than banked, so
    /// a stall costs simulated time instead of causing a burst later.
    pub fn advance(&mut self, elapsed_nanos: u64) -> u32 {
        self.accumulated_nanos = self.accumulated_nanos.saturating_add(elapsed_nanos);

        let dt = self.rate.dt_nanos();
        let mut ticks = (self.accumulated_nanos / dt) as u32;
        self.accumulated_nanos %= dt;

        if ticks > self.max_ticks_per_step {
            ticks = self.max_ticks_per_step;
            self.accumulated_nanos = 0;
        }

        self.tick += u64::from(ticks);
        ticks
    }

    /// Fraction of the way from the previous tick to the next, in `[0, 1)`.
    ///
    /// Renderers use this to interpolate, which is what decouples display rate
    /// from simulation rate.
    #[must_use]
    pub fn interpolation_alpha(&self) -> f32 {
        self.accumulated_nanos as f32 / self.rate.dt_nanos() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: TickRate = TickRate::from_hz(60);

    #[test]
    fn accumulates_partial_ticks() {
        let mut clock = TickClock::new(RATE);
        let half = RATE.dt_nanos() / 2;

        assert_eq!(clock.advance(half), 0);
        assert_eq!(clock.advance(half), 1);
        assert_eq!(clock.tick(), 1);
    }

    #[test]
    fn caps_catch_up_after_a_stall() {
        let mut clock = TickClock::new(RATE);
        let ten_seconds = 10 * 1_000_000_000;

        let ticks = clock.advance(ten_seconds);
        assert_eq!(ticks, 8, "a stall must not produce a 600-tick burst");
        assert_eq!(
            clock.interpolation_alpha(),
            0.0,
            "surplus is dropped, not banked"
        );
    }

    #[test]
    fn tick_count_is_exact_over_a_long_run() {
        let mut clock = TickClock::new(RATE);
        let dt = RATE.dt_nanos();

        for _ in 0..100_000 {
            clock.advance(dt);
        }
        // Integer nanoseconds mean no drift. A float accumulator would be off
        // by a handful of ticks after this many steps.
        assert_eq!(clock.tick(), 100_000);
    }

    #[test]
    fn alpha_stays_in_unit_interval() {
        let mut clock = TickClock::new(RATE);
        for step in 0..1000 {
            clock.advance(step * 137);
            let a = clock.interpolation_alpha();
            assert!((0.0..1.0).contains(&a), "{a} out of range");
        }
    }
}
