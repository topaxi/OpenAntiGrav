//! What Pulse's own front end is called, and when its dev/pub reel acts.
//!
//! Literals, in the [ADR-0022] sense: every state name here is a string from the
//! original's executable or from the front-end XML on the disc, and every frame
//! count was read out of `0x088d7e1c`. The state *machine* that drives them is
//! `oag_game::frontend` and stays there - a second title with different screen
//! names would reuse the whole of it.
//!
//! Where this build's sequence diverges from the disc's own is documented at
//! `docs/architecture/frontend-boot.md`, and the divergences are the composition
//! root's, not this table's.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// Frame counts at which the dev/pub reel state acts, from `0x088d7e1c`.
pub const PAUSE_FRAMES: [usize; 2] = [144, 231];
/// The frame at which the dev/pub reel state sets its finish flag.
pub const FINISH_FRAME: usize = 260;
/// How long a pause is held, in seconds.
pub const HOLD_SECONDS: f64 = 2.0;

/// State names this build drives. Every one is a literal from the original -
/// from its executable, or from the front-end XML on the disc.
pub mod states {
    /// The screen that plays `Data\Movies\Intro.PMF`, named by the front-end
    /// XML. This is the leg the disc's own boot runs.
    pub const LOGO_FMV: &str = "LogoFMV";
    /// The dev/pub reel's parent state.
    pub const INTRO: &str = "Intro Screen";
    /// The dev/pub reel state, spelled the way the original's own string
    /// literal is. Not on the disc's boot path; see
    /// `oag_game::frontend`'s module docs.
    pub const INTRO_MOVIE: &str = "Intro Screen->IntroMovie1";
    /// The one-shot redirect the reel state fires when it finishes or is
    /// skipped.
    pub const DEV_PUB_REDIRECT: &str = "DevPubRedirect";
    /// The one-shot redirect `LogoFMV`'s five skip buttons fire.
    ///
    /// A real screen in the front-end XML, and it holds nothing but a single
    /// `Redirect` with `forward="none"` - so its whole job is to leave, the
    /// same shape as [`DEV_PUB_REDIRECT`]. On the disc it goes to
    /// [`SHOW_LOGO`], which is where `LogoFMV`'s own `AutoRedirect` goes too;
    /// here it goes to [`LANGUAGE_SELECTION`], because the picker is what this
    /// build has next. See `oag_game::frontend`'s module docs.
    pub const LOGO_FMV_REDIRECT: &str = "LogoFMVRedirectScreen";
    /// The Pulse logo and PRESS START.
    ///
    /// **Spelled bare rather than as a path, and that is evidence rather than
    /// convenience.** In the XML this screen is nested three deep, at
    /// `Top FE Screen->FE Screen->Show Logo`, but the running machine's own
    /// current-state buffer reads `"Show Logo"`: at `0x08d0a820+0x18c` it was
    /// caught holding `"Show Logo\0election\0"`, the shorter name overwriting
    /// `"Language Selection"` in place. Every `goto` that reaches it spells it
    /// bare as well. Confidence **95**, runtime-observed - see
    /// `docs/ghidra/functions/psp-pulse-usa/main-loop.md`. What the buffer does
    /// with a name that genuinely needs its parents is not established, so
    /// nothing here concludes that the original's states are flat.
    pub const SHOW_LOGO: &str = "Show Logo";
    /// The language picker.
    pub const LANGUAGE_SELECTION: &str = "Language Selection";
    /// Where picking a language goes, and where a race starts.
    pub const LAUNCH_GAME: &str = "Launch Game";
}
