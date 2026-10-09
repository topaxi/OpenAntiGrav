//! The `Dev` overlay's memory line: how much physical RAM this process is
//! holding right now.
//!
//! Every platform's answer costs a file read or a kernel query even at its
//! cheapest, and `memory_stats::memory_stats` is what actually asks - see
//! `Cargo.toml` for why that crate rather than this workspace's own code,
//! which `unsafe_code = "deny"` rules out for a raw syscall. [`Probe`] is
//! what keeps that cost off the frame loop: it throttles the read the same
//! way the module doc on [`super`] argues [`super::Meter::record`] should be
//! fed a duration rather than read a clock itself - a caller passes `now` in,
//! [`Probe::sample`] never reads it.

use web_time::{Duration, Instant};

/// How often the resident set is actually re-read.
///
/// Four times a second is fast enough that a track's memory spike shows up
/// well within the two seconds [`super::WINDOW`] keeps a hitch on screen for,
/// and infrequent enough that the read costs nothing next to the frame it is
/// reporting on - [`super::Vsync::Off`]'s own doc measures over a thousand of
/// those a second, which is what an unthrottled per-frame read would be
/// competing with.
const SAMPLE_INTERVAL: Duration = Duration::from_millis(250);

/// Caches the last resident-set-size read and only re-samples once
/// [`SAMPLE_INTERVAL`] has passed.
///
/// Lives on the composition root next to [`super::Meter`], not in anything
/// the simulation touches: `docs/architecture/determinism.md` forbids a tick
/// reading the wall clock, and `now` reaching [`Probe::sample`] is exactly
/// that read, done once by the frame loop instead of once inside this type.
#[derive(Debug, Clone, Default)]
pub struct Probe {
    last: Option<(Instant, Option<u64>)>,
}

impl Probe {
    /// A probe that samples on its first call.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The process's resident set size in bytes, sampled at most every
    /// [`SAMPLE_INTERVAL`] and cached between samples.
    ///
    /// `None` when the platform has no reader or the read failed - the
    /// overlay leaves the line out rather than show a stale or invented
    /// number, the same contract [`super::draw_list`] already gives a scene
    /// or a video label it was not handed.
    pub fn sample(&mut self, now: Instant) -> Option<u64> {
        if let Some((at, value)) = self.last
            && now.saturating_duration_since(at) < SAMPLE_INTERVAL
        {
            return value;
        }
        let value = memory_stats::memory_stats().map(|stats| stats.physical_mem as u64);
        self.last = Some((now, value));
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_probe_samples_on_its_first_call() {
        let mut probe = Probe::new();
        probe.sample(Instant::now());
        assert!(probe.last.is_some());
    }

    #[test]
    fn a_call_inside_the_interval_does_not_resample() {
        let mut probe = Probe::new();
        let now = Instant::now();
        probe.sample(now);
        let (first_at, _) = probe.last.expect("sampled");

        probe.sample(now + SAMPLE_INTERVAL - Duration::from_millis(1));
        let (second_at, _) = probe.last.expect("still sampled");
        assert_eq!(first_at, second_at, "should have reused the cached read");
    }

    #[test]
    fn a_call_past_the_interval_resamples() {
        let mut probe = Probe::new();
        let now = Instant::now();
        probe.sample(now);
        let (first_at, _) = probe.last.expect("sampled");

        probe.sample(now + SAMPLE_INTERVAL);
        let (second_at, _) = probe.last.expect("still sampled");
        assert!(second_at > first_at, "should have re-sampled");
    }

    /// Not a determinism test - it is asserting the crate answers plausibly
    /// on the one platform every contributor and CI runner actually has, the
    /// same way `just` itself runs on `ubuntu-latest` alone. Windows and
    /// macOS are covered by `memory-stats`'s own CI, not this suite.
    #[cfg(target_os = "linux")]
    #[test]
    fn resident_memory_is_plausible_on_linux() {
        let bytes = Probe::new()
            .sample(Instant::now())
            .expect("linux always answers");
        assert!(bytes > 1024 * 1024, "{bytes} bytes is not a real process");
    }
}
