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
    fn feeding_back_the_tick_length_produces_exactly_one_tick_each_time() {
        let mut clock = TickClock::new(RATE);
        let dt = RATE.dt_nanos();

        for _ in 0..100_000 {
            clock.advance(dt);
        }
        // This says the accumulator is exact in integer nanoseconds, and no
        // more: it feeds the clock its own rounded tick length, so it cannot
        // see the rounding. That is what the next test is for.
        assert_eq!(clock.tick(), 100_000);
    }

    /// Feeds real seconds rather than the clock's own tick length.
    ///
    /// `1_000_000_000 / 60` truncates to 16,666,666 ns, so a tick is 2/3 of a
    /// nanosecond short and the clock runs *fast* against a real second. The
    /// drift is bounded and known: one extra tick every 416,667 seconds, which
    /// is 4.8 days of continuous simulation. It is a deliberate trade (integer
    /// nanoseconds are worth more than an exact 60 Hz), and the point of pinning
    /// it is that a change in the accumulator would move this number.
    ///
    /// The previous version of this test claimed to prove no drift while feeding
    /// the clock its own rounded tick length, so it could not have detected any.
    #[test]
    fn the_truncated_tick_length_makes_the_clock_run_slightly_fast() {
        // 60 ticks of 16,666,666 ns are 40 ns short of a second, and that
        // shortfall is the entire drift.
        let dt = RATE.dt_nanos();
        assert_eq!(1_000_000_000 - dt * u64::from(RATE.hz()), 40);

        // So one whole tick is gained after dt / 40 seconds, a little under
        // 416,667, which is 4.8 days of continuous simulation.
        assert_eq!(dt / 40, 416_666);

        // Confirm it by running, in steps small enough that the catch-up cap
        // never truncates. A cap hit would *lose* time, which is the opposite
        // error and would hide this one.
        const STEP_NANOS: u64 = 100_000_000;
        let mut clock = TickClock::new(RATE);
        let mut fed = 0u64;
        let mut first_ahead = None;

        while first_ahead.is_none() {
            let produced = clock.advance(STEP_NANOS);
            assert!(produced <= 8, "the catch-up cap was hit");
            fed += STEP_NANOS;

            let ideal = fed * u64::from(RATE.hz()) / 1_000_000_000;
            assert!(
                clock.tick() >= ideal,
                "the clock is behind after {fed} ns, which the truncation cannot cause"
            );
            if clock.tick() > ideal {
                first_ahead = Some(fed / 1_000_000_000);
            }
        }

        assert_eq!(
            first_ahead,
            Some(416_666),
            "the drift rate moved; work out why before touching this number"
        );
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
