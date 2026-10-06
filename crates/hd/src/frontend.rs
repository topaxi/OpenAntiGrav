//! HD's front end, as far as its own XML states it.
//!
//! Read off `/data/plugins/frontend/gui/skin.xml`, which HD ships **six times**,
//! once in every archive but the sound-only `DATA01`, and which agrees on every
//! layout global across all six. The evidence, quoted, is in
//! [`hd-frontend.md`]; this module is the table that page produces.
//!
//! # Why [`crate::TITLE`] ships a front end, and what it is now claiming
//!
//! It ships one as of 2026-08-17, and for the first two and a half weeks of that
//! the chain was labelled [`oag_title::Provenance::Declared`]: the order was
//! read redirect by redirect out of the XML and nobody had watched a PS3 take
//! it. [ADR-0025] is the type that let it be expressed and labelled at once,
//! and it said what would end the state - "an emulator capture now *upgrades* a
//! title rather than unlocking it."
//!
//! **That capture happened on 2026-09-05 and [`BOOT`] is
//! [`oag_title::Provenance::Measured`].** Three cold boots on RPCS3, savedata
//! moved aside so the boot is genuinely a first play, walked all eight steps of
//! [`BOOT_CHAIN`] in this order and no other, with nothing extra between them -
//! `just rpcs3-bootchain`, kept under `data/reference/hd-boot-chain/`. See
//! [`hd-frontend.md`].
//!
//! **One thing that capture did not settle, and it is not this field's - it
//! has since been answered on the docs page, not in the type.**
//! [`states::LANGUAGE_SELECTION`] is *entered* on every boot; whether a frame
//! of it is ever *shown* was open until four more RPCS3 boots crossed system
//! language against savedata and found `LanguageAutoRedirect` firing within
//! 2-296 ms regardless of either - not "a console whose XMB already answers
//! the question", which was the working hypothesis and is now falsified: a
//! system language with no shipped plugin left the screen exactly as fast.
//! See [`hd-frontend.md`]'s "what the capture did not settle, and what a
//! later one did". RPCS3 being an emulator, not a PS3, is still why that
//! score tops out at 85 rather than higher, same as the chain order below.
//! `Provenance` is deliberately two-valued and grades neither; ADR-0025
//! rejected a confidence number in the type and put both scores on the docs
//! page instead.
//!
//! [ADR-0025]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md
//!
//! **The entry point is the one part the PS3 executable has been read for, and
//! the capture corroborated it.** `FUN_000186f0` is four instructions returning
//! `"Launch Game"` when a flag on the game-root object is set and
//! `"Language Selection"` otherwise - both real HD screen names. Confidence
//! **88** that it returns one of those two on that flag, **70** that it is the
//! boot entry point, the call site being virtual and unresolved. That 70 used to
//! be the whole of what was known about the runtime, and this paragraph used to
//! end by saying so ("70 on the one step nearest the start is not a measured
//! chain of eight"). It is now a *second, static* line of evidence agreeing with
//! a boot that was watched: every cold boot does open on `Language Selection`.
//! Neither number moves - a static read is not confirmed by a dynamic one, they
//! simply concur.
//!
//! **Which of the six `skin.xml` copies is live is `DATA00`'s**, settled by the
//! same 2026-09-05 boots and by two independent lines. The game's own `printf`
//! loads `data01` through `data06` and then `data00` **last**; and during
//! `Studio Logo` the runtime stats a path ending
//! [`names::STUDIO_LOGO_MOVIE_FURY`], a filename that appears in exactly one
//! file on the whole disc - `DATA00`'s copy. Every other copy names
//! [`names::STUDIO_LOGO_MOVIE`]. Confidence 92; the evidence is quoted on
//! [`hd-frontend.md`].
//!
//! That also retires the framing the other copies used to be discussed in. They
//! are not alternative boot paths that a run might take instead; they are
//! superseded copies of one file, and the chains they declare are **unreachable**
//! rather than merely unexercised.
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
    // agree. See `oag_display::space::Space`.
    space: (1920.0, 1080.0),
    // `FEGlobals->MenuXOffset`.
    menu_x: 800.0,
    menu_scale: 1.0,
    title_x: 194.0,
    title_y: 62.0,
    title_scale: 1.0,
    // `<Text idstring="FE_MM" font="Title" ...>` on `MainMenu_Definition.xml`'s
    // own title widget - see `oag_title::MenuSkin::title_font`'s own doc for
    // the full read and `docs/formats/hd-frontend.md`'s `TitleColor` section
    // for the widget in context. Confidence 92, the same figure `strip`
    // below carries: five of seven archives agree and nothing here has been
    // seen to be *consumed* by the executable.
    title_font: Some("Title"),
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
    // `FE Screen` authors its own `<ScreenClear Colour="FEGlobals->HD_BG">`,
    // so `Frame::clear` already carries this and the field stays unneeded.
    background: None,
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
    // A 2026-09-05 census of HD's entire front-end XML (772 entries, all seven
    // archives) found no oscillation attribute anywhere - no `<Key>` can key a
    // colour, and `pulse="true"` never appears on a menu `<Entry>` - so this is
    // a measurement, not a gap. See `docs/formats/hd-frontend.md`.
    selected_pulse_period_secs: None,
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
        // Confirmed live, 2026-09-01: `Additional -> Settings`'s own "Menu
        // Style" row switches this exact global on screen, in the same frame
        // the value changes. See `docs/formats/hd-frontend.md`.
        selected_fill: Some("HD_Blue"),
    }),
    blocks: Some(MENU_BLOCKS),
    // `<Item OffsetX="160" OffsetY="170">` around every row of every
    // `type="Settings"` screen in `additional_definition.xml`, the rows' own
    // `y` stepping `0, 50, 100, ...` - four archives, no disagreement. This is
    // where HD's settings rows are, and `MenuXOffset` (800) is not: that
    // global places the language picker's `<Menu>` and nothing else this build
    // draws. Confidence 90.
    list: Some(oag_title::MenuList {
        x: 160.0,
        y: 170.0,
        pitch: 50.0,
        text_scale: 0.8,
    }),
    // Unmeasured, and HD's own front end authors no per-row help text
    // anywhere in the census `docs/formats/hd-frontend.md` already ran for
    // its selected-row pulse - see `oag_title::MenuSkin::help_text`.
    help_text: None,
};

/// The box behind every entry of HD's strip and settings rows, as
/// `EBOOT.elf` draws it.
///
/// **Every number is the executable's** - read out of `Block_Item.cpp`,
/// `HorizMenu_Item.cpp` and `List_Item.cpp`, with each constant's address on
/// [`menu-blocks.md`]. Nothing here was measured off a capture and nothing is
/// authored in the GUI XML; the strip's `ItemWidth` default is what the main
/// menu draws with *because* it authors none. The three captures that exist
/// agree with these to the unit, which is recorded on the same page.
///
/// The three textures are named, not transcribed: the caller decodes them off
/// the served archives, and the fill's alpha - a swatch texel of the first -
/// is sampled from the decoded picture rather than written here.
///
/// [`menu-blocks.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/ghidra/functions/ps3-hdfury-eu/menu-blocks.md
pub const MENU_BLOCKS: oag_title::MenuBlocks = oag_title::MenuBlocks {
    frame_texture: names::MENU_BLOCK_FRAME,
    cursor_texture: names::MENU_STRIP_CURSOR,
    arrow_texture: names::MENU_LIST_ARROW,
    strip: oag_title::StripBlocks {
        // `HorizMenu_Construct`, `0x001b47c8`: `+0xe0 = 298.0`, the field
        // `HorizMenu_ParseXml` fills from `ItemWidth`.
        item_width: 298.0,
        // `HorizMenu_LayoutBlocks`, `0x001b4158`: `0x008ad5a0`.
        focus_extra: 70.0,
        // The same function: `0x008ad59c`, the block pitch's `+10` and the
        // label's inset.
        gap: 10.0,
        // `HorizMenu_AddEntryBlock`, `0x001b2b98`: `0x008ad4f0`.
        height: 64.0,
        // `HorizMenu_LayoutBlocks`: `0x008ad59c` and `0x008ad5a4`.
        underline_offset: (10.0, 35.0),
        // The same function: the `Image` child's `+0xac`/`+0xb0`.
        underline_size: (32.0, 16.0),
        // The same function's 17-frame counter: white for `0..7`, invisible
        // for `8..16`. Confidence 70 - see the page for the captures that
        // argue with it.
        underline_on_ticks: 8,
        underline_off_ticks: 9,
    },
    list: oag_title::ListBlocks {
        // `List_Construct`, `0x001bf7f8`: `+0x12c`, `+0x13c`, `+0x140`.
        label_width: 520.0,
        value_width: 280.0,
        value_focus_width: 340.0,
        // `List_CreateWidgets`, `0x001bdb48`: `0x008ad8d0`.
        gap: 10.0,
        // `Block_Construct`'s own default, `0x0018b818`, which `List` keeps.
        height: 40.0,
        // `List_CreateWidgets`: `0x008ad8cc` and `0x008ad8c8`.
        marker_offset: (8.0, 4.0),
        // `List_Construct`: `+0xe8..+0xf4` (32), `+0xf8` (-18), `+0xfc` (-30).
        arrow_size: 32.0,
        arrow_left_offset: -18.0,
        arrow_right_offset: -30.0,
        // `List_Update`, `0x001c03e0`, and measured at alpha `0.247` in
        // `hd-settings-screenshot-2/01.png`.
        arrow_inert: 0x3FFF_FFFF,
    },
    // `0x008ad594`, `0x008afc94` and `0x008ad9c0`: one sixth, in all three.
    ease: 1.0 / 6.0,
};

/// The boot order HD takes, read out of `skin.xml` and then **watched**.
///
/// `DATA00`'s `skin.xml` declares these eight redirects and three cold boots on
/// RPCS3 went through all eight in this order, twice, with nothing extra between
/// them. Confidence **85** - capped only because an emulator is not a PS3. See
/// [`hd-frontend.md`] for the transcript and `just rpcs3-bootchain` to repeat it.
///
/// **[`states::FIRST_PLAY`] is the step that makes the savedata matter.** With a
/// save present HD skips it and goes [`states::EPILEPSY_WARNING`] straight to
/// [`states::SAVE_WARNING`], so a boot watched on a used profile shows seven
/// steps and looks like a whole chain. Both captures behind this constant moved
/// `BCES00664-AUTO-` aside first.
///
/// **Three of the eight steps exist only because the game is online**:
/// [`states::PRE_FMV_CONNECT`] runs a connection check before the logo, and
/// [`states::EULA`] and [`states::UPDATE_ANNOUNCEMENT`] both carry `<SVOData>`.
/// Pulse's boot has no equivalent to any of them.
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

/// HD's boot sequence, as data, with [`BOOT_CHAIN`] as its order.
///
/// **The provenance is the field to read first.** Everything else here is a
/// reading of the disc; `provenance` is what says whether the order among those
/// readings has been watched. It has, as of 2026-09-05. See
/// [`oag_title::Provenance`] and this module's own docs.
///
/// Three fields are `None`/empty and each is a measurement rather than a gap:
///
/// - **No reel.** Pulse's `--reel` is an off-path dev/pub state; HD's two logo
///   reels are both on the boot path, so there is nothing off it to point the
///   flag at.
/// - **No menu backdrop.** HD's menus sit on a real-time `.vex` scene
///   ([`names::FRONT_END_SCENE`]), not a looping movie, so there is no entry to
///   name. This is the same shape of `None` as Pure's, for a different reason:
///   Pure ships no backdrop movie, HD ships something that is not a movie.
/// - **No fallback globals.** All 64 `FEGlobals` the screens name are declared
///   in `skin.xml` itself - `hd-frontend.md` checked, and found none referenced
///   but undeclared in any of the six copies.
pub const BOOT: &oag_title::BootProfile = &oag_title::BootProfile {
    // Read **and** watched, 2026-09-05: three cold boots on RPCS3 walked all eight
    // steps of `BOOT_CHAIN` in this order and no other, savedata moved aside so
    // `FIRST_PLAY` was not skipped. `just rpcs3-bootchain`, kept under
    // `data/reference/hd-boot-chain/`. Confidence 85 - an emulator is not a PS3.
    provenance: oag_title::Provenance::Measured,
    chain: BOOT_CHAIN,
    reel: None,
    menu_backdrop: None,
    menu_scene: false,
    picker_backdrop_parent: None,
    picker_from_loaded_faces: true,
    fallback_globals: &[],
    // No src-less widget found on this title yet.
    fallback_images: &[],
};

/// HD's front end: the layout it authors and the boot order it takes.
///
/// The two halves were recovered to very different standards and
/// [`oag_title::FrontEnd`] cannot say so - which is why [`BOOT`] carries its own
/// provenance rather than this. They have since converged: [`MENU_SKIN`] is
/// confidence 92 off six agreeing copies of an authored file, and [`BOOT_CHAIN`]
/// is 85 off two boots that were watched.
pub const FRONT_END: &oag_title::FrontEnd = &oag_title::FrontEnd {
    root: names::FRONTEND_ROOT,
    language_plugins: LANGUAGE_PLUGINS,
    disc_strings: None,
    language_manifests: &[],
    menu: Some(MENU_SKIN),
    menu_ps2: None,
    // HD authors the controller-driven `FEGlobals`/`<HorizMenu>` vocabulary
    // `MenuSkin` describes, not 2048's touch-icon idiom - see ADR-0054.
    touch: None,
    boot: BOOT,
    menu_frame: Some(states::FE_SCREEN),
    // HD authors its track and ship screens in two files of their own, in
    // its own dialect - see `track_select` and `team_select`.
    race_box: None,
    team_select: Some(names::TEAM_SELECTION_DEFINITION),
    track_select: Some(names::TRACK_SELECTION_DEFINITION),
    race_setup: None,
    // `false`, and not because HD has no 3-D craft on this screen - it
    // does (`ShipModel`). The one preview path this build has reads
    // `<team>\ship_FE.vex`, which HD's own per-team `screen.xml` names and
    // no HD archive carries; the race hull is a `.vex`/`.rcsmodel` pair the
    // preview does not load yet. See `oag_ui_screens::picker::hd`.
    preview_meshes: false,
    // `DATA02`'s copy - `oag_assets::Archives::holder_of`'s own mount order
    // (`data` then `fe` then `extra`) reaches it first, the same precedence
    // [`names::FRONTEND_ROOT`] documents reaching `DATA00`'s `skin.xml`.
    // **This build's own mount order, not a measurement of the original's** -
    // see `docs/formats/hd-endrace-screens.md`.
    endrace_entry: Some(names::ENDRACE_DEFINITION),
    endrace_style: Some(oag_title::EndRaceStyle::measured(
        oag_title::EndRaceDialect::Field,
    )),
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

    /// HD's own `Team Selection` - see `oag_ui_screens::picker::hd` for the screen.
    /// **`DATA06` alone carries it**, and `DATA00`'s live [`FRONTEND_ROOT`]
    /// includes it by `SrcRel="Team_Selection_Definition.xml"` - so there is
    /// no precedence question for this file, unlike most of HD's front end.
    /// The older `Selection_Definition.xml` on `DATA02`/`03`/`05`, which
    /// also authors a `Team Selection`, is included by no live skin.
    pub const TEAM_SELECTION_DEFINITION: &str =
        r"Data\Plugins\Frontend\Gui\Team_Selection_Definition.xml";

    /// HD's own `Track Creation` - see `oag_ui_screens::picker::hd::track`. Like
    /// [`TEAM_SELECTION_DEFINITION`], **`DATA06` alone carries it** and
    /// `DATA00`'s live skin includes it by `SrcRel`. `racebox_definition.xml`
    /// only *names* `Track Creation` as a `goto` target; the screen is not in
    /// it, on any of the five archives that carry that file.
    pub const TRACK_SELECTION_DEFINITION: &str =
        r"Data\Plugins\Frontend\Gui\Track_Selection_Definition.xml";

    /// The three screens a race ends on - `EndRace Results`/`EndRace
    /// Rewards`/`EndRace Menu` - HD's own copy, at a named plugin path like
    /// [`FRONTEND_ROOT`] rather than Pulse's numbered
    /// `Data\Plugins\PI001\GUI\EndRace_Definition.xml`.
    ///
    /// **Present in five of the seven archives, no two alike by MD5** -
    /// `DATA02`/`DATA03`/`DATA04` carry `Results`/`Rewards`/`Menu`, `DATA05`
    /// adds `Podium`, `DATA06` carries `Podium` and no `Rewards` at all. See
    /// `docs/formats/hd-endrace-screens.md` for the widget-by-widget read.
    pub const ENDRACE_DEFINITION: &str = r"Data\Plugins\Frontend\Gui\EndRace_Definition.xml";

    /// The nine-patch every menu `Block` cuts its border from and samples its
    /// fill's alpha off: 64x64 `A8R8G8B8`, white, in `DATA06` only. Named by
    /// the pointer table `Block_Construct` reads (`0x00920a00`), not by any
    /// screen - which is why `oag_game::boot::load_sprites` has to be told
    /// about it. See `docs/ghidra/functions/ps3-hdfury-eu/menu-blocks.md`.
    pub const MENU_BLOCK_FRAME: &str = r"Data\FE\Images\file2.gtf";

    /// The strip's underline mark: a white 21x8 bar at texel `(1, 1)` of a
    /// 32x16 DXT texture in `DATA02`. `HorizMenu_Construct` loads it as
    /// `cursor.mip`, the PSP-era spelling the loader maps to `.gtf`.
    pub const MENU_STRIP_CURSOR: &str = r"Data\FE\Images\cursor.gtf";

    /// The settings rows' step arrow: a white right-pointing triangle in a
    /// 32x32 DXT texture in `DATA02`, drawn mirrored for the left one.
    /// `List_Construct`'s TOC slot `0x4cc`.
    pub const MENU_LIST_ARROW: &str = r"Data\FE\Images\HD_options_arrow.gtf";

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
    /// therefore the whole of the evidence; `oag_video::bik` and
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

    /// The Fury style's backdrop settings: its camera paths and colours, in
    /// `.envsettings` syntax. `BackgroundAnimFury_Load` reads it; see
    /// `oag_tables::fury_backdrop` and [`menu-backdrop.md`].
    ///
    /// [`menu-backdrop.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md
    pub const FURY_SETTINGS: &str = "Data/fe/fury.envsettings";

    /// The nineteen point clouds the Fury backdrop flies past, as the
    /// executable's own table at `0x009209b4` spells them - forward slashes
    /// and mixed case, unlike the backslashed `Data\FE\Images` names beside
    /// it. `Load` picks twelve of these at random per boot; see
    /// [`menu-backdrop.md`].
    ///
    /// [`menu-backdrop.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md
    pub const FURY_CLOUDS: [&str; 19] = [
        "data/FE/Fury/AG_Systems_c1.points2",
        "data/FE/Fury/AG_Systems_n1.points2",
        "data/FE/Fury/Assegai_c1.points2",
        "data/FE/Fury/Assegai_n1.points2",
        "data/FE/Fury/Auricom_c1.points2",
        "data/FE/Fury/Detonator.points2",
        "data/FE/Fury/EGX_c1.points2",
        "data/FE/Fury/Feisar_c1.points2",
        "data/FE/Fury/Goteki_c1.points2",
        "data/FE/Fury/Harimau_c1.points2",
        "data/FE/Fury/Harimau_n1.points2",
        "data/FE/Fury/Icaras_c1.points2",
        "data/FE/Fury/Mirage_c1.points2",
        "data/FE/Fury/Mirage_n1.points2",
        "data/FE/Fury/Piranha_c1.points2",
        "data/FE/Fury/Qirex_c1.points2",
        "data/FE/Fury/Qirex_n1.points2",
        "data/FE/Fury/Triakis_c1.points2",
        "data/FE/Fury/Triakis_n1.points2",
    ];

    /// How many of [`FURY_CLOUDS`] `Load` reads into the widget's table.
    pub const FURY_CLOUDS_LOADED: usize = 12;
}
