//! The stopwatch the boot report names each step of the load with.
//!
//! Split out of `boot.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// A step the load has to be quick enough at for nobody to notice. Anything
/// slower gets named in the report; anything faster would only be noise there.
const FELT: std::time::Duration = std::time::Duration::from_millis(20);

/// A stopwatch that names each step of the load as it passes.
///
/// The boot is the longest wait this build asks anyone to sit through and it
/// used to be one opaque call, so "which part of it" was a question nobody
/// could answer without a profiler. One [`Steps::lap`] per step answers it on
/// every boot, in the report the load already prints, which is also what keeps
/// the answer current: a step that gets slower says so rather than waiting to
/// be re-measured.
///
/// Wall clock, deliberately, and this is the one place in the codebase that is
/// allowed to be - see `docs/architecture/determinism.md`. Nothing here reaches
/// the simulation: these are strings for a human.
pub(super) struct Steps {
    at: web_time::Instant,
    steps: Vec<(&'static str, std::time::Duration)>,
}

impl Steps {
    pub(super) fn new() -> Self {
        Self {
            at: web_time::Instant::now(),
            steps: Vec::new(),
        }
    }

    /// Closes the step that has been running since the last lap.
    pub(super) fn lap(&mut self, what: &'static str) {
        let now = web_time::Instant::now();
        self.steps.push((what, now - self.at));
        self.at = now;
    }

    /// One line: the total, then the steps that were felt, slowest first.
    ///
    /// Slowest first rather than in load order because the reason to read this
    /// line at all is "what am I waiting for", and that is the first name on it.
    pub(super) fn describe(&self, what: &str) -> String {
        let total: std::time::Duration = self.steps.iter().map(|(_, took)| *took).sum();
        let mut felt: Vec<_> = self
            .steps
            .iter()
            .filter(|(_, took)| *took >= FELT)
            .collect();
        felt.sort_by_key(|(_, took)| std::cmp::Reverse(*took));
        let named = felt
            .iter()
            .map(|(name, took)| format!("{name} {:.2}", took.as_secs_f32()))
            .collect::<Vec<_>>()
            .join(", ");
        if named.is_empty() {
            return format!("{what} took {:.2} s", total.as_secs_f32());
        }
        format!("{what} took {:.2} s - {named}", total.as_secs_f32())
    }
}
