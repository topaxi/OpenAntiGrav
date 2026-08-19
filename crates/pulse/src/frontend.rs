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

/// Pulse's boot sequence, cold-boot measured.
///
/// **The chain is the runtime's, not the XML's, and they disagree.** The disc's
/// own `Skin.xml` runs `Language Selection` first, its `LanguageAutoRedirect`
/// going on to `LogoFMV`. A cold boot of `pulse-psp-eu.chd` does not: the first
/// frame after power-on is *inside* `Data\Movies\Intro.PMF` - the movie's own
/// SCEE presents card, in a proportional font rather than the front end's bitmap
/// one - and the sequence runs on to `Show Logo` with no picker in between.
/// Confidence **90**, observed 2026-08-10; frames under
/// `data/shots/pulse-cold-boot-2026-08-10/`.
///
/// This is the measurement that makes a boot sequence a per-title table rather
/// than anything derivable: the same question asked of Pure's XML gets the right
/// answer, and asked of Pulse's gets the wrong one. See [ADR-0023].
///
/// **Where the picker belongs on Pulse is still open.** This build puts it
/// between `LogoFMV` and `Show Logo`, which is where it has always been and is
/// the order that was asked for; the disc showed no picker there on a cold boot,
/// and what it does with an unset language is not established. Recorded in the
/// chain as this build's order, flagged here as unfinished rather than evidenced.
///
/// [ADR-0023]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0023-boot-sequence-as-title-data.md
pub const BOOT_PROFILE: &oag_title::BootProfile = &oag_title::BootProfile {
    // Cold-booted under PPSSPP, and **this title is why the field exists**: the
    // disc's own `Skin.xml` declares the picker first and its runtime opens on
    // `LogoFMV` instead, so the declaration and the measurement disagree here in
    // exactly the way that makes reading one for the other a real bug.
    provenance: oag_title::Provenance::Measured,
    chain: &[
        oag_title::BootStep::playing(states::LOGO_FMV, crate::names::INTRO_MOVIE),
        oag_title::BootStep::screen(states::LANGUAGE_SELECTION),
        oag_title::BootStep::screen(states::SHOW_LOGO),
    ],
    // `Intro Screen->IntroMovie1` is real, evidenced code at `0x088d7e1c` with
    // its own frame holds - and its trigger is unknown, so it is reached by
    // `--reel` rather than at boot. See this module's own docs.
    reel: Some(oag_title::BootStep::playing(
        states::INTRO_MOVIE,
        crate::names::DEVPUB_REEL,
    )),
    menu_backdrop: Some(crate::names::BACKDROP_MOVIE),
    // The picker carries its own background on this title.
    picker_backdrop_parent: None,
    // Pulse's own `Skin.xml` declares every global its screens name.
    fallback_globals: &[],
};

/// How Pulse lays its menus out and moves between them.
///
/// **Presentation, not structure.** The tree these numbers arrange is this
/// project's own (`assets/ui/menu.toml`, and `docs/architecture/menus.md` for
/// why); what is Pulse's here is where the rows sit, what colour they are, and
/// how a page change moves.
///
/// Every value is either read out of `Data\Plugins\PI001\GUI\Skin.xml`'s
/// `FEGlobals` block and `MainMenu_Definition.xml`, or measured off a capture
/// of the original under an emulator. `oag_title::MenuSkin`'s own field docs say
/// which is which, and `docs/ui/menus-original.md` carries the captures and the
/// confidence scores.
pub const MENU_SKIN: &oag_title::MenuSkin = &oag_title::MenuSkin {
    // The PSP's screen. **The PS2 pressing is not a second answer here**: this
    // table was read off `pulse-psp-usa.chd`, so 480x272 is where its numbers
    // came from whichever pressing is mounted, and the PS2's own 640x448 grid
    // is the source's rather than the title's. The caller scales one into the
    // other; see `oag_title::MenuSkin::space`.
    //
    // Reading the PS2 `Skin.xml`'s own `FEGlobals` would settle whether that
    // scaling is what the disc does - `oag_game::frontend::Space` records that
    // 30 of the 43 coordinates the two files share land within a pixel of the
    // ratio and 13 do not, so it is an approximation and not a derivation.
    space: (480.0, 272.0),
    // `FEGlobals->MenuXOffset`, and a capture puts the rows' left edge at
    // exactly 50.0.
    menu_x: 50.0,
    menu_scale: 1.0,
    title_x: 50.0,
    title_y: 0.0,
    title_scale: 1.0,
    // `MainMenu_Definition.xml`'s menu widget, `y="32"`.
    first_row_y: Some(32.0),
    // Measured, not authored - see the field's own docs for the four menus this
    // came off and why the authored `gap` is not it.
    row_extra_leading: Some(6.0),
    menu_font: Some("menu"),
    text: Some(0xFF33_A6B9),
    title: Some(0xFF00_0000),
    // Measured. The XML states no selected colour at all.
    selected: Some(0xFF9D_FFFF),
    transition_secs: 0.5,
    // **A measurement, not a gap.** Pulse's main menu is a vertical `<Menu>`,
    // and no `<HorizMenu>` exists anywhere on either pressing: every blob of
    // `Data.wad`, `FE.wad` and `FEData.wad` on `pulse-psp-usa.chd` (1,411
    // files), of `Data.wad` on `pulse-psp-eu.chd` (1,138) and of `WADSP.WAD` on
    // `pulse-ps2-eu.chd` (193) was searched, and the literal appears in none of
    // them. See `oag_title::MenuStrip` for the whole census and for why a string
    // search finds a shortened element.
    strip: None,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_boot_leg_plays_the_movie_the_disc_opens() {
        assert_eq!(BOOT_PROFILE.start().state, states::LOGO_FMV);
        assert_eq!(
            BOOT_PROFILE.start().movie,
            Some(crate::names::INTRO_MOVIE),
            "cold-boot confirmed: the first frame after power-on is inside this movie"
        );
    }

    #[test]
    fn nothing_plays_after_the_picker() {
        // The regression guard for "Show Logo is static". It is Pulse's own
        // counterpart to Pure's `FMV Intro`, and the difference between the two
        // titles here is exactly what the profile exists to carry.
        let after = BOOT_PROFILE
            .next_after(states::LANGUAGE_SELECTION, |_| true)
            .expect("the picker is not the last screen");
        assert_eq!(after.state, states::SHOW_LOGO);
        assert!(after.movie.is_none());
    }

    #[test]
    fn the_reel_names_the_entry_this_crate_hashes() {
        let reel = BOOT_PROFILE
            .reel
            .expect("Pulse has an evidenced reel state");
        assert_eq!(reel.state, states::INTRO_MOVIE);
        assert_eq!(reel.movie, Some(crate::names::DEVPUB_REEL));
        assert_eq!(
            crate::names::DEVPUB_REEL,
            r"Data\Movies\IntroMovieP1_EU.PMF",
            "the name was recovered from the localised movie widgets' suffix rule"
        );
    }

    #[test]
    fn pulse_ships_a_menu_backdrop_and_declares_all_its_globals() {
        assert_eq!(
            BOOT_PROFILE.menu_backdrop,
            Some(crate::names::BACKDROP_MOVIE)
        );
        assert!(BOOT_PROFILE.fallback_globals.is_empty());
        assert!(BOOT_PROFILE.picker_backdrop_parent.is_none());
    }
}
