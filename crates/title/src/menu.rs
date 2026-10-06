//! How a title's front end lays its menus out and moves between them.
//!
//! # Why this is a type and not two piles of constants
//!
//! [ADR-0022] licenses an axis only once a *second* corpus has been measured,
//! which is the trap ADR-0009 named. This axis clears that bar: both titles'
//! `Skin.xml` files were read off their own discs, and they disagree on every
//! number they share - `MenuXOffset` 50 against 21, `MenuScale` 1.0 against
//! 1.15, `TitleScale` 1.0 against 0.97 - while Pure declares two of the colours
//! Pulse declares not at all. A single set of constants in `oag-pulse` would
//! have to be either wrong for Pure or silently reused as if measured.
//!
//! The layout numbers are the disc's. **The menu *tree* is not**, and stays
//! ours: see `docs/architecture/menus.md`. This type carries presentation only.
//!
//! # Where the numbers came from
//!
//! Two sources, and the difference matters when reading a field's confidence:
//!
//! - **Authored**, read straight out of the title's `Skin.xml` `FEGlobals`
//!   block or its screen definitions. Exact, and re-checkable by a
//!   disc-backed test.
//! - **Measured**, read off a capture of the original running under an
//!   emulator, because the data either does not state it or states it in units
//!   nothing decodes. [`Self::row_extra_leading`] and [`Self::selected`] are
//!   the two.
//!
//! Every field says which it is. See `docs/ui/menus-original.md` for the
//! captures and the confidence scores.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// A packed `0xAARRGGBB` colour, exactly as the front-end XML writes one.
///
/// Kept packed rather than unpacked to `[f32; 4]` so a title package's table
/// reads the same as the `Skin.xml` line it came from, and a reviewer can
/// compare the two without arithmetic.
pub type Argb = u32;

/// A menu drawn as one horizontal strip rather than as a column of rows.
///
/// The widget is the front end's own `<HorizMenu>`, and it is the second menu
/// idiom the dialect has: entries run left to right from one anchor instead of
/// down from one. Everything a `<HorizMenu>` authors is here and nothing else -
/// the widget states a position and a colour, and states no spacing between
/// entries at all, so how far apart they sit is the drawing side's own choice
/// and is marked as such where it is made.
///
/// # Why this is an axis and not a Wipeout HD constant
///
/// [ADR-0022]'s bar is a *second* measured corpus, and this clears it the way
/// the rest of [`MenuSkin`] does - by the two sides disagreeing, measured on
/// each disc rather than assumed:
///
/// Every blob of every archive named below was extracted and searched, on
/// 2026-08-19. The `Menu` column is the control - it is what says the search can
/// see this vocabulary at all - and it is a plain substring, so it counts a
/// screen called `MPMainMenu` as readily as a `<Menu>` widget.
///
/// | disc | archives read | files | holding `Menu` | holding `HorizMenu` |
/// | --- | --- | ---: | ---: | ---: |
/// | `pulse-psp-usa.chd` | `Data.wad`, `FE.wad`, `FEData.wad` | 1,411 | 30 | **0** |
/// | `pulse-psp-eu.chd` | `Data.wad` | 1,138 | 27 | **0** |
/// | `pulse-ps2-eu.chd` | `WADSP.WAD`, `WADS2.WAD` | 7,393 | 76 | **0** |
/// | `pure-psp-eu.chd` | `Data.wad`, `FE.wad`, `FEData.wad` | 1,241 | 46 | **0** |
/// | `hdfury-ps3-eu-dec.iso` | `DATA06`'s front-end tree | 29 | 18 | **8** |
///
/// So `None` on both PSP titles is a **measurement** - four pressings across two
/// titles and two consoles author no such widget anywhere - rather than a field
/// nobody has filled in yet. HD's eight files carry ten `<HorizMenu>` widgets
/// against seventeen vertical `<Menu>`, so it is an idiom of its front end and
/// not one screen's exception.
///
/// The census is a string search on purpose: the PSP dialect shortens element
/// names per file and writes the full name into that file's own `<code>`
/// dictionary, so a `HorizMenu` anywhere would put the literal `HorizMenu` in
/// the blob that used it. See `oag_tables::fexml`. **The PS2 pressing's other
/// two archives, `PRERACE.WAD` and `PS2MUSIC.WAD`, are swept too** (2026-09-01),
/// with zero hits for `Menu` or `HorizMenu` either, but not because the
/// vocabulary is absent: both are `oag_formats::ps2_music`-shaped raw-PCM
/// containers, not `oag_tables::fexml` at all, so neither can hold an XML
/// widget by format. `WADSP.WAD`/`WADS2.WAD` are the whole of that disc's
/// front end.
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuStrip {
    /// Left edge of the first entry, in [`MenuSkin::space`]. Authored, the
    /// widget's own `x`.
    ///
    /// The *left* edge and not a centre: every `<HorizMenu>` measured says
    /// `align="left"`, which is why no alignment field sits beside this one.
    pub x: f32,
    /// Top of the entries' line box, in [`MenuSkin::space`]. Authored, the
    /// widget's own `y`.
    ///
    /// This is what [`MenuSkin::first_row_y`] cannot be for a title whose main
    /// menu is a strip: there is no first *row* to put a `y` on, and the number
    /// that exists belongs to the strip rather than to a column.
    pub y: f32,
    /// The entries' own colour. Authored, the widget's own `color`.
    ///
    /// **Not what a strip's text is drawn in - a 2026-09-01 capture settled
    /// that.** The reasoning once here was that this must override
    /// [`MenuSkin::text`], since a widget declaring its own colour would
    /// otherwise be pointless - plausible, and wrong: an RPCS3 capture of the
    /// real menu shows every entry, selected or not, in the same white
    /// `FEGlobals->TextColor`, not in this widget's `0xff705070`. What that
    /// literal *is* for is still open - it names neither the text nor either
    /// style's tab fill (`HD_Grey`/`HD_Blue`, see [`Self::selected_fill`]) - and
    /// the field stays because it is still what the widget authors, just no
    /// longer read as the text colour. See `docs/formats/hd-frontend.md`.
    pub color: Argb,
    /// The name of the global a *selected* entry's own tab fills with, when
    /// the disc states one. `None` on a title with no strip at all.
    ///
    /// A name, not a colour, for the reason [`crate::loading::Palette`] is:
    /// the value is the FE style rather than a fixed table.
    /// `FEGlobals->HD_Blue` is `0xffac0717` (Fury, red) on `DATA00` and
    /// `0xff8ac0ca` (HD, teal) on `DATA06` - a colour recorded here would pin
    /// one archive's look into a build that has to draw both. The unselected
    /// fill needs no equivalent field: it is `FEGlobals->HD_Grey`, which
    /// `oag_game::menu::frame::Frame::ink` already resolves per served
    /// archive off the frame screen's own marks. Both fills are **confirmed
    /// exact reads**, not approximations: a 2026-09-01 capture's own pixels
    /// are `HD_Grey`/`HD_Blue` to the byte on the served archive. The tab's
    /// *shape* is a separate, much rougher measurement - see
    /// `docs/formats/hd-frontend.md`.
    pub selected_fill: Option<&'static str>,
}

/// One title's menu presentation.
///
/// Held by [`crate::Title`] so that opening one title's archives while drawing
/// another's skin is a state that cannot be constructed - the same argument
/// [`crate::boot::BootProfile`] is hung off `Title` for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuSkin {
    /// The grid every coordinate below was **read in**, as `(width, height)`.
    ///
    /// Not the grid they are drawn in, which belongs to the *source* rather than
    /// to the title: Wipeout Pulse ships a PSP pressing and a PS2 one, this
    /// table was read off the PSP's `Skin.xml`, and the PS2's own file places
    /// widgets in 640x448. So the two questions have different answers for one
    /// title and a single field could not hold both. The caller converts - see
    /// `oag_game::menu::Skin::new`.
    ///
    /// **The field exists because a third title disagreed by a factor of four.**
    /// Both PSP titles author at 480x272 and Wipeout HD authors at 1920x1080,
    /// so until HD's menus were drawn there was exactly one grid and nothing
    /// needed to say which it was. Drawing HD's `MenuXOffset` of 800 as if it
    /// were a PSP coordinate put its whole label column off the side of the
    /// frame; `oag_hd::frontend::MENU_SKIN` predicted that in its own comment
    /// and left it for whoever drew these first.
    pub space: (f32, f32),
    /// Left edge of the menu rows. Authored, `FEGlobals->MenuXOffset`.
    pub menu_x: f32,
    /// Scale applied to menu rows. Authored, `FEGlobals->MenuScale`.
    pub menu_scale: f32,
    /// Left edge of the screen title. Authored, `FEGlobals->TitleXOffset`.
    pub title_x: f32,
    /// Top of the screen title. Authored, `FEGlobals->TitleYOffset`.
    pub title_y: f32,
    /// Scale applied to the screen title. Authored, `FEGlobals->TitleScale`.
    pub title_scale: f32,
    /// The font role the screen title is drawn in, as the language plugin
    /// names it - the same mechanism as [`Self::menu_font`], one widget up.
    ///
    /// **`None` means "draw the title in the default face", which is what
    /// this build has always done and is not the same claim as "this title's
    /// chrome authors no role".** Wipeout HD's `mainmenu_definition.xml`
    /// authors `<Text idstring="FE_MM" font="Title" ...>` on its main-menu
    /// title, resolving to `helvb.fnt` (44px) against `Default`'s `helv.fnt`
    /// (33px) - see `docs/formats/fnt.md`'s font table and
    /// `docs/formats/hd-frontend.md`'s `TitleColor` section for the widget
    /// itself. `oag_hd::frontend::MENU_SKIN` is `Some("Title")`.
    ///
    /// **Pulse's own `MainMenu_Definition.xml` authors the same thing** -
    /// `<b d="FE_MM" p="title" ...>` (`p` is `font` in that file's own tag
    /// dictionary), matched case-insensitively by [`oag_ui::language::Language::font`]
    /// to the same `Title` slot, which Pulse's plugins resolve to
    /// `Pulse_14.fnt` (17px) against `Default`'s 13px `pulse_text.fnt` - and
    /// `TournamentLoad`'s own title says `font="Title"` too. Read directly off
    /// `Data.wad`'s `Data\Plugins\PI001\GUI\MainMenu_Definition.xml`,
    /// 2026-09-13, while wiring this field for HD.
    ///
    /// **Left `None` for Pulse anyway - a deliberate scope cut, not a second
    /// measurement disagreeing with the first.** Flipping it would move
    /// already-capture-verified, confidence-95 output
    /// (`docs/ui/menus-original.md`'s Layout table) with no capture of its
    /// own checking whether the title's face is what changes; that is a
    /// separate pass this one did not budget for. Whoever takes it next has
    /// the widget already found, not just the question. Still `None` on both
    /// PSP Pulse pressings and the PS2 port for exactly that reason.
    ///
    /// **Pure is checked now, and wired: `oag_pure::frontend::MENU_SKIN` is
    /// `Some("Title")`.** `Skin.xml`'s own `Main Menu` screen authors
    /// `idstring="Main Menu" font="Title" x="FEGlobals->TitleXOffset"
    /// y="FEGlobals->TitleYOffset" scale="FEGlobals->TitleScale"
    /// color="FEGlobals->TitleColor"`, read directly off both
    /// `pure-psp-eu.chd` and `pure-psp-usa.chd`, 2026-09-25. Unlike Pulse,
    /// this one carried no capture risk to weigh: Pure's `Title` role
    /// resolves to `FX300ANG.fnt`, the *same file* `Default` already does
    /// (`oag_ui::language::roles`' own font table), so the flip changes
    /// nothing about which glyphs draw - only that the title now goes
    /// through its own authored role rather than an untagged `Draw::Text`
    /// that happened to land on the identical atlas.
    pub title_font: Option<&'static str>,
    /// Top of the first row, in the layer the rows are drawn in.
    ///
    /// Authored per *screen* rather than in `FEGlobals` - Pulse's `Main Menu`
    /// says `y="32"` - so a title whose screens were never read leaves this
    /// `None` and the caller keeps its own value.
    ///
    /// This is the top of the row's **line box**, not of its glyphs: a capture
    /// puts the first row's ink at y=40 for a 22-pixel face, and that 8-pixel
    /// inset is already baked into the glyph boxes the atlas hands back.
    ///
    /// A title whose main menu is a strip has no first row for this to be the
    /// top of, and says so by leaving it `None` and filling [`Self::strip`]
    /// instead. The two are not alternatives in general - a strip title still
    /// authors vertical `<Menu>` widgets elsewhere - but for the one screen
    /// each field is read off, they are.
    pub first_row_y: Option<f32>,
    /// Added to the row font's line height to get the row pitch. **Measured.**
    ///
    /// `None` for a title whose menus have not been captured, and then the
    /// caller supplies its own leading rather than borrowing the other title's
    /// measurement - the rule below was measured on Pulse and there is no
    /// evidence it holds anywhere else.
    ///
    /// The authored `gap` attribute is *not* this, and reading it as row
    /// spacing produces a visibly wrong menu. Four of Pulse's menus, across two
    /// faces and two authored gaps, all measure `line height + 6`:
    ///
    /// | screen | `gap` | face line height | pitch |
    /// | --- | --- | --- | --- |
    /// | `Main Menu` | 15 | 22 | 28 |
    /// | `Racebox` | 15 | 22 | 28 |
    /// | `Single Player` | - | 17 | 23 |
    /// | `Cell Setup` | 0 | 17 | 23 |
    ///
    /// `gap` 0 and `gap` 15 give the same pitch for the same face, so whatever
    /// `gap` is for, it is not this. The 6 is a measured constant of unknown
    /// origin; nothing here claims to have derived it.
    pub row_extra_leading: Option<f32>,
    /// The font role menu rows are drawn in, as the language plugin names it.
    ///
    /// Authored - Pulse's `Main Menu` rows say `font="menu"`, which its language
    /// plugins resolve to a face two-thirds taller than the body one.
    ///
    /// **`None` means "draw the rows in the default face", which covers two
    /// different findings.** A title whose menu definitions were never read
    /// leaves this `None` because nothing is known; a title whose definitions
    /// *were* read and name no separate role leaves it `None` because there is
    /// nothing to name - Pure's measured case, recorded in
    /// `oag_pure::frontend::MENU_SKIN`. The caller does the same thing either
    /// way, so the distinction lives in each title crate's own comment rather
    /// than in this type.
    pub menu_font: Option<&'static str>,
    /// Unselected row text, as this title's own menu screens draw it.
    ///
    /// `FEGlobals->TextColor` on both PSP titles, whose menu widgets name that
    /// global and nothing else. **Wipeout HD is the case that made the wording
    /// "as its screens draw it" rather than "TextColor"**: it declares that
    /// global white and clears its whole front end to a white `HD_BG`, so the
    /// two cannot both describe what is on screen. See
    /// `oag_hd::frontend::MENU_SKIN`, which records the contradiction rather
    /// than resolving it, and takes the colour its screens actually use.
    ///
    /// `None` for a title whose `Skin.xml` does not declare it, which is Pure's
    /// case. The caller supplies its own and must not borrow the other title's.
    pub text: Option<Argb>,
    /// Screen title text. Authored, `FEGlobals->TitleColor`.
    ///
    /// Pulse's is `0xFF000000`, black, because its title sits on a light top
    /// bar this build does not draw yet. `None` where undeclared.
    pub title: Option<Argb>,
    /// What a menu clears to when its own frame screen carries neither a
    /// `<ScreenClear>` nor a full-screen fill of its own. **Measured, and
    /// only ever needed once**: Wipeout HD authors `<ScreenClear
    /// Colour="FEGlobals->HD_BG">` on `FE Screen` and Pulse's own `FE Screen`
    /// opens with a full-screen black `<Image>`, so [`super::Frame::clear`]
    /// already carries this for both and this field stays `None`. Pure names
    /// a widget for exactly this job - `BackgroundController`'s own
    /// `BackgroundImage`, sized to the full screen - and then leaves it
    /// without a `src`, so [`super::Frame::clear`] comes back `None` too, on
    /// a screen this build used to clear to black for want of anything else.
    /// A captured `Main Menu` (`pure-psp-usa.chd`, PPSSPP 1.20.4, 2026-08-25)
    /// shows solid white behind the row list, so `0xFFFFFFFF` is what stands
    /// in: an "engine carries a compiled-in default this project has not
    /// found" gap. `oag_pure::frontend::FALLBACK_GLOBALS` used to document
    /// four colours the same way and no longer does - the disc authored them
    /// after all, in a *style* skin the front-end root does not include, so
    /// **check for one before concluding a name is undeclared**. `None` for a
    /// title whose frame already supplies its own clear, or that draws no
    /// frame at all.
    pub background: Option<Argb>,
    /// The selected row. **Measured on Pulse; authored on Wipeout HD.**
    ///
    /// The two routes are worth telling apart, and the field cannot say which
    /// it holds - each title's own comment does. Pulse's XML states no selected
    /// colour anywhere, so its value came off a capture. HD's states one:
    /// twenty-five `highlightColor` attributes across its front end, every
    /// single one `FEGlobals->HD_Blue`, always paired with a `color` of
    /// `HD_Grey` - which is this pair of fields, on widgets other than a menu.
    ///
    /// Nothing in Pulse's XML states a selected colour. A capture shows the row
    /// brightened toward white rather than given a fill bar or the pink
    /// `MenuHighLightArrowColor`, which appears nowhere on these screens.
    ///
    /// **This is the pulse's peak, not a static colour, when
    /// [`Self::selected_pulse_period_secs`] is `Some`.** A frame-accurate
    /// PPSSPP capture (breaking on `Gfx_PresentFrame`, 150 consecutive
    /// presented frames, one uncontaminated run) superseded the two ad hoc
    /// samples this field used to hold (`rgb(157,255,255)`,
    /// `rgb(107,226,247)`, both channel-clipped and irreconcilable under any
    /// single lerp): the brightest pixel of the selected label's own ink hit
    /// exactly `0xFFFFFFFF` on three independent cycles, while the same pixel
    /// on an unselected row never moved off `TextColor`. See
    /// `docs/ui/menus-original.md`.
    ///
    /// `None` where no capture has been taken; the caller supplies its own.
    pub selected: Option<Argb>,
    /// How long one brighten-and-return cycle of [`Self::selected`] takes, in
    /// seconds. **Measured on Pulse only.**
    ///
    /// The same capture as [`Self::selected`]: the selected label's own ink
    /// cycles between `TextColor` and white with an exact, repeating period of
    /// 33 presented frames across four consecutive cycles (troughs at frames
    /// 12, 45, 78, 111, 144 - each exactly 33 apart), at the front end's own
    /// measured 30 Hz presentation rate - `33 / 30 = 1.1` seconds. Confidence
    /// 90: one capture, but four cycles agreeing to the frame and a flat
    /// control column (an unselected row sampled at the same coordinates
    /// never moved). The shape of the curve between the two endpoints was not
    /// solved - eleven discrete brightness levels were measured, unevenly
    /// spaced, which rules out a linear ramp but does not by itself name a
    /// curve - so the drawing side picks its own shape between the measured
    /// endpoints and period, marked as ours the same way
    /// `oag_game::anim::Tween::eased`'s own invented curve is.
    ///
    /// `None` where no capture exists (HD authors [`Self::selected`] as a
    /// flat, undecorated colour - a 2026-09-05 census of its whole front-end
    /// XML found no oscillation attribute anywhere) or where the highlight's
    /// own behaviour is simply unmeasured (Pure). `None` must never be filled
    /// in from Pulse's own number - see `docs/ui/menus-original.md`.
    pub selected_pulse_period_secs: Option<f32>,
    /// How long a page change takes, in seconds. Authored, `transition=`.
    ///
    /// Confirmed as seconds rather than assumed: Pulse's `Main Menu` is
    /// authored `0.5` and its transition runs about 13 presented frames against
    /// a front end measured at 30 Hz.
    pub transition_secs: f32,
    /// The main menu's `<HorizMenu>`, for a title that draws one.
    ///
    /// **`None` is a measurement here, not a gap** - see [`MenuStrip`] for the
    /// census that makes it one. A title that authors no such widget anywhere is
    /// not a title whose strip is unread.
    ///
    /// Read off the *main menu's own* widget rather than pooled across the
    /// front end, because a strip is authored per screen exactly as
    /// [`Self::first_row_y`] is: Wipeout HD's ten `<HorizMenu>` widgets agree on
    /// `x` and on `align`, and six of them say `y="125"` against four at
    /// `y="140"`.
    pub strip: Option<MenuStrip>,
    /// The box this title's executable draws behind a menu entry, and the
    /// widths it gives the widgets that use it. **Recovered from the
    /// executable**, not read off the disc's XML - see [`MenuBlocks`].
    ///
    /// `None` for a title whose entries are bare text, which is what both
    /// PSP titles' captures show, and for one whose executable has not been
    /// read for it.
    pub blocks: Option<MenuBlocks>,
    /// Where this title's settings rows sit, when its screens author a
    /// position for them separately from `MenuXOffset`. See [`MenuList`].
    pub list: Option<MenuList>,
    /// A row's second line, drawn only under the selected row, for a title
    /// whose main menu screen was captured for it. See [`HelpText`].
    ///
    /// `None` for a title with no capture, and the caller draws no subtitle
    /// at all rather than guessing at Pulse's own numbers - the same rule
    /// [`Self::selected_pulse_period_secs`] states for its own pulse.
    pub help_text: Option<HelpText>,
}

/// A row's help text, as `MainMenu_Definition.xml`'s own seven `helptext`
/// widgets place and colour it. **Layout and colour only** - what the text
/// actually *says* is this project's own wording, not the disc's marketing
/// copy; see `docs/architecture/menus.md#a-per-row-subtitle` for why.
///
/// Measured off Pulse, `docs/ui/menus-original.md`'s own Layout and Colours
/// tables, confidence 90: `helptext0` through `helptext6` sit at `y = 50, 78,
/// 106, ...`, each exactly 18 below its own row's `y = 32, 60, 88, ...`, and
/// only the selected row's ever draws - the seven widgets share one `x` and
/// would otherwise overlap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HelpText {
    /// How far below the selected row's own `y` the subtitle sits, in
    /// [`MenuSkin::space`]. `18.0` on Pulse.
    pub offset_y: f32,
    /// The subtitle face's scale, as a multiple of [`MenuSkin::menu_scale`]
    /// rather than an absolute size.
    ///
    /// **`1.0` on Pulse, confidence 90** since the caller routes this
    /// widget's own draw to its own `Default`-role atlas rather than the
    /// row face's - `oag_ui::menu::rows::draw_text_rows` via a second atlas
    /// `oag-game`'s menu stage loads beside its `menu`-role primary (see
    /// `crates/game/src/boot/fonts.rs`'s `face_atlas_slot`). Before that,
    /// this field carried a derived `13.0 / 22.0` at confidence 70: the row
    /// face and the help-text face are two different widgets and neither
    /// authors a `scale` relative to the other, so it was two numbers
    /// measured by different methods (`docs/ui/menus-original.md`'s row
    /// face, 22px, `font="menu"`; and its own note that the authored y=50
    /// draws "the 13-pixel help text") rather than a single ruler on both.
    /// A title whose menu stage has no second atlas for this role falls
    /// back to drawing it through the row face at whatever this states, so
    /// `1.0` on such a title would be the *old* bug, not this fix - a title
    /// adding this field has to add the routing too, not just the number.
    pub scale: f32,
    /// The subtitle's ink colour. `0xFFFFFFFF` on Pulse - `helptext0`'s own
    /// `color="0xffffffff"`, authored rather than measured, the one part of
    /// this type the XML actually states.
    pub color: Argb,
}

/// Where a title's settings rows are anchored, read off its screens.
///
/// **Authored.** Wipeout HD's `Settings` screens do not use `MenuXOffset` at
/// all: every one of them nests its `<List>`/`<Slider>` rows inside an
/// `<Item OffsetX="160" OffsetY="170">` and authors each row's own `y` at
/// `0, 50, 100, ...` - `additional_definition.xml`, all four archives that
/// carry it. So the column's left edge, its top and its pitch are three
/// authored numbers, distinct from [`MenuSkin::menu_x`] and
/// [`MenuSkin::first_row_y`], which a title whose rows *are* placed by the
/// globals keeps using.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuList {
    /// The `<Item>`'s `OffsetX`, in [`MenuSkin::space`].
    pub x: f32,
    /// The `<Item>`'s `OffsetY`, in [`MenuSkin::space`].
    pub y: f32,
    /// The step between two rows' authored `y`, in [`MenuSkin::space`].
    pub pitch: f32,
    /// The rows' own `scale`, as every `<List>` and `<Slider>` on those
    /// screens authors it (`scale="0.8"` on HD), multiplying
    /// [`MenuSkin::menu_scale`].
    pub text_scale: f32,
}

/// The `Block` a title draws behind a menu entry, as its executable draws it.
///
/// **Every number here is the executable's, not the disc's** - which is the
/// third provenance this type carries, beside the authored and measured
/// fields the rest of [`MenuSkin`] documents. Wipeout HD's GUI XML authors a
/// `<HorizMenu>`'s position and colour and nothing about the box behind each
/// entry; the box, its border, its translucent inside, the selected entry
/// being wider and easing toward that width are all `Block_Item.cpp`,
/// `HorizMenu_Item.cpp` and `List_Item.cpp` in `EBOOT.elf`. See
/// `docs/ghidra/functions/ps3-hdfury-eu/menu-blocks.md` for every address.
///
/// What *is* the disc's is named here rather than transcribed: the textures
/// are entry names for the caller to decode, and the fill's alpha is a texel
/// of one of them, sampled by whoever decodes it rather than written down.
///
/// The geometry of the box itself - the three-unit border, the two-unit fill
/// inset, the ten-unit chamfer, the seventeen-unit landing, the 40x18 corner
/// pieces - is the drawing routine's and lives with the drawing code, the
/// way a reimplemented function's constants do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuBlocks {
    /// The nine-patch every block's border is cut from and whose swatch
    /// texels its fill samples: `Data\FE\Images\file2.gtf`, named by a
    /// pointer table the `Block` constructor reads (`0x00920a00`).
    pub frame_texture: &'static str,
    /// The strip's underline mark: `Data\FE\Images\cursor.gtf`, loaded by
    /// `HorizMenu_Construct`.
    pub cursor_texture: &'static str,
    /// The settings rows' step arrow: `Data\FE\Images\HD_options_arrow.gtf`,
    /// loaded by `List_Construct`.
    pub arrow_texture: &'static str,
    /// Whether the three textures above are stored in a `.gnf` with the
    /// rows in the order HD's `.gtf` keeps them, bottom-up, so the sheet
    /// must reverse them as it does a `.gtf`'s.
    ///
    /// `true` on Omega, `false` on HD (whose `.gtf` decode is reversed for
    /// every image already). A `.gnf` carries no flag for it: Omega's
    /// packager re-authored the images a screen *names* top-down (276 of 276
    /// that differ from HD's), but the eight the executable draws on its own
    /// (`file`, `file2`, `cursor`, `corner`, `corner2`, `square`, `line`,
    /// `unlocked_corner`) are HD's bytes exactly, rows in HD's file order -
    /// `docs/formats/omega-status.md`, "The menu blocks".
    pub art_rows_bottom_up: bool,
    /// The `<HorizMenu>` widget's own numbers.
    pub strip: StripBlocks,
    /// The `<List>` widget's own numbers.
    pub list: ListBlocks,
    /// How much of the remaining distance a block's width closes per tick
    /// toward its target: `0.16667` in all three widgets
    /// (`HorizMenu_LayoutBlocks`, `VertMenu_LayoutBlocks`, `List_Update`).
    /// Per *frame* in the original, which runs its front end at 60 Hz - the
    /// same rate this build's menu stage ticks at.
    pub ease: f32,
}

/// What `HorizMenu_Item.cpp` gives a strip's blocks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripBlocks {
    /// An unselected entry's block width: the `ItemWidth` attribute's
    /// default, `298.0`, set by `HorizMenu_Construct`. The main menu authors
    /// no `ItemWidth`, so this is what it draws with.
    pub item_width: f32,
    /// How much wider the selected entry's block is: `70.0` in
    /// `HorizMenu_LayoutBlocks`. `368 + 4 * 298 + 4 * 10 = 1600`, the frame
    /// rules' own span.
    pub focus_extra: f32,
    /// The step from one block's right edge to the next block's left:
    /// `10.0`, and also the label's inset from its block's left edge.
    pub gap: f32,
    /// Block height, `64.0`, set by `HorizMenu_AddEntryBlock`.
    pub height: f32,
    /// Where the underline `Image` is placed, relative to its entry's block:
    /// `(10.0, 35.0)`. The mark's own bar is at texel `(1, 1)` of a 32x16
    /// texture, so the bar's left edge is one unit further in.
    pub underline_offset: (f32, f32),
    /// The underline image's size, `(32.0, 16.0)`.
    pub underline_size: (f32, f32),
    /// The underline's blink: visible for this many ticks, then hidden for
    /// [`Self::underline_off_ticks`], on a free-running counter.
    pub underline_on_ticks: u32,
    /// See [`Self::underline_on_ticks`]. `8` on and `9` off in
    /// `HorizMenu_LayoutBlocks`.
    pub underline_off_ticks: u32,
}

/// What `List_Item.cpp` gives a settings row's blocks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListBlocks {
    /// The label block's width, `520.0`.
    pub label_width: f32,
    /// The value block's width when the row is not focused, `280.0`.
    pub value_width: f32,
    /// The value block's width when the row is focused, `340.0`; it eases
    /// between the two at [`MenuBlocks::ease`].
    pub value_focus_width: f32,
    /// Between the label block's right edge and the value block's left,
    /// `10.0`.
    pub gap: f32,
    /// Both blocks' height, `40.0`.
    pub height: f32,
    /// The step arrows' size, `32.0` square.
    pub arrow_size: f32,
    /// Where the row's marker `Image` sits, relative to the label block's
    /// corner: `(8.0, 4.0)`. The label's text follows it, one arrow-width
    /// further in; the arrows share its `y`.
    pub marker_offset: (f32, f32),
    /// The **left** arrow's `x`, relative to the label block's right edge;
    /// it is drawn mirrored, extending leftward from there: `-18.0`.
    pub arrow_left_offset: f32,
    /// The **right** arrow's `x`, relative to the label block's right edge:
    /// `-30.0`.
    pub arrow_right_offset: f32,
    /// The colour an arrow draws in when a step that way is impossible:
    /// white at a quarter alpha, `0x3FFF_FFFF`. Measured at `0.247` over two
    /// backgrounds in the Fury settings capture, and the same literal
    /// `List_Update` writes for a disabled row.
    pub arrow_inert: Argb,
}

impl MenuSkin {
    /// The distance between two rows drawn in a `line_height`-tall face.
    ///
    /// `extra` is what the caller uses when this title's own leading is
    /// unmeasured - passed in rather than defaulted here so the number that
    /// ends up on screen belongs to whoever is drawing, and a title package's
    /// silence never quietly turns into another title's measurement.
    ///
    /// The scale multiplies both terms because a scaled row is a scaled gap
    /// too - which is untested against the original, every menu measured so far
    /// being authored at scale 1.0. Pure's `MenuScale` of 1.15 is the first
    /// case that will exercise it.
    #[must_use]
    pub fn row_pitch(&self, line_height: f32, extra: f32) -> f32 {
        (line_height + self.row_extra_leading.unwrap_or(extra)) * self.menu_scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pulse's own numbers, as a fixture rather than as a dependency: this
    /// crate must not know any title's data, so the check that the arithmetic
    /// is right cannot reach for `oag-pulse`.
    const PULSE_SHAPED: MenuSkin = MenuSkin {
        space: (480.0, 272.0),
        menu_x: 50.0,
        menu_scale: 1.0,
        title_x: 50.0,
        title_y: 0.0,
        title_scale: 1.0,
        // This fixture predates the finding in `title_font`'s own doc that
        // Pulse's disc does author the role - kept `None` here too, matching
        // the shipped `oag_pulse::FRONT_END`'s own deliberate scope cut.
        title_font: None,
        first_row_y: Some(32.0),
        row_extra_leading: Some(6.0),
        menu_font: Some("menu"),
        text: Some(0xFF33_A6B9),
        title: Some(0xFF00_0000),
        background: None,
        selected: Some(0xFFFF_FFFF),
        selected_pulse_period_secs: Some(1.1),
        transition_secs: 0.5,
        // Pulse's own answer: its discs author no `<HorizMenu>` at all.
        strip: None,
        blocks: None,
        list: None,
        help_text: None,
    };

    /// The four pitches measured off the original, reproduced by the rule.
    #[test]
    fn the_pitch_rule_reproduces_every_measured_menu() {
        // The 22-pixel `menu` face, on `Main Menu` and `Racebox`.
        assert!((PULSE_SHAPED.row_pitch(22.0, 0.0) - 28.0).abs() < f32::EPSILON);
        // The 17-pixel `small` face, on `Single Player` and `Cell Setup`.
        assert!((PULSE_SHAPED.row_pitch(17.0, 0.0) - 23.0).abs() < f32::EPSILON);
    }

    /// A scaled skin scales the leading with the text, rather than leaving a
    /// constant gap that would crowd at 1.15 and sprawl at 0.8.
    #[test]
    fn scale_reaches_the_leading_too() {
        let scaled = MenuSkin {
            menu_scale: 2.0,
            ..PULSE_SHAPED
        };
        assert!((scaled.row_pitch(22.0, 0.0) - 56.0).abs() < f32::EPSILON);
    }

    /// A title that measured no leading of its own takes the caller's, not the
    /// measurement some other title happens to carry.
    #[test]
    fn an_unmeasured_leading_falls_back_to_the_callers() {
        let unmeasured = MenuSkin {
            row_extra_leading: None,
            ..PULSE_SHAPED
        };
        assert!((unmeasured.row_pitch(22.0, 2.0) - 24.0).abs() < f32::EPSILON);
    }
}
