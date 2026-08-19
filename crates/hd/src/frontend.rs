//! HD's front end, as far as its own XML states it.
//!
//! Read off `/data/plugins/frontend/gui/skin.xml`, which HD ships **six times**,
//! once in every archive but the sound-only `DATA01`, and which agrees on every
//! layout global across all six. The evidence, quoted, is in
//! [`hd-frontend.md`]; this module is the table that page produces.
//!
//! # Why [`crate::TITLE`] ships a front end, and what it is not claiming
//!
//! It ships one as of 2026-08-17, and **the thing that changed is the type, not
//! the evidence**. This section used to say `front_end: None`, and its reasoning
//! was sound: [`oag_title::BootProfile::chain`] was defined as a
//! **measurement** - its own module doc opens by recording that "a front-end
//! XML's declared entry point is not the runtime's" - and [`DECLARED_CHAIN`] is
//! the declared order and nothing more. Shipping it as `chain` would have put a
//! hypothesis in the one field whose contract was that it never held one.
//!
//! [`oag_title::Provenance`] is the way out, and it is a third answer rather
//! than a compromise on either of the first two. [`BOOT`] carries
//! [`oag_title::Provenance::Declared`], so the order is expressed *and* labelled
//! as read rather than watched, and `oag-game` says so on every boot report it
//! prints. Nothing below is claimed to be what a PS3 does. See
//! [ADR-0025](https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md).
//!
//! **What an emulator capture would still settle**, and what this build shows in
//! the meantime, is on [`hd-frontend.md`]. The short version: the order, whether
//! the picker runs at all on a machine that takes its language from the XMB,
//! and which of the six `skin.xml` copies is live.
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
//! So the layout is recovered and sits here as [`MENU_SKIN`], and what a capture
//! would now do is *upgrade* [`BOOT`]'s provenance rather than unlock the front
//! end.
//!
//! [`hd-frontend.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/hd-frontend.md

/// HD's menu layout, out of `skin.xml`'s `FEGlobals` block.
///
/// **These are 1920x1080 coordinates**, where Pulse's and Pure's are 480x272,
/// and [`oag_title::MenuSkin::space`] is where that is stated. This comment used
/// to warn that nothing carried a coordinate space and that a caller drawing
/// these at Pulse's scale would put the list 800 pixels off the side of a
/// PSP-sized frame - recorded rather than fixed, "because fixing it is a change
/// to the type and the type should change when something draws these".
///
/// Something draws them, the type changed, and the warning was accurate: the
/// RACE page came up as a column of values with no labels beside them, because
/// every label was off the right-hand edge. `oag_game::menu::Skin` converts both
/// sides now - the title's numbers out of [`MENU_SKIN::space`](oag_title::MenuSkin::space)
/// and this build's own out of the 480x272 they are written in - and the menus
/// are drawn in the *source's* grid rather than in either. See
/// `docs/formats/hd-frontend.md`.
///
/// Confidence **92** on the five authored numbers and the two colours: six
/// independent copies of an authored file agreeing to the digit. Short of 95
/// only because nothing has been seen to *read* them.
pub const MENU_SKIN: &oag_title::MenuSkin = &oag_title::MenuSkin {
    // **1920x1080, where both PSP titles are 480x272**, and the reason
    // `oag_title::MenuSkin` carries a grid at all. This paragraph used to be a
    // warning in the doc comment below - that a caller drawing these at Pulse's
    // scale would put the list 800 pixels off the side of a PSP-sized frame,
    // recorded rather than fixed "because fixing it is a change to the type and
    // the type should change when something draws these". Something draws them,
    // the type changed, and this field is the fix.
    //
    // Confidence 95: `skin.xml` declares a `<Movie>` at `Width="1920"
    // height="1080"` and places widgets out to `y="994"`, and all six copies
    // agree. See `oag_game::frontend::Space`.
    space: (1920.0, 1080.0),
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
    // `FEGlobals->TextColor`. White, against Pulse's cyan - and **one of the
    // globals that does not move between the archives**, which is worth saying
    // here because half the palette does. See `selected` below.
    text: Some(0xFFFF_FFFF),
    // `FEGlobals->TitleColor`. HD's menu screens usually reach for an `HD_*`
    // palette instead, whose `HD_Grey` is declared to the same value - so the
    // two agree today and could diverge. See `hd-frontend.md`.
    title: Some(0xFF64_6464),
    // **`None`, and the reason changed while this was being filled in.** It was
    // "a measured field and nothing measured", which was true and incomplete.
    //
    // The disc *does* name a highlight colour: `highlightColor`, 25 times across
    // its front end, and every one of the 25 is `FEGlobals->HD_Blue`, always
    // beside a `color` of `FEGlobals->HD_Grey`. That is this field and
    // [`MenuSkin::text`], on `<BracketButton>`s and the controls screen's button
    // widgets rather than on a menu.
    //
    // **What stops it becoming a number here is that the number is the FE
    // style.** The `HD_*` palette is declared per archive and the archives do
    // not agree: `DATA06` says `HD_Blue` `0xff8ac0ca` and `HD_BG` white,
    // `DATA00` says `0xffac0717` - red - and `HD_BG` black. Those are the two
    // styles the `OPT_FE_STYLE` option offers (`HD` and `FURY`), and which one a
    // boot draws is decided by which archive serves the front-end root. A
    // constant here would hard-code one of them into a table that cannot say
    // which. See `docs/formats/hd-frontend.md`.
    selected: None,
    // The dominant `<LeftLayer transition=>` across HD's own GUI files, which is
    // the weakest number here: confidence **70**, because HD spreads transitions
    // across eleven distinct values where Pulse uses four, so "dominant" carries
    // much less than it did there.
    transition_secs: 0.5,
    // **HD's main menu is horizontal, and this is the widget that says so.**
    // Read off `Data\Plugins\Frontend\Gui\MainMenu_Definition.xml`, whose one
    // `<HorizMenu name="Mode">` is the screen's only menu widget:
    //
    // ```xml
    // <HorizMenu name="Mode" focus="true" transition="0.4" delay="0.2">
    //   <Values align="left" x="160" y="125" color="0xff705070"></Values>
    // ```
    //
    // **Confidence 92.** Five of the seven archives carry that file - `DATA00`,
    // `DATA02`, `DATA03`, `DATA05` and `DATA06`, the sound-only `DATA01` and
    // `DATA04` carrying none - and all five write those three numbers
    // identically, which is the "exact agreement across many real files" case
    // the rubric puts at the top of the 85-94 band. It stops short of 95 for the
    // same reason `menu_x` does: nothing shows the engine *consuming* them, and
    // which copy the runtime serves is still open.
    //
    // Two things the widget does **not** state, both left to the drawing side
    // and marked as ours there rather than invented here:
    //
    // - **No `scale`.** Its two siblings in `additional_definition.xml` say
    //   `scale="1"`, and `FEGlobals->MenuScale` is 1.0, so nothing is lost by
    //   taking `menu_scale` - but the main menu's own widget is silent and this
    //   comment is the record of that.
    // - **No spacing.** There is no `gap` on this widget and no second anchor to
    //   derive one from, so how far apart two entries sit is unauthored. A
    //   capture would settle it; none exists.
    strip: Some(oag_title::MenuStrip {
        x: 160.0,
        y: 125.0,
        color: 0xFF70_5070,
    }),
};

/// The boot order HD's own `skin.xml` **declares**, which is not the same claim
/// as the order it boots in.
///
/// Confidence **80** that this is what the XML says, and **no claim at all**
/// about the runtime - which is what [`BOOT`] labels it, rather than what its
/// use is gated on. `hd-frontend.md` has the redirects this was read out of.
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

/// HD's boot sequence, as data, with [`DECLARED_CHAIN`] as its order.
///
/// **The provenance is the field to read first.** Everything else here is a
/// reading of the disc; `provenance` is what says the order among those
/// readings has never been watched. See [`oag_title::Provenance`] and this
/// module's own docs.
///
/// Three fields are `None`/empty and each is a measurement rather than a gap:
///
/// - **No reel.** Pulse's `--reel` is an off-path dev/pub state; HD's two logo
///   reels are both on the declared boot path, so there is nothing off it to
///   point the flag at.
/// - **No menu backdrop.** HD's menus sit on a real-time `.vex` scene
///   ([`names::FRONT_END_SCENE`]), not a looping movie, so there is no entry to
///   name. This is the same shape of `None` as Pure's, for a different reason:
///   Pure ships no backdrop movie, HD ships something that is not a movie.
/// - **No fallback globals.** All 64 `FEGlobals` the screens name are declared
///   in `skin.xml` itself - `hd-frontend.md` checked, and found none referenced
///   but undeclared in any of the six copies.
pub const BOOT: &oag_title::BootProfile = &oag_title::BootProfile {
    // Read, never watched. No PS3 emulator capture of this title exists in this
    // project, and the executable settles only the entry point (confidence 70).
    provenance: oag_title::Provenance::Declared,
    chain: DECLARED_CHAIN,
    reel: None,
    menu_backdrop: None,
    picker_backdrop_parent: None,
    fallback_globals: &[],
};

/// HD's front end: the layout it authors and the boot order it declares.
///
/// The two halves are recovered to very different standards and
/// [`oag_title::FrontEnd`] cannot say so - which is why [`BOOT`] carries the
/// caveat rather than this. [`MENU_SKIN`] is confidence 92 off six agreeing
/// copies of an authored file; the chain is a declaration.
pub const FRONT_END: &oag_title::FrontEnd = &oag_title::FrontEnd {
    root: names::FRONTEND_ROOT,
    language_plugins: LANGUAGE_PLUGINS,
    menu: MENU_SKIN,
    boot: BOOT,
    menu_frame: Some(states::FE_SCREEN),
};

/// The sixteen plugins that carry a language, named rather than numbered.
///
/// **Sixteen against the PSP titles' five**, and the difference is the release
/// rather than the format: each is a `Data\Plugins\Languages\<name>` directory
/// holding the same `Definition.xml` and `entries.xml` pair a numbered PSP
/// plugin holds, so `oag-game` joins the token into a path with no change.
///
/// Listed in the order the archive lists them, which is alphabetical and
/// therefore **not** a menu order: what the picker shows is each plugin's own
/// declared name, read back out of its definition. `American` and `English`
/// being separate entries is the disc's, not a duplicate - the two differ in
/// their string tables.
///
/// Read off `DATA02`, which is one of the two archives carrying the full set;
/// every one of the sixteen resolves. Confidence **94** on the list, being a
/// directory listing rather than an inference, and no claim at all about which
/// of them a PS3 offers on a given machine - the console's XMB language is what
/// the executable would consult, and that has not been read.
pub const LANGUAGE_PLUGINS: &[&str] = &[
    r"Languages\American",
    r"Languages\Danish",
    r"Languages\Dutch",
    r"Languages\English",
    r"Languages\Finnish",
    r"Languages\French",
    r"Languages\German",
    r"Languages\Italian",
    r"Languages\Japanese",
    r"Languages\Korean",
    r"Languages\Norwegian",
    r"Languages\Portuguese",
    r"Languages\Russian",
    r"Languages\Spanish",
    r"Languages\Swedish",
    r"Languages\TraditionalChinese",
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
    /// The frame every menu screen is drawn inside, and
    /// [`TOP_FE_SCREEN`]'s only child.
    ///
    /// **What is authored on it** - all of it in `skin.xml`, all six copies
    /// agreeing, and every screen the menus use is nested somewhere under it:
    ///
    /// - `<ScreenClear Colour="FEGlobals->HD_BG">`, which is `0xffffffff`.
    ///   Wipeout HD's front end is drawn on **white**, not on the black this
    ///   build clears to for want of anything else.
    /// - two `line.gtf` rules, `1600x8` at `(160, 110)` and `(160, 975)`,
    ///   tinted `FEGlobals->HD_Grey`, which are the horizontal bars above and
    ///   below every page.
    /// - `Title_Arrow_HD.gtf`, `32x32` at `(160, 73)`, tinted the same.
    ///
    /// The disc's own comment on the sibling `<Image name="networkbusy">` -
    /// *"have this before movie and it doesnt appear! superb"* - is a reminder
    /// that document order on this screen means something to the engine that
    /// nothing here has read.
    ///
    /// **Its `<Text>` widgets are not part of the frame**, and that is a
    /// distinction with a reason rather than a convenience: they are the trial
    /// build's (`FE_TRIAL_MODE`, `FE_PURCHASE_NOW`) or belong to a
    /// `<NavigationController>`, a widget that decides per screen which button
    /// prompts to show and which this build does not have. The three images and
    /// the clear carry no such owner.
    pub const FE_SCREEN: &str = "FE Screen";
}

/// The three assets HD's front-end XML names for itself.
pub mod names {
    /// The front-end root: every boot screen, the `FEGlobals` block, and the
    /// `LoadXML` list that pulls in the rest.
    ///
    /// **A named plugin where both PSP titles use a numbered one**
    /// (`Data\Plugins\PI001\GUI\Skin.xml`), which is the difference that made
    /// this a per-title axis at all - see [`oag_title::FrontEnd::root`]. It is
    /// the same shape of divergence HD's soundtrack showed: its `PI_Music`
    /// declarations are under `Data\Plugins\Frontend` too.
    ///
    /// **This path resolves in six of the seven archives and they differ by
    /// MD5**, so which copy is served is decided by
    /// `oag_assets::ArchiveCandidates`' order and is a documented choice rather
    /// than a measurement of what a PS3 loads. All six agree on every layout
    /// global; they do not agree on the screen list. See `hd-frontend.md`.
    pub const FRONTEND_ROOT: &str = r"Data\Plugins\Frontend\Gui\Skin.xml";

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
