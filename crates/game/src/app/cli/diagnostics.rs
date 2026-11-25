//! Reports and one-off actions that run in place of the game.

/// Reports and one-off actions that run in place of the game.
#[derive(clap::Args, Debug)]
pub(crate) struct DiagnosticsArgs {
    /// Show the language picker even when a language is already chosen.
    ///
    /// Without this the picker is skipped once `settings.toml` names a
    /// language, which is what a player wants and what makes the second run
    /// shorter than the first.
    #[arg(long)]
    pub(crate) pick_language: bool,

    /// Print every state transition as it happens, exits included.
    #[arg(long)]
    pub(crate) trace: bool,

    /// Report what was loaded and exit, without rendering. With `--prefetch`: convert everything, write the cache manifest, then exit.
    #[arg(long)]
    pub(crate) dry_run: bool,

    /// Print each connected pad's button presses with the value `[controls.music]` takes for them
    /// (a button name, or `raw:CODE`), until stopped with Ctrl-C. Opens no window and no audio.
    /// See docs/tools/oag-game.md, "Music controls in a race".
    #[arg(long, conflicts_with_all = ["race", "screenshot", "dry_run", "trace_out", "launcher", "drive", "drive_file"])]
    pub(crate) probe_pad: bool,

    /// Download the pinned LGPL `ffmpeg` that plays Pulse's, Pure's and 2048's music and every
    /// movie, into the per-user cache directory, then exit. Naming the flag is the consent. Does
    /// nothing when an `ffmpeg` is already on `PATH` unless `[ffmpeg] use_pinned` is set. Opens
    /// no window and no audio. See docs/architecture/ffmpeg.md.
    #[arg(long, conflicts_with_all = ["race", "screenshot", "dry_run", "trace_out", "launcher", "drive", "drive_file", "probe_pad"])]
    pub(crate) fetch_ffmpeg: bool,
}
