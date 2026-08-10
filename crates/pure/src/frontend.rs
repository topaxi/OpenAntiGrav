//! What Pure's own front end is called, at the one seam this build reaches.
//!
//! Literals, in the [ADR-0022] sense: every name here is a screen name read
//! straight off `Data\Plugins\PI001\GUI\Skin.xml`. The state *machine* that
//! drives them is `oag_game::frontend`, the same one Pulse's boot leg uses -
//! see that module's own docs, which anticipated a second title reusing it.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// State names this build drives, all literal screen names from the disc.
pub mod states {
    /// The WipEout Pure logo and PRESS START - Pure's own counterpart to
    /// Pulse's `Show Logo`.
    ///
    /// **Runtime-observed, not read off the XML alone.** Driving
    /// `pure-psp-usa.chd` under PPSSPP (Xvfb, `SDL_VIDEODRIVER=x11`,
    /// `ws://127.0.0.1:47821/debugger`, 2026-08-10) and confirming a language
    /// walks the real firmware through the disc's own literal chain with no
    /// reordering trick: `Language Selection` -> (blank) `Developer Publisher
    /// Screen` -> `MemoryStickWarning` ("PRESS X TO CONTINUE") -> `FMV Intro`
    /// (a movie) -> here, showing "WipEout pure" / "PRESS START BUTTON". This
    /// is the one difference from Pulse's own boot, which reorders instead of
    /// following its XML literally - see `oag_game::frontend`'s module docs.
    /// Confidence **90**: directly observed on the real emulator, screen name
    /// confirmed against `Data\Plugins\PI001\GUI\Skin.xml`'s own
    /// `<Screen type="Title" name="Title Screen">`.
    ///
    /// **What advances past it is not established.** The screen's only
    /// `Redirect` (`TitleRedirect`) carries no `forward` attribute at all -
    /// unlike `Show Logo`'s explicit `forward="start"` - so whatever leaves
    /// this screen on the original is not modelled here, the same way Pulse's
    /// own unnamed `Show Logo` redirect is left unfired rather than guessed.
    /// Its `Default goto="Profile Manager"`, a screen this build does not
    /// have either.
    pub const TITLE_SCREEN: &str = "Title Screen";

    /// The screen that plays the second boot movie
    /// (`oag_pure::names::FMV_INTRO_MOVIE`), between the language picker and
    /// [`TITLE_SCREEN`].
    ///
    /// **Runtime-observed**, the same session and method as `TITLE_SCREEN`'s
    /// own doc comment: pressing START while its arrow-logo animation plays
    /// advanced straight to `Title Screen`. Matches the screen's own single
    /// `Redirect` (`FMVRedirect`, `Default goto="Title Screen"`, no named
    /// button) exactly. Confidence **90**, same basis as `TITLE_SCREEN`.
    ///
    /// The screen carries no widgets of its own on the disc - just a
    /// placeholder `Item` and that one `Redirect` - so `draw_screen` on it
    /// alone is an honest blank frame, not a bug standing in for the movie.
    pub const FMV_INTRO: &str = "FMV Intro";
}
