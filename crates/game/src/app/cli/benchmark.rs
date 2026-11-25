//! `--benchmark` and `--preset`: the in-game benchmark as a run of its own.

/// The benchmark as a command line asks for it.
#[derive(clap::Args, Debug, Default)]
pub(crate) struct BenchmarkArgs {
    /// Fly the benchmark: one fixed race (Single Race, seed 1, the title's default circuit, the
    /// full field, the player on the autopilot, the title's laps) drawn in the window with your
    /// settings, ending on a stats screen and a result file in the user data directory that
    /// `just bench-compare` reads. The disc is the one the run would otherwise open; name another
    /// as the first argument. See docs/tools/in-game-benchmark.md.
    #[arg(long, conflicts_with_all = ["race", "launcher", "demo", "wallpaper", "dry_run", "screenshot"])]
    pub(crate) benchmark: bool,

    /// With `--benchmark`: draw with this preset of the title's `benchmark.toml` instead of your
    /// own graphics settings. Nothing is written back to your settings.
    #[arg(long, value_name = "NAME", requires = "benchmark")]
    pub(crate) preset: Option<String>,
}

impl BenchmarkArgs {
    /// What the run asks for, when it is a benchmark.
    ///
    /// `ticks` is `--ticks`: where it is non-zero the run stops there and says it did not finish
    /// (a short run for a test; a result file records the cap it ran under).
    pub(crate) fn request(&self, ticks: u32) -> Option<crate::benchmark::Request> {
        self.benchmark.then(|| crate::benchmark::Request {
            preset: self.preset.clone(),
            tick_cap: (ticks > 0).then_some(ticks),
        })
    }
}
