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
    /// Pure's language picker, and **the screen its boot opens on**.
    ///
    /// The same literal Pulse's XML uses, restated here rather than borrowed
    /// from `oag_pulse`: a title package must not read another's, and two discs
    /// agreeing on a string is not the same as one of them owning it.
    ///
    /// Cold-boot confirmed on both pressings, 2026-08-10: the first frame after
    /// power-on is this screen, with **no movie before it**. Confidence **90**.
    /// See `docs/architecture/pure-boot.md`, which also records why an earlier
    /// reading of this got it wrong.
    pub const LANGUAGE_SELECTION: &str = "Language Selection";

    /// The picker's parent, and where its white background comes from.
    ///
    /// `<Screen type="Intro" name="Intro Screen">` opens with a comment reading
    /// `<?Background Image white?>` and a full-screen `Image` of `0xFFFFFFFF`.
    /// The picker is a child screen carrying no fill of its own, so without the
    /// parent's it would draw on black. This screen also holds all four of the
    /// title's `Movie` widgets.
    pub const INTRO_SCREEN: &str = "Intro Screen";

    /// The dev/pub reel, between the picker and [`MEMORY_STICK_WARNING`].
    ///
    /// **It plays [`crate::names::INTRO_MOVIE`], and this is the same state
    /// Pulse's `Intro Screen->IntroMovie1` is.** The screen declares only a
    /// placeholder `Item` and a `DevPubRedirect` to `MemoryStickWarning`; the
    /// movie is the `IntroMovie1` widget on the parent `Intro Screen`, inherited
    /// the same way the picker inherits that parent's white background.
    ///
    /// Confirmed by matching captured frames against the decoded video: the
    /// "SONY COMPUTER ENTERTAINMENT ... PRESENTS" card is frame 144 and
    /// "A STUDIO LIVERPOOL GAME" is frame 231, each within resampling noise of
    /// the capture. Confidence **95**. Everything on those cards - the logo, the
    /// frame graphics, the barcode, even the small arrow glyph - is video
    /// content.
    ///
    /// **The frame holds are Pure's own.** `144`, `231` and `260` appear as three
    /// `li` immediates in Pure's `BOOT.BIN`, the only such site in the binary,
    /// each beginning an identical pause-and-reload block - so this title
    /// implements the pause logic itself rather than inheriting it. The hold
    /// *duration* is loaded from a global rather than an immediate, so
    /// `oag_pulse::frontend::HOLD_SECONDS` remains Pulse's measurement.
    ///
    /// An earlier reading recorded here had the cards as engine-drawn teletyped
    /// text. That was wrong, and wrong from reasoning about a screenshot's
    /// appearance instead of checking the asset; see
    /// `docs/architecture/pure-boot.md`.
    pub const DEVELOPER_PUBLISHER: &str = "Developer Publisher Screen";

    /// The storage-access disclaimer, waiting on cross.
    ///
    /// Unlike [`DEVELOPER_PUBLISHER`], this screen's content **is** in the XML:
    /// six `MSInfo`/`MSWarning` texts, two rule `Image`s, the `MSWarningScale`
    /// and `MSWarningColour1`/`2` globals, and `MemoryStickRedirect`
    /// (`StartEnabled="true"`, `Default goto="FMV Intro"`). Runtime-confirmed to
    /// hold until cross is pressed - "PRESS X TO CONTINUE" - rather than timing
    /// out. Confidence **90**.
    ///
    /// **This build deliberately rewords its text.** The disc's strings are
    /// about a Memory Stick Duo being physically removed; this is a
    /// reimplementation on hardware where storage is assumed present, so the
    /// screen keeps its structure, colours and gate but takes its own wording,
    /// phrased around autosave. A product decision, recorded so it is not
    /// "fixed" back to the disc's strings later.
    pub const MEMORY_STICK_WARNING: &str = "MemoryStickWarning";

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
    /// **The button that leaves it is measured; where it goes is not.** The
    /// screen declares a text widget whose `idstring` is the literal
    /// `PRESS START`, drawn beside a blinking `_` in the same face, and the
    /// session above watched the firmware hold here until START - so reading
    /// START is evidence, at the same 90 as the rest of this comment.
    ///
    /// Its destination is a different question and still open. The screen's only
    /// `Redirect` (`TitleRedirect`) carries no `forward` attribute at all -
    /// unlike `Show Logo`'s explicit `forward="start"` - and its
    /// `Default goto="Profile Manager"` names a screen this build does not have.
    /// `oag_game::frontend::Frontend::update_title_screen` therefore fires
    /// `Launch Game` and opens the menus, which is a **deliberate divergence**
    /// and exactly the one Pulse already makes from `Show Logo`. See
    /// `docs/architecture/pure-boot.md`.
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
/// (`FEGlobals->TitleColor`, `FEGlobals->DesignColor`, `FEGlobals->TextColor`,
/// `FEGlobals->FrameLineColor`) that `Data\Plugins\PI001\GUI\Skin.xml` never
/// declares - the disc's own copy authors exactly nine globals (`TitleScale`,
/// `TitleXOffset`, `TitleYOffset`, `MenuScale`, `MenuXOffset`,
/// `MSWarningScale`, `MSWarningColour1`, `MSWarningColour2`,
/// `FMVFrameCount`), confirmed by reading the file directly - zero of
/// `FrameLineColor`'s own seventeen usages on `Title Screen` are backed by a
/// declaration either, checked the same way. `TextColor` is the same gap
/// `HANDOVER.md` already had on file, now with a measured value rather than
/// only a citation; `TitleColor`/`DesignColor`/`FrameLineColor` are `Title
/// Screen`'s own. Either an unmerged `LoadXML` base skin defines all four, or
/// the original engine carries a compiled-in default table this project has
/// not found. Neither is resolved; this is a **measured stand-in**, not a
/// derivation.
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
/// **`FrameLineColor` is measured the same way, off `Title Screen`'s own
/// corner-bracket lines around the `START` button, at confidence 60 rather
/// than 65.** A screenshot at native `2x` resolution (`--xres 960 --yres
/// 544`, eliminating the fractional-scale blending a windowed capture
/// otherwise has) still shows every line as two rows, not one - `RGB(179,
/// 215, 226)` and a lighter `RGB(229, 241, 245)` in equal counts across the
/// whole visible line, which reads as the renderer's own sub-pixel placement
/// rather than a scaling artefact, since exact `2x` has nothing left to blend
/// with. The darker row is taken as the widget's own colour and the lighter
/// as a partial-coverage edge; that split, not just the usual antialiasing
/// margin, is the extra point of uncertainty the lower confidence covers.
///
/// Consulted by `oag_game::boot::load`, merged in only for keys Pure's own
/// `Skin.xml` left undeclared - a real declaration always wins.
pub const FALLBACK_GLOBALS: &[(&str, &str)] = &[
    ("TitleColor", "0xFFED4896"),
    ("DesignColor", "0xFF64DCF6"),
    ("TextColor", "0xFF88D6E8"),
    ("FrameLineColor", "0xFFB3D7E2"),
];

/// How Pure lays its menus out, as far as its own disc states it.
///
/// **Deliberately thinner than Pulse's, and thin in the places Pure is
/// genuinely unread.** `docs/formats/pure-status.md` holds this crate to what
/// has been measured, and four things separate the two titles here:
///
/// - Pure's `Skin.xml` is plain `<?xml`, not `<code>`-shortened, and it
///   declares **six** layout globals where Pulse declares dozens.
/// - There is **no `MainMenu_Definition.xml`** on this disc. Its `LoadXML` list
///   names twelve other `*_Definition.xml` files instead, so
///   [`oag_title::MenuSkin::first_row_y`] below is measured across all of them
///   rather than off one named screen the way Pulse's `y="32"` is.
/// - [`oag_title::MenuSkin::row_extra_leading`] stays `None`. Pure's 33 `<Menu>`
///   widgets carry exactly **one** `gap` between them (`gap="25"`), and that
///   field's own docs already record that `gap` is not row pitch on Pulse. No
///   capture of a Pure menu exists to measure a pitch off instead.
/// - There is **no `LeftLayer` element anywhere** in Pure's GUI definitions,
///   so where its `transition` attaches is unknown even though the durations
///   are authored.
///
/// The numbers that *are* here were read off `pure-psp-eu.chd`'s own
/// `Skin.xml`, and every one of them differs from Pulse's: `MenuXOffset` 21
/// against 50, `MenuScale` 1.15 against 1.0, `TitleScale` 0.97 against 1.0,
/// `TitleYOffset` 20 against 0. Borrowing Pulse's table would have been wrong
/// on all four.
pub const MENU_SKIN: &oag_title::MenuSkin = &oag_title::MenuSkin {
    // The PSP's screen, read off both pressings' own `Skin.xml`.
    space: (480.0, 272.0),
    menu_x: 21.0,
    menu_scale: 1.15,
    title_x: 21.0,
    title_y: 20.0,
    title_scale: 0.97,
    // **Authored, and measured across the whole GUI tree rather than off one
    // screen.** Pure has no `MainMenu_Definition.xml` to read a single number
    // out of, so every `<Menu>` widget in `Skin.xml` and the twelve
    // `*_Definition.xml` files its `LoadXML` list names was counted: 33 widgets,
    // **19 of them `y="45"`** and no other value reached four. The runners-up
    // are `y="106"` (3), `y="25"` (2) and nine singletons - screens with
    // something above the list rather than a different row origin. Confidence
    // **80**: authored and strongly dominant, but read off the files alone with
    // no capture of a Pure menu to confirm where the ink lands.
    first_row_y: Some(45.0),
    row_extra_leading: None,
    // **`None` because Pure's menus are drawn in the default face, which is a
    // measurement rather than a gap.** This field exists so a title whose rows
    // use a *bigger* face than its body text says so - Pulse's rows are
    // `font="menu"`, which its language plugins resolve to `Pulse_20.fnt` where
    // the default is `pulse_text.fnt`. Pure has no such split, on two counts that
    // agree:
    //
    // - Of its 33 `<Menu>` widgets, only five name a font at all - two
    //   `font="Default"` and three `font="InGame"` on the in-race pause menu -
    //   and the remaining 28 name none, which is the default face too.
    // - Its language plugins fill in **eight** `<Font>` slots, the same number
    //   Pulse's do, but the set differs by one: Pulse has `Menu` and Pure has
    //   `Scroll` in its place. There is no `Menu` slot on this disc to resolve.
    //
    // Pure's `Scroll` face (`LTe50325.fnt`) is not the menus' - `Skin.xml` uses
    // it for the `Confirm button` and `Back button` prompt rows - so nothing here
    // substitutes it. Pinned across both titles by
    // `crates/game/tests/font_roles_ground_truth.rs`.
    menu_font: None,
    // The same pixel measurements [`FALLBACK_GLOBALS`] carries, restated in the
    // units this type uses. Confidence 65, and of the *effect* rather than of
    // the source - see that constant's own docs.
    text: Some(0xFF88_D6E8),
    title: Some(0xFFED_4896),
    // No capture of a Pure menu with a cursor on it exists.
    selected: None,
    // Authored, but as one of four values this disc uses (0, 0.25, 0.3, 0.5)
    // rather than the single dominant one Pulse has. 0.5 is the most common of
    // them in the five definition files read; which value a *menu* page change
    // takes is unread, because the element carrying it on Pulse does not exist
    // here.
    transition_secs: 0.5,
    // **A measurement, not a gap**, and on this title it is the stronger of the
    // two readings: Pure's menus are the 33 vertical `<Menu>` widgets
    // `first_row_y` above was counted off, and searching every blob of
    // `Data.wad`, `FE.wad` and `FEData.wad` on `pure-psp-eu.chd` - 1,241 files,
    // 46 of them naming `Menu` at all - finds no `HorizMenu` anywhere. See
    // `oag_title::MenuStrip`.
    strip: None,
};

/// Pure's boot sequence, cold-boot measured on both pressings.
///
/// The chain is what the disc's own `Skin.xml` declares, redirect for redirect,
/// **and** what its runtime does - the two agree here, which is what puts it at
/// confidence 90. That agreement is not a general rule: Pulse's XML declares an
/// entry point its runtime does not use, which is the whole reason this is a
/// measured table per title rather than something derived. See
/// `docs/architecture/pure-boot.md` and [ADR-0023].
///
/// **The boot step plays no movie, and the step after the picker does.** Nothing
/// runs before `Language Selection`; `crate::names::INTRO_MOVIE` - the dev/pub
/// reel, byte-for-byte the one Pulse ships - plays on
/// [`states::DEVELOPER_PUBLISHER`] straight after it. So this title's boot is
/// **two** movies, not one, and the reel is very much on its path.
///
/// [ADR-0023]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0023-boot-sequence-as-title-data.md
pub const BOOT_PROFILE: &oag_title::BootProfile = &oag_title::BootProfile {
    // Cold-booted on both pressings, with the savedata profile moved aside; see
    // `docs/architecture/pure-boot.md`. Pure's runtime *does* agree with its own
    // XML about the entry point, which is a second measurement rather than a
    // reason to trust the next title's declaration.
    provenance: oag_title::Provenance::Measured,
    chain: &[
        oag_title::BootStep::screen(states::LANGUAGE_SELECTION),
        oag_title::BootStep::playing(states::DEVELOPER_PUBLISHER, crate::names::INTRO_MOVIE),
        oag_title::BootStep::screen(states::MEMORY_STICK_WARNING),
        oag_title::BootStep::playing(states::FMV_INTRO, crate::names::FMV_INTRO_MOVIE),
        oag_title::BootStep::screen(states::TITLE_SCREEN),
    ],
    // Pure's reel state is on its boot path, so `--reel` has a real target here -
    // and a better-evidenced one than Pulse's, whose own reel state is never
    // entered at boot. Naming it means the flag shows the same screen the disc
    // shows rather than being refused.
    reel: Some(oag_title::BootStep::playing(
        states::DEVELOPER_PUBLISHER,
        crate::names::INTRO_MOVIE,
    )),
    // Neither pressing carries `Data\Movies\Backdrop.PMF`.
    menu_backdrop: None,
    picker_backdrop_parent: Some(states::INTRO_SCREEN),
    fallback_globals: FALLBACK_GLOBALS,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// The fact an earlier pass got wrong, pinned as a compiled assertion.
    ///
    /// A stale PPSSPP save profile was skipping the real boot, and this build
    /// played `IntroMovieP1_US.PMF` before the picker on that basis. A true cold
    /// boot of both pressings opens on the picker with nothing before it.
    #[test]
    fn pures_boot_plays_no_movie() {
        assert_eq!(BOOT_PROFILE.start().state, states::LANGUAGE_SELECTION);
        assert!(
            BOOT_PROFILE.start().movie.is_none(),
            "cold-boot confirmed on both pressings; see docs/architecture/pure-boot.md"
        );
    }

    /// Two movies, and which screens play them.
    ///
    /// The dev/pub reel was recorded here for a while as "not on this title's boot
    /// path", on the strength of `Developer Publisher Screen` declaring no widgets
    /// of its own. It inherits the parent's, and frames 144 and 231 of that reel
    /// are the two cards the screen shows.
    #[test]
    fn the_chain_plays_two_movies_and_names_the_screens_that_play_them() {
        let movies: Vec<_> = BOOT_PROFILE
            .chain
            .iter()
            .filter_map(|step| step.movie.map(|movie| (step.state, movie)))
            .collect();
        assert_eq!(
            movies,
            vec![
                (states::DEVELOPER_PUBLISHER, crate::names::INTRO_MOVIE),
                (states::FMV_INTRO, crate::names::FMV_INTRO_MOVIE),
            ]
        );
    }

    /// The reel state is on the boot path, which is the whole correction.
    #[test]
    fn the_reel_is_a_step_of_the_boot_rather_than_an_off_path_curiosity() {
        let reel = BOOT_PROFILE.reel.expect("Pure plays its reel at boot");
        assert_eq!(reel.state, states::DEVELOPER_PUBLISHER);
        assert_eq!(reel.movie, Some(crate::names::INTRO_MOVIE));
        assert_eq!(
            BOOT_PROFILE
                .step(states::DEVELOPER_PUBLISHER)
                .map(|s| s.movie),
            Some(reel.movie),
            "--reel shows the screen the disc itself shows, rather than being refused"
        );
    }

    /// Every cut is a `_<REGION>` spelling of the same two names.
    #[test]
    fn the_regional_cuts_share_one_naming_rule() {
        for (region, name) in crate::names::INTRO_MOVIE_CUTS {
            assert_eq!(*name, format!(r"Data\Movies\IntroMovieP1_{region}.PMF"));
        }
        for (region, name) in crate::names::FMV_INTRO_MOVIE_CUTS {
            assert_eq!(*name, format!(r"Data\Movies\WoFMVNew_{region}.PMF"));
        }
        assert!(
            crate::names::INTRO_MOVIE_CUTS
                .iter()
                .any(|(_, name)| *name == crate::names::INTRO_MOVIE),
            "the cut this build picks is one of the four it knows"
        );
    }

    #[test]
    fn the_chain_is_the_five_screens_the_disc_walks() {
        let states: Vec<_> = BOOT_PROFILE.chain.iter().map(|step| step.state).collect();
        assert_eq!(
            states,
            vec![
                "Language Selection",
                "Developer Publisher Screen",
                "MemoryStickWarning",
                "FMV Intro",
                "Title Screen",
            ]
        );
    }

    #[test]
    fn pure_ships_no_menu_backdrop() {
        assert!(BOOT_PROFILE.menu_backdrop.is_none());
    }

    #[test]
    fn the_picker_inherits_its_parents_background() {
        assert_eq!(
            BOOT_PROFILE.picker_backdrop_parent,
            Some(states::INTRO_SCREEN),
            "the picker carries no fill of its own and would draw on black"
        );
    }
}
