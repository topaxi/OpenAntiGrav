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
    /// implements the pause logic itself rather than inheriting it. **The hold
    /// *duration* is measured too, not imported (2026-09-23):** the block reads
    /// a global, but that global is a running clock sampled to compute elapsed
    /// time, not the duration itself - the duration is a `2.0` built as a raw
    /// float immediate, identically on both pressings, and a live PPSSPP
    /// breakpoint measured the actual held span at 1.9965 s. So
    /// `oag_pulse::frontend::HOLD_SECONDS` is confirmed for Pure independently,
    /// not merely reused from Pulse's own measurement. See
    /// `docs/ghidra/functions/psp-pure-eu/devpub-reel-hold.md`.
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

    /// The screen every menu is nested inside, and the one that carries
    /// Pure's own frame: `Skin.xml`'s own comment calls it "all FE screens are
    /// inside another screen so that online screens can have no background
    /// but still have music". What it authors: rule lines top and bottom, a
    /// scroll arrow pair, a squiggle-text date/version strip, `ArrowSelect`
    /// (`FETextures.mip`) - and one `<BackgroundController>` wrapping a
    /// `BackgroundImage` and a `BackgroundTopRightImage`, **neither naming a
    /// `src`**, the same class of gap as `Title Screen`'s own missing wordmark
    /// texture, `TitleFrame`. Both resolve through a declared `FEGlobals->`
    /// global now that `BackgroundController_UpdateImages`
    /// (`docs/ghidra/functions/psp-pure-eu/title-screen.md`) is read -
    /// `hashes::MENU_TOPRIGHT_LOGO`'s doc comment has the mechanism and the
    /// literal `Data\Skins\Default\Skin.xml` declares, filled through
    /// [`FALLBACK_IMAGES`]. `BackgroundImage` resolves through the same
    /// global-lookup mechanism to the empty string that same skin declares for
    /// it - a measured "no texture" rather than an unlooked-for gap, which is
    /// what [`crate::MENU_SKIN`]'s `background` field's own flat white colour
    /// fill was already standing in for. See `docs/formats/pure-status.md` for
    /// the scan and [`super::FRONT_END`]'s own `menu_frame`.
    pub const FE_SCREEN: &str = "FE Screen";
}

/// Colour globals Pure's front end names and its own `Skin.xml` never
/// declares - **empty, because the disc authors every one of them after all.**
///
/// This table used to hold `TitleColor`, `DesignColor`, `TextColor` and
/// `FrameLineColor`, sampled off captured PPSSPP frames at confidence 60-65
/// because seventeen usages of `FEGlobals->FrameLineColor` had zero
/// declarations in `Data\Plugins\PI001\GUI\Skin.xml`. They were being looked
/// for in the wrong file. **Pure's front end is skinnable**: its plugin
/// definition activates a second `PI_Skin` at `Data\Skins\Default`, whose own
/// `Skin.xml` declares 41 globals including all four, identically on both
/// pressings. See `oag_game`'s `boot::screens::style_skin_globals` and
/// `docs/formats/race-setup.md`.
///
/// The disc disagrees with three of the four measurements: by a little on
/// `TitleColor` (`0xFFED4796` against the sampled `0xFFED4896`) and
/// `DesignColor` (`0xFF5FDBF6` against `0xFF64DCF6`), and by a lot on
/// `TextColor` (`0xFF11ACD0` against `0xFF88D6E8`). That is what sampling the
/// *effect* rather than the source costs, and why those values were only ever
/// at 65.
///
/// Kept as an empty table rather than deleted, so a name that turns out to be
/// genuinely undeclared has somewhere to go and this note stays attached to
/// the mistake it records.
pub const FALLBACK_GLOBALS: &[(&str, &str)] = &[];

/// `Image` widgets Pure's own `Skin.xml` gives no `src` at all, matched by
/// name to a fallback `src` - the same "assigned programmatically on the
/// original" gap [`FALLBACK_GLOBALS`] fills for a colour, but for a texture.
///
/// **`TitleFrame` is deliberately absent from this table.** Unlike the other
/// two entries, its own real `src` is not the same on both pressings -
/// `Screen_ConstructTitleScreen` bakes a different, region-suffixed literal
/// into each disc's own executable (see [`crate::hashes::TITLE_LOGO`] and
/// [`crate::hashes::TITLE_LOGO_EU`]) - so a single static entry in this table
/// would be right for one pressing and silently wrong for the other, the
/// exact bug [`title_frame_src`] exists to fix. Callers building a full
/// fallback list for Pure append that function's own result to this table;
/// `oag_game::boot::load_shell` is the one that does it.
///
/// **`FE Screen->BackgroundTopRightImage`, the "ワイプアウト" wordmark beside
/// the swoosh logo, resolves through the disc's own declared global** rather
/// than a hard-coded hash: `Data\Skins\Default\Skin.xml` declares
/// `<Variable global="BackgroundTopRightTexture">` with a real WAD path as its
/// value, the same `FEGlobals->Name` indirection `FrameLineColor` and
/// `TitleColor` already go through - see [`crate::hashes::MENU_TOPRIGHT_LOGO`]
/// for the mechanism read and the hash agreement that confirms it.
///
/// **`FE Screen->BackgroundImage` resolves the same way, to nothing.** The
/// same skin declares `<Variable global="BackgroundTexture">` with an empty
/// string, which is why this table names it explicitly rather than relying on
/// the widget silently having no fallback at all: an empty resolved global is
/// a measured "no texture here", not an unlooked-for gap. This is also what
/// [`crate::MENU_SKIN`]'s `background` field's own flat white colour fill was
/// already standing in for, now with the mechanism behind the emptiness read
/// rather than only the effect measured against a captured frame.
///
/// Consulted by `oag-game`'s own front-end XML loader the same way
/// `fallback_globals` is: only where a widget's own XML leaves `src` unset,
/// resolved through `Screens::resolve` exactly like a colour attribute is (so
/// a `FEGlobals->Name` entry here is followed to its declared value, and a
/// plain `hash:`/name entry passes through unchanged) - and a real `src`
/// always wins over any of this.
pub const FALLBACK_IMAGES: &[(&str, &str)] = &[
    (
        "BackgroundTopRightImage",
        "FEGlobals->BackgroundTopRightTexture",
    ),
    ("BackgroundImage", "FEGlobals->BackgroundTexture"),
];

/// `Title Screen->TitleFrame`'s own fallback `src`, for the pressing `serial`
/// names - the one entry [`FALLBACK_IMAGES`] cannot hold as a single static
/// value. See [`crate::hashes::TITLE_LOGO`]/[`TITLE_LOGO_EU`] for the
/// evidence: `Screen_ConstructTitleScreen` loads a literal,
/// region-suffixed name baked into each pressing's own executable, and the
/// two pressings' own values decode to genuinely different pictures (a
/// different wordmark colourway), not the same texture under two names.
///
/// `serial` is the disc's own `AAAA-NNNNN` normalised serial
/// ([`oag_assets::Layout::serial`]) - `None` (an extracted directory, which
/// carries no serial to read) defaults to the EU value, per this project's
/// own "prefer EU over USA" convention for a source that cannot say which
/// pressing it is. A serial this project has not measured falls back to the
/// same default rather than guessing at a third, unread, suffix.
///
/// [`TITLE_LOGO_EU`]: crate::hashes::TITLE_LOGO_EU
#[must_use]
pub fn title_frame_src(serial: Option<&str>) -> (&'static str, &'static str) {
    PRESSINGS.of(serial).title_frame
}

/// Which of [`crate::names::INTRO_MOVIE_CUTS`]/[`crate::names::FMV_INTRO_MOVIE_CUTS`]
/// a pressing's own executable resolves a `localised="true"` `<Movie>` widget
/// to - the same shape of fix [`title_frame_src`] is, on the same evidence
/// class.
///
/// `Movie_ParseAttributes` (`docs/ghidra/functions/psp-pure-eu/
/// movie-localised-suffix.md`) appends a single literal suffix - `"_EU"` on
/// the EU binary, `"_US"` on the USA one - to every `localised="true"`
/// widget's resolved name, read directly off both executables' own memory.
/// Neither binary's string table names the other's suffix, or `_JAP`/`_KO` at
/// all: there is no runtime language read here, just which executable is
/// running, exactly the mechanism `TitleFrame`'s wordmark already settled.
///
/// `serial` is [`oag_assets::Layout::serial`]; `None` or an unmeasured serial
/// default to `"EU"`, the same convention [`title_frame_src`] takes.
#[must_use]
pub fn localised_movie_region(serial: Option<&str>) -> &'static str {
    PRESSINGS.of(serial).movie_region
}

/// What each Pure pressing's own executable resolves, as [`oag_title::Title::pressings`].
///
/// Measured: `docs/ghidra/functions/psp-pure-eu/title-screen.md` (the
/// wordmark) and `movie-localised-suffix.md` (`_EU`/`_US`), read off both
/// executables. The USA serial is `oag_pure::tests`'s. The rows are measured;
/// that an unlisted serial gets the EU row is **chosen**, this project's own
/// "prefer EU" default.
pub const PRESSINGS: &oag_title::Pressings = &oag_title::Pressings {
    listed: &[oag_title::Pressing {
        serial: "UCUS-98612",
        movie_region: "US",
        title_frame: ("TitleFrame", "hash:3af18d90"),
        intro_movie: crate::names::INTRO_MOVIE_CUTS[1].1,
        fmv_intro_movie: crate::names::FMV_INTRO_MOVIE_CUTS[1].1,
    }],
    unlisted: oag_title::Pressing {
        serial: "",
        movie_region: "EU",
        title_frame: ("TitleFrame", "hash:b6677aab"),
        intro_movie: crate::names::INTRO_MOVIE_CUTS[0].1,
        fmv_intro_movie: crate::names::FMV_INTRO_MOVIE_CUTS[0].1,
    },
    origin: oag_title::Origin::Measured,
};

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
    // **Now checked, both pressings**: `Skin.xml`'s own `Main Menu` screen
    // (the same root screen `Race Campaign`/`Racebox`/`Remix`/... draws under)
    // authors `idstring="Main Menu" font="Title" x="FEGlobals->TitleXOffset"
    // y="FEGlobals->TitleYOffset" scale="FEGlobals->TitleScale"
    // color="FEGlobals->TitleColor"` - the same role/offset/scale/color
    // mechanism Pulse's and HD's chrome titles use, read directly off
    // `pure-psp-eu.chd` and `pure-psp-usa.chd`'s own `Data\Plugins\PI001\GUI\Skin.xml`,
    // 2026-09-25. Flipping this is a safe wire rather than the open risk
    // Pulse's own entry describes: `oag_ui::language::roles::TITLE` resolves
    // to `FX300ANG.fnt` for Pure, the *same file* `DEFAULT` already does (see
    // that module's own font table), so nothing about which glyphs draw
    // moves - only the fact that the title now goes through its own
    // authored role instead of drawing as an untagged `Draw::Text` that
    // happens to land on the identical atlas. Safe from `face_atlas_slot`'s
    // other user too: Pure's own campaign-free menus never construct a
    // `Draw::FacedText { role: "Default", .. }` for this to collide with
    // (see that function's own doc).
    title_font: Some("Title"),
    body_font: None,
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
    // `BackgroundController`'s own `BackgroundImage` is the widget for this,
    // sized to the full screen and never given a `src` - see
    // `states::FE_SCREEN`'s own doc for what that widget is. A captured
    // `Main Menu` (`pure-psp-usa.chd`, PPSSPP 1.20.4, 2026-08-25, driven
    // Language Selection -> Developer Publisher Screen -> MemoryStickWarning
    // -> FMV Intro -> Title Screen -> Profile -> New -> Set Name -> Main
    // Menu) shows solid white behind the row list - not the black this build
    // cleared to for want of anything else. Confidence 70: one capture, one
    // fresh profile, no second session or theme choice checked against it.
    background: Some(0xFFFF_FFFF),
    // Measured off the same `Main Menu` capture `background` is - and the
    // opposite direction from Pulse's own "brightening toward white":
    // sampling `SINGLE PLAYER` (selected) against `MULTIPLAYER`/`PROFILE`/
    // `DOWNLOAD` (not) gives a *darker*, more saturated ink for the selected
    // row - `#16AED1` (22,174,209) against `TextColor`'s own `#88D6E8`
    // (136,214,232), which the unselected rows' own sampled pixels confirm
    // to the digit. Not `OUR_SELECTED`'s white: white was invisible here
    // the moment `background` started drawing a white screen under it, which
    // is what caught this. Confidence 65, the same basis as this file's other
    // pixel measurements - one capture, effect rather than source.
    selected: Some(0xFF16_AED1),
    // Whether this darkening pulses at all is unmeasured - it moves in the
    // opposite direction from Pulse's own brightening-toward-white, so
    // Pulse's period must never be filled in here. See
    // `oag_title::MenuSkin::selected_pulse_period_secs`.
    selected_pulse_period_secs: None,
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
    blocks: None,
    list: None,
    settings: None,
    // Unmeasured. Pure's own main-menu screen has not been captured for a
    // helptext-equivalent widget, and Pulse's `18.0`/`13.0/22.0`/white must
    // never be borrowed for it - see `oag_title::MenuSkin::help_text`.
    help_text: None,
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
    menu_scene: false,
    picker_backdrop_parent: Some(states::INTRO_SCREEN),
    picker_from_loaded_faces: false,
    fallback_globals: FALLBACK_GLOBALS,
    fallback_images: FALLBACK_IMAGES,
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
