//! What one peer may cost the server: a log line a second, and a datagram budget.
//!
//! Both read a clock handed to them, so a test drives them on virtual time. The values are
//! **chosen, not measured** (`docs/networking/server-roadmap.md`, "V1: survive the network").

use web_time::{Duration, Instant};

/// Lets a repeated warning through at most once a second and counts the lines it held back,
/// so the next line printed says how many were lost.
#[derive(Debug, Default)]
pub struct LogLimit {
    last: Option<Instant>,
    held: u32,
}

/// The least time between two lines of one [`LogLimit`]. Chosen.
pub const LOG_INTERVAL: Duration = Duration::from_secs(1);

impl LogLimit {
    /// Whether a line may be printed at `now`: `Some(held back since the last line)`, or
    /// `None` when it must be dropped (and is counted).
    pub fn allow(&mut self, now: Instant) -> Option<u32> {
        if self
            .last
            .is_some_and(|last| now.saturating_duration_since(last) < LOG_INTERVAL)
        {
            self.held = self.held.saturating_add(1);
            return None;
        }
        self.last = Some(now);
        Some(std::mem::take(&mut self.held))
    }

    /// Logs `line()` at `warn` if the limit allows, with the count held back.
    pub fn warn(&mut self, line: impl FnOnce() -> String) {
        if let Some(held) = self.allow(Instant::now()) {
            if held == 0 {
                log::warn!("{}", line());
            } else {
                log::warn!("{} ({held} like it held back)", line());
            }
        }
    }
}

/// A token bucket: `rate` a second, at most `burst` saved up.
#[derive(Debug)]
pub struct Budget {
    rate: f64,
    burst: f64,
    tokens: f64,
    at: Instant,
}

impl Budget {
    /// A full bucket at `now`.
    #[must_use]
    pub fn new(rate: u32, burst: u32, now: Instant) -> Self {
        Self {
            rate: f64::from(rate),
            burst: f64::from(burst),
            tokens: f64::from(burst),
            at: now,
        }
    }

    /// Takes one token at `now`; `false` when the bucket is empty.
    pub fn take(&mut self, now: Instant) -> bool {
        let since = now.saturating_duration_since(self.at).as_secs_f64();
        self.at = now;
        self.tokens = (self.tokens + since * self.rate).min(self.burst);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_log_limit_passes_one_line_a_second_and_counts_the_rest() {
        let t0 = Instant::now();
        let mut limit = LogLimit::default();
        assert_eq!(limit.allow(t0), Some(0));
        for i in 1..=100u32 {
            assert_eq!(
                limit.allow(t0 + Duration::from_millis(u64::from(i) * 5)),
                None
            );
        }
        assert_eq!(limit.allow(t0 + Duration::from_millis(1_000)), Some(100));
    }

    #[test]
    fn a_budget_allows_its_burst_then_its_rate() {
        let t0 = Instant::now();
        let mut budget = Budget::new(120, 120, t0);
        assert_eq!((0..500).filter(|_| budget.take(t0)).count(), 120);
        let later = t0 + Duration::from_millis(500);
        assert_eq!((0..500).filter(|_| budget.take(later)).count(), 60);
    }
}
