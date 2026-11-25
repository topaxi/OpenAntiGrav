//! Drawing the loading screen under `--screenshot`.

/// Drawing the loading screen under `--screenshot`.
#[derive(clap::Args, Debug)]
pub(crate) struct LoadingArgs {
    /// With `--screenshot`, draw the loading screen at a stated conversion
    /// state instead of the sequence: `DONE/TOTAL`, e.g. `37/115`.
    ///
    /// The same kind of debugging view `--screen` and `--menu-page` are, and it
    /// exists for the same reason plus one of its own: a warm cache passes
    /// through a real mid-conversion state in a fraction of a second and a cold
    /// one takes ten minutes to leave it, so neither is a way to look at this
    /// screen. What is drawn is the window's own layout with a stated input.
    ///
    /// `--ticks` chooses the frame, which is what picks the wave's beat and the
    /// tip on show: the heartbeat peaks at 5 and 11 of its 24 frames.
    #[arg(long, value_name = "DONE/TOTAL")]
    pub(crate) loading_screen: Option<String>,

    /// With `--loading-screen`, draw a stated phase instead of `--prefetch`'s:
    /// `race`, or the boot's own media phase at a load step - `cached`,
    /// `decoding`, or `transcoding:DONE/TOTAL` (e.g. `transcoding:340/1200`).
    ///
    /// `race` is the screen between the menus and the grid, which is the
    /// title's own rather than this build's: Wipeout HD's full-screen still and
    /// its `FE_LOADINGDOT` caption, Pulse's wave and one of its tips. It counts
    /// nothing, so the `DONE/TOTAL` given to `--loading-screen` is ignored and
    /// `--track` supplies the circuit name instead.
    ///
    /// **Pair it with `--ticks 0`.** Any `DONE/TOTAL` where the two are equal
    /// reads as finished, and a finished screen is fading - so a later tick
    /// captures this phase part-way through its fade rather than at full
    /// opacity.
    ///
    /// The counts above stay the loads - `2/5` is two of five movie loads done -
    /// and this is what the one in flight is doing. Stated for the same reason
    /// they are: a cache hit is over in milliseconds and a frame counter holds
    /// one value for about a second, so neither is a state a capture can be
    /// timed to catch.
    #[arg(long, value_name = "STEP", requires = "loading_screen")]
    pub(crate) loading_step: Option<String>,

    /// With `--loading-screen` and `--loading-step race`, run a real race load
    /// of `--track` on a worker, paced at 60 Hz, and draw the bar at the stage
    /// the load has reached by `--ticks`. How a capture sees a stage-driven bar
    /// step, which no stated state can show.
    #[arg(long, requires = "loading_screen")]
    pub(crate) loading_live: bool,

    /// With `--loading-screen` on Wipeout 2048, draw the race load of a campaign event
    /// launched from its card: `EVENT` or `EVENT:PAGE` (the card's page when Launch was
    /// pressed, 0 the first), e.g. `2048 - Event 2`. `--ticks` is the load's age at 60 Hz and
    /// `--loading-stage` how far the loader has got.
    #[arg(long, value_name = "EVENT[:PAGE]", requires = "loading_screen")]
    pub(crate) loading_card: Option<String>,

    /// With `--loading-screen`, the load stage reported to the bar from the first tick (0 to 4):
    /// the stated counterpart of `--loading-live`'s real stages.
    #[arg(long, default_value_t = 0, requires = "loading_screen")]
    pub(crate) loading_stage: u8,

    /// With `--loading-screen`, the seed the feature is drawn with (default 0). The
    /// original draws a feature at random each time the screen goes up
    /// (`LoadingScreen_Construct`, `Libc_Rand`), so a capture that sits beside one of its frames
    /// names the draw the frame showed: the position in the mode's deck, modulo what the deck
    /// holds, of this seed's first number. Prints the feature it drew.
    #[arg(long, default_value_t = 0, requires = "loading_screen")]
    pub(crate) loading_draw: u64,
}
