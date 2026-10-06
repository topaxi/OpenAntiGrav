//! Omega's front end, read off its own `data09.psarc` copy of
//! `skin.xml`/`mainmenu_definition.xml`/`additional_definition.xml` - not
//! copied from [`oag_hd::frontend`], even though every number below turns out
//! to match HD's to the digit. See `docs/formats/omega-frontend.md` for the
//! full census this module's numbers were re-derived against, and
//! `crates/omega/tests` for the ground-truth check that reads them back off
//! the archive.
//!
//! # What is *not* here, and why
//!
//! - **[`BOOT`]'s provenance is [`oag_title::Provenance::Declared`], and
//!   stays that way for this lane.** No PS4 emulator exists in this
//!   project's toolchain (`oag_trace`'s own list is PCSX2/PPSSPP/RPCS3), so
//!   nothing has watched an Omega boot the way HD's three RPCS3 runs did
//!   before [`oag_hd::frontend::BOOT`] upgraded. See [ADR-0025].
//! - **[`MENU_BLOCKS`]-equivalent numbers do not exist here at all.** HD's
//!   own `MenuBlocks` table is read out of `EBOOT.elf` at named addresses -
//!   nothing authored in any XML - and nobody has disassembled Omega's PS4
//!   executable. Pointing `MENU_SKIN.blocks` at `oag_hd::frontend::MENU_BLOCKS`
//!   would assert an unmeasured equivalence between two different binaries,
//!   so it stays `None`: the menu loses its block background/cursor/arrow
//!   art rather than drawing HD's numbers under Omega's name.
//! - **No race box.** `racebox_definition.xml` exists in `data09.psarc`, but
//!   its dialect has not been checked against either the Pulse/Pure
//!   `Selection_Definition.xml` shape `oag_ui_screens::picker::Layout::read` parses
//!   or HD's own unread `<Model>`-widget one. `None`, a gap rather than a
//!   measurement - see `docs/formats/omega-status.md`.
//!
//! [ADR-0025]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md

/// Omega's menu layout, out of `data09.psarc`'s `skin.xml`/
/// `mainmenu_definition.xml`/`additional_definition.xml`.
///
/// Every field below was read directly against the archive (see
/// `crates/omega/tests`), not copied from [`oag_hd::frontend::MENU_SKIN`] -
/// the fact that the numbers agree is the finding
/// `docs/formats/omega-frontend.md`'s "`FEGlobals` is bit-identical to HD's"
/// section already made, confirmed independently here rather than assumed.
pub const MENU_SKIN: &oag_title::MenuSkin = &oag_title::MenuSkin {
    // `skin.xml` authors no `<Movie>` tag with an explicit width/height the
    // way HD's own six copies do (checked: none found in `data09`'s copy) -
    // but `Studio Logo`'s own `<Movie>` widget states `Width="1920"
    // height="1080"`, and every widget coordinate in the file (`x="1340"`,
    // `y="994"`-scale values in `mainmenu_definition.xml`) reads consistently
    // against that grid. Confidence carried at HD's own level for the same
    // reason: one authored `1920x1080` widget plus a whole file of
    // coordinates consistent with it, not a second independent copy to agree
    // with the way HD had six.
    space: (1920.0, 1080.0),
    // `FEGlobals->MenuXOffset`/`MenuYOffset`/`MenuScale`, read directly:
    // `800`/`300`/`1.0`. Matches HD's to the digit.
    menu_x: 800.0,
    menu_scale: 1.0,
    // `FEGlobals->TitleXOffset`/`TitleYOffset`/`TitleScale`: `194`/`62`/`1.0`.
    title_x: 194.0,
    title_y: 62.0,
    title_scale: 1.0,
    // `mainmenu_definition.xml`'s own title widget: `<Text idstring="FE_MM"
    // font="Title" x="FEGlobals->TitleXOffset" y="FEGlobals->TitleYOffset"
    // color="FEGlobals->HD_Grey">`.
    title_font: Some("Title"),
    // Not authored, on HD's own terms: Omega's main menu is the same
    // `<HorizMenu>` idiom HD's is (see `strip` below), so there is no
    // vertical `<Menu>` first row to converge on either.
    first_row_y: None,
    // A measured field and nothing measured - no capture of Omega running
    // exists in this project.
    row_extra_leading: None,
    menu_font: None,
    // `FEGlobals->TextColor`: `0xFFFFFFFF`, matching HD's.
    text: Some(0xFFFF_FFFF),
    // `FEGlobals->TitleColor`: `0xFF646464`, matching HD's.
    title: Some(0xFF64_6464),
    background: None,
    // Not measured this lane - Omega's own `HD_Colours`/`WOHD_Colours`/
    // `Fury_Colours`/`2048_Colours` switch (`docs/formats/omega-frontend.md`,
    // "The colour-skin switch is restructured, not just copied") is real but
    // which runtime condition selects between them is unread here either.
    selected: None,
    selected_pulse_period_secs: None,
    // Not independently re-measured this lane - carried at HD's own
    // confidence-70 figure since Omega's own front end spreads transitions
    // the same wide way HD's does (`0.4`, `0.5` and others all appear in the
    // files this module reads), and no capture exists to pick a dominant one.
    transition_secs: 0.5,
    // `mainmenu_definition.xml`'s own `<HorizMenu name="Mode" focus="true"
    // transition="0.4" delay="0.2"><Values align="left" x="160" y="125"
    // color="0xff705070">` - read directly, matches HD's to the digit.
    strip: Some(oag_title::MenuStrip {
        x: 160.0,
        y: 125.0,
        color: 0xFF70_5070,
        // Not independently confirmed live on Omega (HD's own confirmation
        // was a captured settings-screen frame; no Omega capture exists) -
        // carried as the same global name since `HD_Blue` is declared in
        // Omega's own `skin.xml` unchanged.
        selected_fill: Some("HD_Blue"),
    }),
    blocks: None,
    // `additional_definition.xml`'s own `<Item OffsetX="160" OffsetY="170">`
    // (three `type="Settings"` screens) - read directly, matches HD's x/y to
    // the digit. `pitch`/`text_scale` not independently re-measured this
    // lane (the row-stepping and label-scale values HD's own page reads off
    // the same widget kind); carried at HD's figures since the two widgets
    // otherwise agree exactly.
    list: Some(oag_title::MenuList {
        x: 160.0,
        y: 170.0,
        pitch: 50.0,
        text_scale: 0.8,
    }),
    help_text: None,
};

/// The boot order Omega's `skin.xml` declares, read redirect by redirect out
/// of `data09.psarc` - **not watched**, see this module's own doc comment.
///
/// Same eight screens, same order, as [`oag_hd::frontend::BOOT_CHAIN`] - and
/// **no dead `LogoFMV`**, confirmed absent from `data09`'s `skin.xml` the
/// same way it is absent from HD's own `DATA00`/`DATA05`/`DATA06` family.
pub const BOOT_CHAIN: &[oag_title::BootStep] = &[
    oag_title::BootStep::screen(states::LANGUAGE_SELECTION),
    oag_title::BootStep::screen(states::PRE_FMV_CONNECT),
    oag_title::BootStep::playing(states::STUDIO_LOGO, names::STUDIO_LOGO_MOVIE),
    oag_title::BootStep::screen(states::EPILEPSY_WARNING),
    oag_title::BootStep::screen(states::FIRST_PLAY),
    oag_title::BootStep::screen(states::SAVE_WARNING),
    oag_title::BootStep::screen(states::EULA),
    oag_title::BootStep::screen(states::UPDATE_ANNOUNCEMENT),
];

/// Omega's boot sequence, as data, with [`BOOT_CHAIN`] as its order.
///
/// **`provenance` is [`oag_title::Provenance::Declared`]** - read out of the
/// XML, never watched. See this module's own doc comment and [ADR-0025].
///
/// [ADR-0025]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md
pub const BOOT: &oag_title::BootProfile = &oag_title::BootProfile {
    provenance: oag_title::Provenance::Declared,
    chain: BOOT_CHAIN,
    reel: None,
    // Omega's menus sit on the same real-time backdrop idiom HD's do
    // (`Top FE Screen`/`FE Screen`), not a looping movie - no entry to name,
    // on the same terms `oag_hd::frontend::BOOT`'s own `None` states.
    menu_backdrop: None,
    // The `BackgroundAnim` scene is the only backdrop Omega can have: it ships
    // no `.points2` clouds for the Fury widget. See
    // `docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop-scene.md`.
    menu_scene: true,
    picker_backdrop_parent: None,
    picker_from_loaded_faces: true,
    fallback_globals: &[],
    fallback_images: &[],
};

/// Omega's front end: the layout it authors and the boot order it takes.
pub const FRONT_END: &oag_title::FrontEnd = &oag_title::FrontEnd {
    root: names::FRONTEND_ROOT,
    language_plugins: LANGUAGE_PLUGINS,
    language_manifests: &[],
    menu: Some(MENU_SKIN),
    menu_ps2: None,
    // Omega authors HD's `FEGlobals`/`<HorizMenu>` vocabulary, not 2048's
    // touch-icon idiom.
    touch: None,
    boot: BOOT,
    menu_frame: Some(states::FE_SCREEN),
    // See this module's own "What is not here" section.
    race_box: None,
    // `Team_Selection_Definition.xml` is in `data09.psarc` at HD's own path,
    // and naming it makes the screen read - but it then draws an empty frame:
    // the team logos sit at `Data\art\published\hdships\<Team>\FE\Logo.gnf`
    // (HD's reader asks `Data\Ships\<Team>\FE\Logo.gtf`), the stat blocks do
    // not draw, and `screen.xml` has no slideshow chain. Left `None` so a
    // confirmed campaign cell races the RACE page's team (`settings.race.team`)
    // rather than opening a screen a player cannot read; measured 2026-09-30, the open item in
    // `docs/formats/omega-status.md`.
    team_select: None,
    track_select: None,
    race_setup: None,
    preview_meshes: false,
    // `EndRace_Definition.xml` is in `data09.psarc` at HD's own path, and
    // dispatching Omega through HD's reader (`oag_game::endrace::load_hd`)
    // draws `EndRace Results`, but `EndRace Menu` then draws no option blocks
    // (HD's draws three), a dead end for the player. Left `None` so a finished
    // race keeps the built-in results table and returns to the menus;
    // measured 2026-09-30, the open item in `docs/formats/omega-status.md`.
    endrace_entry: None,
};

/// Screen names, spelled exactly as `data09.psarc`'s `skin.xml` spells them -
/// confirmed identical to HD's own spellings for every one of these eight,
/// read directly rather than assumed.
pub mod states {
    pub const LANGUAGE_SELECTION: &str = "Language Selection";
    pub const PRE_FMV_CONNECT: &str = "PreFMVConnect";
    pub const STUDIO_LOGO: &str = "Studio Logo";
    pub const EPILEPSY_WARNING: &str = "EpilepsyWarning";
    pub const FIRST_PLAY: &str = "FirstPlay";
    pub const SAVE_WARNING: &str = "Save Warning";
    pub const EULA: &str = "EULA";
    pub const UPDATE_ANNOUNCEMENT: &str = "Update Announcement";
    pub const MAIN_MENU: &str = "Main Menu";
    /// `<Screen type="FEMain" name="Top FE Screen">`, [`FE_SCREEN`]'s parent -
    /// read directly, same nesting HD's own copy uses.
    pub const TOP_FE_SCREEN: &str = "Top FE Screen";
    /// `<Screen name="FE Screen">`, nested inside [`TOP_FE_SCREEN`] - the
    /// frame every menu screen draws inside. Not re-read field by field the
    /// way `oag_hd::frontend::states::FE_SCREEN`'s own doc comment quotes its
    /// `<ScreenClear>`/line widgets; confirmed present and nested correctly,
    /// not confirmed to carry the same three images.
    pub const FE_SCREEN: &str = "FE Screen";
}

/// The assets Omega's front-end XML names for itself, read directly rather
/// than assumed off HD's.
pub mod names {
    /// The front-end root. Same relative path as HD's, spelled with the same
    /// backslash/PascalCase convention this project's other titles use for
    /// readability - the archive's own directory entries are lowercase with
    /// forward slashes (`data/plugins/frontend/gui/skin.xml`), and
    /// `oag_assets`'s path folding resolves either spelling identically. Read
    /// off `data09.psarc`, confirmed present.
    pub const FRONTEND_ROOT: &str = r"Data\Plugins\Frontend\Gui\Skin.xml";

    /// `Studio Logo`'s own `<Movie>` widget:
    /// `src="Data/FE/Images/StudioLiverpool.bik"` - **the base name, not
    /// `_fury`**, matching HD's own finding that this is the one string that
    /// used to settle which PS3 archive was live.
    /// `StudioLiverpool_fury.bik` was not found in any of the nine Omega
    /// archives' path listings this lane checked - unlike HD, which still
    /// ships both cuts. Whether Omega unified the two or dropped Fury's is
    /// unread; `docs/formats/omega-frontend.md`'s own "Open" section already
    /// carries this as unresolved.
    pub const STUDIO_LOGO_MOVIE: &str = "Data/FE/Images/StudioLiverpool.bik";
}

/// The language plugins `data09.psarc` carries, in the archive listing's own
/// order (alphabetical, and therefore **not** a menu order, on the same
/// terms HD's own list states).
///
/// **Twenty-three against HD's sixteen**, and lowercase where HD's are
/// capitalised (`Languages\american`, not `Languages\American`) - confirmed
/// by direct directory listing, not assumed. `asia` is present as its own
/// directory and its role (a shared catch-all, or a distinct locale) was not
/// read.
pub const LANGUAGE_PLUGINS: &[&str] = &[
    r"Languages\american",
    r"Languages\arabic",
    r"Languages\asia",
    r"Languages\danish",
    r"Languages\dutch",
    r"Languages\english",
    r"Languages\finnish",
    r"Languages\french",
    r"Languages\german",
    r"Languages\italian",
    r"Languages\japanese",
    r"Languages\korean",
    r"Languages\norwegian",
    r"Languages\polish",
    r"Languages\portuguese",
    r"Languages\portuguesebr",
    r"Languages\russian",
    r"Languages\simplifiedchinese",
    r"Languages\spanish",
    r"Languages\spanishla",
    r"Languages\swedish",
    r"Languages\traditionalchinese",
    r"Languages\turkish",
];
