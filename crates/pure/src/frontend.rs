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

/// `FEGlobals` colours Pure's own screens reference
/// (`FEGlobals->TitleColor`, `FEGlobals->DesignColor`, `FEGlobals->TextColor`)
/// that `Data\Plugins\PI001\GUI\Skin.xml` never declares - the disc's own
/// copy authors exactly nine globals (`TitleScale`, `TitleXOffset`,
/// `TitleYOffset`, `MenuScale`, `MenuXOffset`, `MSWarningScale`,
/// `MSWarningColour1`, `MSWarningColour2`, `FMVFrameCount`), confirmed by
/// reading the file directly. `TextColor` is the same gap `HANDOVER.md`
/// already had on file, now with a measured value rather than only a
/// citation; `TitleColor`/`DesignColor` are `Title Screen`'s own. Either an
/// unmerged `LoadXML` base skin defines all three, or the original engine
/// carries a compiled-in default table this project has not found. Neither
/// is resolved; this is a **measured stand-in**, not a derivation.
///
/// **Measured by sampling pixels**, not read from any XML: driving
/// `pure-psp-usa.chd` under PPSSPP (the same session `states::TITLE_SCREEN`
/// was evidenced in) and reading the rendered text directly off captured
/// frames. `TitleColor` (`Title Screen`'s "PRESS START BUTTON") samples solid
/// at RGB(237, 72, 150); `DesignColor` (`Title Screen`'s "HOLD ON!") at
/// RGB(100, 220, 246); `TextColor` (`Language Selection`'s unselected language
/// rows, e.g. `ESPAÑOL`) at RGB(136, 214, 232). `DesignColor` is close enough
/// to Pulse's own `DesignColor` (`0xFF5FDBF6`, from `pulse-psp-usa.chd`'s
/// `Skin.xml`) that the two may be the same constant - unconfirmed, and not
/// assumed here; every value below is Pure's own measurement, not borrowed
/// from Pulse's. `TitleColor` does **not** match Pulse's own same-named
/// global (`0xFF000000`, black) - the two titles genuinely differ here, so
/// borrowing Pulse's table wholesale would have been wrong. Confidence
/// **65**: a real, repeatable pixel measurement, but of the *effect* rather
/// than of the *source* - the true value could differ slightly from what
/// antialiasing and video compression left in a captured frame, and the
/// mechanism that is supposed to supply it is still unknown.
///
/// Consulted by `oag_game::boot::load`, merged in only for keys Pure's own
/// `Skin.xml` left undeclared - a real declaration always wins.
pub const FALLBACK_GLOBALS: &[(&str, &str)] = &[
    ("TitleColor", "0xFFED4896"),
    ("DesignColor", "0xFF64DCF6"),
    ("TextColor", "0xFF88D6E8"),
];
