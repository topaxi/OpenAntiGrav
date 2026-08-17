//! HD's front end, as far as its own XML states it.
//!
//! Read off `/data/plugins/frontend/gui/skin.xml`, which HD ships **six times**,
//! once in every archive but the sound-only `DATA01`, and which agrees on every
//! layout global across all six. The evidence, quoted, is in
//! [`hd-frontend.md`]; this module is the table that page produces.
//!
//! # Why [`crate::TITLE`] still says `front_end: None`
//!
//! Because half of what an [`oag_title::FrontEnd`] promises is not here, and the
//! missing half is the boot chain rather than the layout.
//!
//! [`oag_title::BootProfile::chain`] is defined as a **measurement**: its own
//! module doc opens by recording that "a front-end XML's declared entry point is
//! not the runtime's", and it says so because both PSP titles were cold-booted
//! and Pulse's runtime disagreed with Pulse's XML. [`DECLARED_CHAIN`] below is
//! the declared order and nothing more, so shipping it as `chain` would put a
//! hypothesis in the one field of `oag-title` whose contract is that it is not
//! one.
//!
//! **The entry point specifically is in better shape than the rest of the
//! chain**, and it is the one part the PS3 executable has been read for.
//! `FUN_000186f0` is four instructions returning `"Launch Game"` when a flag on
//! the game-root object is set and `"Language Selection"` otherwise - both real
//! HD screen names. Confidence **88** that it returns one of those two on that
//! flag, **70** that it is the boot entry point, the call site being virtual and
//! unresolved. So HD's runtime default looks like its XML's, and the only
//! alternative is a straight-into-a-race trial path rather than a different
//! front-end order. That is Pure's situation and not Pulse's - but 70 on the one
//! step nearest the start is not a measured chain of eight.
//!
//! There is a second, independent reason and it points the same way: six copies
//! of `skin.xml` and no manifest means **which of them the runtime loads is
//! unknown**, and the two families declare different logo movies
//! ([`names::STUDIO_LOGO_MOVIE`] against [`names::STUDIO_LOGO_MOVIE_FURY`]).
//! The executable does not settle it either: it composes the skin path from the
//! plugin's own location, identical for all six, and the strings `LogoFMV`,
//! `Show Logo`, `StudioLiverpool` and `Backdrop` are **not in the binary at
//! all**. The reel is chosen by the XML, so the question folds back into which
//! `skin.xml` is live.
//!
//! So the layout is recovered and sits here as [`MENU_SKIN`], ready for the
//! change that also settles the chain. That change is an emulator capture, not a
//! decision.
//!
//! [`hd-frontend.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/hd-frontend.md

/// HD's menu layout, out of `skin.xml`'s `FEGlobals` block.
///
/// **These are 1920x1080 coordinates**, where Pulse's and Pure's are 480x272.
/// Nothing in [`oag_title::MenuSkin`] carries a coordinate space, because until
/// this table existed there was only one; a caller that draws HD's menus at
/// Pulse's scale will put the list 800 pixels off the side of a PSP-sized
/// frame. Recorded here rather than fixed, because fixing it is a change to the
/// type and the type should change when something draws these.
///
/// Confidence **92** on the five authored numbers and the two colours: six
/// independent copies of an authored file agreeing to the digit. Short of 95
/// only because nothing has been seen to *read* them.
pub const MENU_SKIN: &oag_title::MenuSkin = &oag_title::MenuSkin {
    // `FEGlobals->MenuXOffset`.
    menu_x: 800.0,
    menu_scale: 1.0,
    title_x: 194.0,
    title_y: 62.0,
    title_scale: 1.0,
    // **Not authored, and the reason is the finding.** HD's main menu is a
    // `<HorizMenu>` - horizontal - so it has no first row to put a y on, and the
    // 17 vertical `<Menu>` widgets elsewhere in the tree do not converge on one
    // value the way Pure's 33 did. Pure's route to this field does not exist
    // here; nothing is being left for later.
    first_row_y: None,
    // A *measured* field, and nothing has been measured: no capture of HD
    // running exists in this project.
    row_extra_leading: None,
    // **Not authored, and also a measurement.** HD's language plugins declare
    // five font slots to Pulse's eight, and none of them is `Menu`. Four
    // `<Menu>` widgets in `online_definition.xml` say `font="menu"` anyway,
    // naming a slot no plugin declares - what the engine does with that needs
    // the executable.
    menu_font: None,
    // `FEGlobals->TextColor`. White, against Pulse's cyan.
    text: Some(0xFFFF_FFFF),
    // `FEGlobals->TitleColor`. HD's menu screens usually reach for an `HD_*`
    // palette instead, whose `HD_Grey` is declared to the same value - so the
    // two agree today and could diverge. See `hd-frontend.md`.
    title: Some(0xFF64_6464),
    // A *measured* field. Nothing measured.
    selected: None,
    // The dominant `<LeftLayer transition=>` across HD's own GUI files, which is
    // the weakest number here: confidence **70**, because HD spreads transitions
    // across eleven distinct values where Pulse uses four, so "dominant" carries
    // much less than it did there.
    transition_secs: 0.5,
};

/// The boot order HD's own `skin.xml` **declares**, which is not the same claim
/// as the order it boots in.
///
/// Confidence **80** that this is what the XML says, and **no claim at all**
/// about the runtime. See this module's own docs for why that keeps
/// [`crate::TITLE`]'s `front_end` at `None`, and `hd-frontend.md` for the
/// redirects this was read out of.
///
/// **Three of the nine steps exist only because the game is online**:
/// [`states::PRE_FMV_CONNECT`] runs a connection check before the logo, and
/// [`states::EULA`] and [`states::UPDATE_ANNOUNCEMENT`] both carry `<SVOData>`.
/// Pulse's boot has no equivalent to any of them.
pub const DECLARED_CHAIN: &[oag_title::BootStep] = &[
    oag_title::BootStep::screen(states::LANGUAGE_SELECTION),
    oag_title::BootStep::screen(states::PRE_FMV_CONNECT),
    oag_title::BootStep::playing(states::STUDIO_LOGO, names::STUDIO_LOGO_MOVIE),
    oag_title::BootStep::screen(states::EPILEPSY_WARNING),
    oag_title::BootStep::screen(states::FIRST_PLAY),
    oag_title::BootStep::screen(states::SAVE_WARNING),
    oag_title::BootStep::screen(states::EULA),
    oag_title::BootStep::screen(states::UPDATE_ANNOUNCEMENT),
];

/// Screen names, spelled exactly as HD's XML spells them.
///
/// The spellings are inconsistent - `Language Selection` and `Save Warning`
/// carry a space, `EpilepsyWarning` and `FirstPlay` do not - and that is the
/// disc's, not a transcription slip. A normaliser here would stop these matching
/// the file.
pub mod states {
    /// The declared entry point. Whether it is ever *shown* on a machine that
    /// takes its language from the XMB is unread.
    pub const LANGUAGE_SELECTION: &str = "Language Selection";
    /// A `<UnityConBasic>` connection check, before the logo.
    pub const PRE_FMV_CONNECT: &str = "PreFMVConnect";
    /// The one declared boot step that plays a movie.
    pub const STUDIO_LOGO: &str = "Studio Logo";
    /// Health warning.
    pub const EPILEPSY_WARNING: &str = "EpilepsyWarning";
    /// First-run branch.
    pub const FIRST_PLAY: &str = "FirstPlay";
    /// Save-data warning.
    pub const SAVE_WARNING: &str = "Save Warning";
    /// Carries `<SVOData>`; online.
    pub const EULA: &str = "EULA";
    /// Carries `<SVOData>`; online.
    pub const UPDATE_ANNOUNCEMENT: &str = "Update Announcement";
    /// Where the declared chain arrives.
    pub const MAIN_MENU: &str = "Main Menu";
    /// The screen carrying the real-time menu backdrop; see
    /// [`names::FRONT_END_SCENE`].
    pub const TOP_FE_SCREEN: &str = "Top FE Screen";
}

/// The three assets HD's front-end XML names for itself.
pub mod names {
    /// The logo reel, as `DATA06`'s `skin.xml` spells it.
    ///
    /// **The forward slashes are the disc's**, not a transcription slip: the
    /// attribute reads `src="Data/FE/Images/StudioLiverpool.bik"` where every
    /// other asset reference in the same file uses backslashes. It is left
    /// alone here for the same reason a normaliser would be wrong - that is what
    /// the data says.
    ///
    /// **Decoded on 2026-08-17, and the name and the content agree**: 531
    /// frames of 1920x1080 at 59.94 Hz, 8.86 s, resolving out of black to a
    /// card reading `STUDIO Liverpool`. This doc comment used to record that
    /// the file was Bink, that nothing here decoded it, and that the name was
    /// therefore the whole of the evidence; [`oag_formats::bik`] and
    /// `crates/game/tests/hd_movie_ground_truth.rs` are what replaced that. See
    /// [`bik.md`].
    ///
    /// What is **not** settled by decoding it is which reel plays - that is
    /// still the question of which `skin.xml` the runtime loads, above.
    ///
    /// [`bik.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/bik.md
    pub const STUDIO_LOGO_MOVIE: &str = "Data/FE/Images/StudioLiverpool.bik";

    /// The other family's spelling, and the only one of the two that is in
    /// `DATA00`.
    ///
    /// Which of the six `skin.xml` copies the runtime loads decides which of
    /// these plays. That question needs the executable.
    pub const STUDIO_LOGO_MOVIE_FURY: &str = "Data/FE/Images/StudioLiverpool_fury.bik";

    /// The menu backdrop, which on HD is **a real-time `.vex` scene** rather
    /// than a looping movie.
    ///
    /// Pulse loops `Data\Movies\Backdrop.PMF` behind its menus; HD ships nothing
    /// equivalent, and its `Top FE Screen` carries a `<BackgroundAnim>` naming
    /// this file with a camera pose and per-screen blur instead. Both the `.vex`
    /// and its `.rcsmodel` are in `DATA02`, so the geometry is behind the same
    /// undecoded container the circuits' is.
    pub const FRONT_END_SCENE: &str = r"Data\FE\FrontEndScene\FrontEndScene_HD_ATG.vex";
}
