//! Where the game comes from and the boot movie.

/// Where the game comes from and the boot movie.
#[derive(clap::Args, Debug)]
pub(crate) struct BootArgs {
    /// Your game: a disc image (`.chd` or `.iso`, for Pulse, Pure or HD/Fury),
    /// or an unpacked folder (2048 or Omega, from their `.pkg`s), for example
    /// `data/images/pulse-psp-eu.chd`. See `oag_assets::Layout`.
    ///
    /// Left out, it is searched for: `data/images/` in the current directory,
    /// then beside the program (AppImage or executable), then
    /// `<data dir>/oag/images`. Europe is opened before the USA; name the USA
    /// one here to play it. `$OAG_IMAGE` short-circuits the search, and finding
    /// more than one title opens the chooser - see `--launcher`.
    pub(crate) source: Option<String>,

    /// Show the disc chooser, even when a source would resolve on its own.
    ///
    /// Without this the chooser appears only when nothing named a source and
    /// the search path holds more than one image; one image boots straight in,
    /// as it always did. With it the screen is shown for a single image too.
    ///
    /// **It lists the search path, and only the search path.** A source named
    /// on the command line, in `$OAG_IMAGE` or in `settings.toml` is not a row:
    /// this is how to reach something *other* than the one configured, not a
    /// list of everything reachable. An empty search path is still the ordinary
    /// "no disc image found" error rather than an empty screen.
    ///
    /// Needs a window: `--race`, `--dry-run` and `--screenshot` all name their
    /// own source and are refused. `--drive` can walk it, with no window.
    #[arg(long)]
    pub(crate) launcher: bool,

    /// Show the chooser as a list of every disc image (platform, serial, file
    /// name) instead of one card per title, for this run. `[launcher] style =
    /// "list"` in settings.toml does the same for every run.
    #[arg(long)]
    pub(crate) launcher_list: bool,

    /// Which movie to play: an archive entry name, or `hash:XXXXXXXX` for one of
    /// the reels whose name is not recovered.
    ///
    /// Defaults to `Data\Movies\Intro.PMF`, which is what the `LogoFMV` screen
    /// plays and the only movie the disc's own boot opens, or to the European
    /// cut of the dev/pub reel under `--reel`.
    #[arg(long)]
    pub(crate) movie: Option<String>,

    /// Boot into `Intro Screen->IntroMovie1` instead of `LogoFMV`.
    ///
    /// The code-side state with the frame-counted holds at 144, 231 and 260,
    /// playing the 260-frame dev/pub reel those counters describe. The disc's
    /// own boot never enters it - see `docs/architecture/frontend-boot.md` - so
    /// this is a way to watch a real, evidenced code path, not the boot order.
    #[arg(long)]
    pub(crate) reel: bool,

    /// Convert only the first this many frames of the movie, rather than all of
    /// them.
    ///
    /// The whole 40-second intro is 33 MiB of cache and about 80 seconds of
    /// `ffmpeg`, once. `--movie-frames 261` is enough for the reel leg.
    #[arg(long)]
    pub(crate) movie_frames: Option<usize>,

    /// Do not convert the movie at all. The sequence still plays, without a
    /// picture.
    #[arg(long)]
    pub(crate) no_video: bool,

    /// Where converted frames are cached.
    #[arg(long)]
    pub(crate) cache: Option<std::path::PathBuf>,

    /// Convert every movie again even when the cache already holds it,
    /// overwriting the cached file.
    ///
    /// **For a cache written by a build whose conversion has since changed.**
    /// The movie cache is keyed by the *source* bytes - the WAD name hash and
    /// the entry size - so a fixed transcode produces the same key as the wrong
    /// one it replaces, and every later run would reuse the stale file forever.
    /// This is the way out, and clearing the cache directory by hand is the
    /// other.
    ///
    /// Only affects movies that are transcoded at all: on a `native-video`
    /// build the H.264 reels decode through GStreamer and write no cache file,
    /// so there is nothing there for this to ignore. It converts nothing that
    /// would not otherwise have been converted - it only refuses the shortcut.
    ///
    /// The sounds are untouched. `--prefetch` still skips every ATRAC3+ stream
    /// it finds cached, which is five seconds of the wait rather than ten
    /// minutes of it.
    #[arg(long)]
    pub(crate) refresh_video: bool,

    /// Build the AV1 cache for the boot's movies rather than letting this
    /// build's platform decoder handle them.
    ///
    /// **You need this once.** A cache file that already exists is preferred
    /// over the platform decoder with no flag at all, so this is the run that
    /// creates one; every run after it picks the file up by itself. `--prefetch`
    /// does the same thing for every movie on the disc rather than just the
    /// boot's.
    ///
    /// Only affects the PSP's H.264 `.PMF` reels on a Linux `native-video`
    /// build; the PS2's `.PSS` and `.IPF` have no platform decoder and always
    /// went through the cache. For an uncached first run the platform decoder
    /// stays the default - see
    /// [ADR-0017](../../docs/architecture/adr/0017-gstreamer-native-video.md).
    ///
    /// The trade is a one-off transcode against a faster start every time after
    /// it. GStreamer decodes every wanted frame into memory before it returns;
    /// the cache is opened lazily and decoded a frame at a time. Measured on
    /// this workspace against the EU PSP disc, both reels:
    ///
    /// | | Media phase |
    /// | --- | --- |
    /// | GStreamer, nothing cached | **4.80 s**, and again on every boot |
    /// | This flag, nothing cached | **36.08 s** once (intro 30.33, backdrop 5.74) |
    /// | A cache file present - no flag needed | **0.05 s** |
    ///
    /// So it pays for itself after about eight boots.
    #[arg(long)]
    pub(crate) prefer_av1_cache: bool,

    /// Convert every movie and every sound on the disc up front, on a
    /// background thread, instead of one at a time on first use.
    ///
    /// **Off by default and deliberately so.** Converting lazily is what makes
    /// a cold boot half a second rather than ten minutes; this is the opposite
    /// trade, taken once, so that nothing afterwards ever waits. Measured on
    /// this workspace: 5.5 s for all 93 ATRAC3+ streams and about ten minutes
    /// for the ~13,000 movie frames, of which `Data\Movies\Intro.PMF` alone is
    /// 57 s.
    ///
    /// The game boots and runs normally while it happens, and progress goes to
    /// stdout. Interrupting is safe: both caches are keyed by content, so the
    /// next run picks up whatever finished. With `--dry-run` it is the whole run, no window: `crate::prefetch`.
    #[arg(long)]
    pub(crate) prefetch: bool,
}
