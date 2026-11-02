//! How a title's front end lays its menus out and moves between them.
//!
//! # Why a type
//!
//! [ADR-0022] licenses an axis once a *second* corpus has been measured. Both
//! titles' `Skin.xml` files were read off their discs and disagree on every number
//! they share (`MenuXOffset` 50 against 21, `MenuScale` 1.0 against 1.15,
//! `TitleScale` 1.0 against 0.97), and Pure declares two colours Pulse declares
//! not at all. One set of constants in `oag-pulse` would be wrong for Pure or
//! silently reused as if measured.
//!
//! The layout numbers are the disc's. **The menu *tree* is not** and stays ours
//! (`docs/architecture/menus.md`); this type carries presentation only.
//!
//! # Where the numbers came from
//!
//! - **Authored**: read out of the title's `Skin.xml` `FEGlobals` block or its
//!   screen definitions. Exact, and re-checkable by a disc-backed test.
//! - **Measured**: read off a capture of the original under an emulator, where
//!   the data does not state it or states it in units nothing decodes
//!   ([`Self::row_extra_leading`] and [`Self::selected`]).
//!
//! Every field says which. Captures and confidence scores:
//! `docs/ui/menus-original.md`.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// A packed `0xAARRGGBB` colour, as the front-end XML writes one, so a title
/// package's table reads like the `Skin.xml` line it came from.
pub type Argb = u32;

/// A menu drawn as one horizontal strip rather than as a column of rows.
///
/// The widget is the front end's `<HorizMenu>`, the dialect's second menu idiom:
/// entries run left to right from one anchor. It states a position and a colour
/// and no spacing between entries, so the spacing is the drawing side's choice.
///
/// # An axis, not an HD constant
///
/// [ADR-0022]'s bar is a second measured corpus, cleared by the two sides
/// disagreeing. A string search of every blob of every archive of four PSP/PS2
/// pressings found `<Menu>` widgets (30, 27, 76 and 46 files) and **no**
/// `HorizMenu`; HD's eight front-end files carry ten `<HorizMenu>` widgets
/// against seventeen vertical `<Menu>`. So `None` on the PSP titles is a
/// **measurement**, and the strip is an idiom of HD's front end. The census
/// table, and why the PS2 pressing's `PRERACE.WAD`/`PS2MUSIC.WAD` cannot hold the
/// widget by format, are in `docs/formats/hd-frontend.md`, "It is an axis, and
/// here is the measurement that licensed it".
///
/// The search is a plain substring on purpose: the PSP dialect writes the full
/// element name into each file's `<code>` dictionary (`oag_tables::fexml`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuStrip {
    /// Left edge of the first entry, in [`MenuSkin::space`]. Authored, the widget's
    /// own `x`: the *left* edge, since every `<HorizMenu>` measured says
    /// `align="left"`.
    pub x: f32,
    /// Top of the entries' line box, in [`MenuSkin::space`]. Authored, the widget's
    /// own `y`. A strip has no first *row* for [`MenuSkin::first_row_y`] to
    /// position.
    pub y: f32,
    /// The entries' own colour. Authored, the widget's own `color`.
    ///
    /// **Not what a strip's text is drawn in.** An RPCS3 capture shows every
    /// entry, selected or not, in the same white `FEGlobals->TextColor`, not this
    /// widget's `0xff705070`. What the literal is for is open: it names neither
    /// the text nor either style's tab fill (`HD_Grey`/`HD_Blue`, see
    /// [`Self::selected_fill`]). Kept because it is what the widget authors
    /// (`docs/formats/hd-frontend.md`).
    pub color: Argb,
    /// The name of the global a *selected* entry's tab fills with, when the disc
    /// states one. `None` on a title with no strip.
    ///
    /// A name, not a colour, as for [`crate::loading::Palette`]: the value is the
    /// FE style. `FEGlobals->HD_Blue` is `0xffac0717` (Fury, red) on `DATA00` and
    /// `0xff8ac0ca` (HD, teal) on `DATA06`. The unselected fill is
    /// `FEGlobals->HD_Grey`, resolved per served archive by
    /// `oag_game::menu::frame::Frame::ink`. Both fills are **exact reads** (a
    /// capture's pixels are `HD_Grey`/`HD_Blue` to the byte); the tab's *shape*
    /// is a much rougher measurement (`docs/formats/hd-frontend.md`).
    pub selected_fill: Option<&'static str>,
}

/// One title's menu presentation.
///
/// Held by [`crate::Title`] so that opening one title's archives while drawing
/// another's skin cannot be constructed, as for [`crate::boot::BootProfile`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuSkin {
    /// The grid every coordinate below was **read in**, as `(width, height)`.
    ///
    /// Not the grid they are drawn in, which belongs to the *source*: Pulse ships a
    /// PSP and a PS2 pressing, this table was read off the PSP's `Skin.xml`, and
    /// the PS2's file places widgets in 640x448. The caller converts
    /// (`oag_game::menu::Skin::new`).
    ///
    /// **Exists because HD disagreed by a factor of four**: both PSP titles author
    /// at 480x272 and HD at 1920x1080. HD's `MenuXOffset` of 800 read as a PSP
    /// coordinate put its label column off the frame.
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
    /// The font role the screen title is drawn in, as the language plugin names it
    /// (the mechanism of [`Self::menu_font`], one widget up).
    ///
    /// **`None` draws the title in the default face**, which is not a claim that
    /// the title's chrome authors no role. HD's `mainmenu_definition.xml` authors
    /// `<Text idstring="FE_MM" font="Title" ...>`, resolving to `helvb.fnt` (44px)
    /// against `Default`'s `helv.fnt` (33px) (`docs/formats/fnt.md`;
    /// `docs/formats/hd-frontend.md`, `TitleColor`); `oag_hd::frontend::MENU_SKIN`
    /// is `Some("Title")`.
    ///
    /// **Pulse authors the same**: `<b d="FE_MM" p="title" ...>` in
    /// `Data\Plugins\PI001\GUI\MainMenu_Definition.xml`, resolving to `Pulse_14.fnt`
    /// (17px) against `Default`'s 13px `pulse_text.fnt`; `TournamentLoad` says
    /// `font="Title"` too. Wired on both PSP pressings and the PS2 port: a capture
    /// puts `MAIN MENU` at 17 px a glyph where the `menu` face drew 25, the ratio
    /// of `Pulse_14.fnt` to `Pulse_20.fnt` (`docs/ui/menus-original.md`, "What
    /// still differs, ranked").
    ///
    /// **Pure is wired too**: `Skin.xml`'s `Main Menu` authors `font="Title"` with
    /// `TitleXOffset`/`TitleYOffset`/`TitleScale`/`TitleColor` (read on
    /// `pure-psp-eu.chd` and `pure-psp-usa.chd`). Pure's `Title` role resolves to
    /// `FX300ANG.fnt`, the same file `Default` does, so the glyphs do not change;
    /// the title now goes through its authored role.
    pub title_font: Option<&'static str>,
    /// The role of the face widgets other than the screen title draw their body
    /// text in, **when it has to be loaded beside [`Self::title_font`]**: Pulse's
    /// `Default` role (`pulse_text.fnt`, the only face with real lowercase), used by
    /// the footer prompts, a picker's labels and the ticker while the title draws
    /// in `Title` (`Pulse_14.fnt`). The renderer has one slot for each. `None` for
    /// a title whose body text is the `menu` face or its title's face (HD, Pure,
    /// 2048, Omega).
    pub body_font: Option<&'static str>,
    /// The role of the face a `font="Stats"` widget draws in, **when the title has
    /// a face of its own for it** and no `Buttons` face to hold the renderer's
    /// third glyph slot: Pure's `Stats` (`LTe50325.fnt`), the small letter-spaced
    /// face its team and circuit panels print labels in (PPSSPP capture of `Team
    /// Selection` and `Track Selection`, `docs/ui/menus-original.md`). `None` draws
    /// a `Stats` widget in the menu face.
    pub stats_font: Option<&'static str>,
    /// Whether a selection screen's `font="default"` rows draw at their authored
    /// scale in the one face the title has (`true`: HD and Omega, which load no
    /// `menu`-role face) or as the `menu` face scaled by Pulse's `default` over
    /// `menu` ratio (`false`: Pulse's measured 13/22, and Pure, whose capture it
    /// matches). `true` is **measured on HD only**: Ship Select and Track Select
    /// stat and record rows were 0.59 of the original's size with the Pulse ratio.
    pub default_face_is_row_face: bool,
    /// Top of the first row, in the layer the rows are drawn in.
    ///
    /// Authored per *screen*, not in `FEGlobals` (Pulse's `Main Menu` says
    /// `y="32"`), so a title whose screens were never read leaves this `None` and
    /// the caller keeps its own value.
    ///
    /// The top of the row's **line box**, not its glyphs: a capture puts the first
    /// row's ink at y=40 for a 22-pixel face, an 8-pixel inset already in the
    /// atlas's glyph boxes.
    ///
    /// A title whose main menu is a strip leaves this `None` and fills
    /// [`Self::strip`]; the two are alternatives only for the one screen each is
    /// read off (a strip title still authors vertical `<Menu>` widgets elsewhere).
    pub first_row_y: Option<f32>,
    /// Added to the row font's line height to get the row pitch. **Measured.**
    ///
    /// `None` for a title whose menus have not been captured: the caller supplies
    /// its own leading rather than borrowing the other title's measurement.
    ///
    /// The authored `gap` attribute is *not* this; reading it as row spacing
    /// draws a visibly wrong menu. Four of Pulse's menus, across two faces and
    /// two authored gaps, all measure `line height + 6`:
    ///
    /// | screen | `gap` | face line height | pitch |
    /// | --- | --- | --- | --- |
    /// | `Main Menu` | 15 | 22 | 28 |
    /// | `Racebox` | 15 | 22 | 28 |
    /// | `Single Player` | - | 17 | 23 |
    /// | `Cell Setup` | 0 | 17 | 23 |
    ///
    /// `gap` 0 and 15 give the same pitch for the same face. The 6 is a measured
    /// constant of unknown origin.
    pub row_extra_leading: Option<f32>,
    /// The font role menu rows are drawn in, as the language plugin names it.
    /// Authored: Pulse's `Main Menu` rows say `font="menu"`, which its plugins
    /// resolve to a face two-thirds taller than the body one.
    ///
    /// **`None` draws the rows in the default face** and covers two findings: a
    /// title whose menu definitions were never read (nothing known), and one whose
    /// definitions were read and name no separate role (Pure,
    /// `oag_pure::frontend::MENU_SKIN`). The caller does the same either way.
    pub menu_font: Option<&'static str>,
    /// Unselected row text, as this title's own menu screens draw it.
    ///
    /// `FEGlobals->TextColor` on both PSP titles. HD is why the wording is "as its
    /// screens draw it": it declares that global white yet clears its front end to
    /// a white `HD_BG`, so the two cannot both describe the screen
    /// (`oag_hd::frontend::MENU_SKIN` takes the colour its screens use).
    ///
    /// `None` where `Skin.xml` does not declare it (Pure); the caller supplies its
    /// own and must not borrow the other title's.
    pub text: Option<Argb>,
    /// Screen title text. Authored, `FEGlobals->TitleColor`. Pulse's is
    /// `0xFF000000` because its title sits on a light top bar this build does not
    /// draw yet. `None` where undeclared.
    pub title: Option<Argb>,
    /// What a menu clears to when its frame screen carries neither a
    /// `<ScreenClear>` nor a full-screen fill. **Measured, and needed once**: HD
    /// authors `<ScreenClear Colour="FEGlobals->HD_BG">` on `FE Screen` and
    /// Pulse's `FE Screen` opens with a full-screen black `<Image>`, so
    /// [`super::Frame::clear`] carries this for both and the field stays `None`.
    /// Pure's `BackgroundController` names a `BackgroundImage` sized to the full
    /// screen with no `src`, so `Frame::clear` is `None` too. A captured `Main
    /// Menu` (`pure-psp-usa.chd`, PPSSPP 1.20.4) shows solid white behind the row
    /// list, so `0xFFFFFFFF` stands in: an "engine carries a compiled-in default
    /// this project has not found" gap. **Check for a name in a *style* skin the
    /// front-end root does not include before concluding it is undeclared**
    /// (`oag_pure::frontend::FALLBACK_GLOBALS` found four that way). `None` where
    /// the frame supplies its own clear or the title draws no frame.
    pub background: Option<Argb>,
    /// The selected row. **Measured on Pulse; authored on HD.**
    ///
    /// Pulse's XML states no selected colour; a capture shows the row brightened
    /// toward white, not a fill bar or the pink `MenuHighLightArrowColor` (which
    /// appears nowhere on these screens). HD states one: twenty-five
    /// `highlightColor` attributes, all `FEGlobals->HD_Blue`, always paired with a
    /// `color` of `HD_Grey`.
    ///
    /// **The pulse's peak, not a static colour, when
    /// [`Self::selected_pulse_period_secs`] is `Some`.** A frame-accurate PPSSPP
    /// capture (150 consecutive presented frames, one run) found the brightest
    /// pixel of the selected label's ink hit exactly `0xFFFFFFFF` on three
    /// independent cycles while the same pixel on an unselected row never left
    /// `TextColor`. Two earlier samples (`rgb(157,255,255)`, `rgb(107,226,247)`)
    /// were channel-clipped and irreconcilable under any single lerp. See
    /// `docs/ui/menus-original.md`.
    ///
    /// `None` where no capture was taken; the caller supplies its own.
    pub selected: Option<Argb>,
    /// How long one brighten-and-return cycle of [`Self::selected`] takes, in
    /// seconds. **Measured on Pulse only.**
    ///
    /// Same capture: the selected label's ink cycles between `TextColor` and white
    /// with a period of 33 presented frames across four consecutive cycles
    /// (troughs at frames 12, 45, 78, 111, 144) at the front end's 30 Hz, `33 / 30
    /// = 1.1` s. Confidence 90: one capture, four cycles agreeing to the frame, and
    /// a flat control column. The curve between the endpoints was not solved
    /// (eleven unevenly spaced levels rule out a linear ramp), so the drawing side
    /// picks its own shape, as `oag_game::anim::Tween::eased` does.
    ///
    /// `None` where no capture exists (HD authors [`Self::selected`] flat, and its
    /// whole front-end XML has no oscillation attribute) or the highlight is
    /// unmeasured (Pure). Never fill it from Pulse's number
    /// (`docs/ui/menus-original.md`).
    pub selected_pulse_period_secs: Option<f32>,
    /// Where `Selector`'s top-left sits relative to the selected hex's own
    /// top-left, in [`Self::space`]'s grid, on Grid Selection and Cell Selection.
    ///
    /// `None` is the PSP's `(-5, -7)` (`oag_ui_screens::campaign`'s
    /// `SELECTOR_ORIGIN`, from a GE dump of widget update `0x088a3e8c`, twin
    /// `GridController_UpdateSelectorPulse` `0x088a5700`). **Pulse PS2 is
    /// `Some((-11, -14))`, measured** by a PCSX2 GS dump: the 56x56 quad sits at the
    /// outline's top-left minus (11, 14) (Grid Selection tile 0 at (87,247),
    /// selector (76,233); Cell Selection tile (155,143), selector (144,129)).
    /// `FUN_001905d8` keeps the PSP's `5.0`/`7.0` literals, so the PS2 art's 56 px
    /// stretch of a 43 px sprite likely explains the rest; the y half is not.
    pub selector_origin: Option<(f32, f32)>,
    /// How long a page change takes, in seconds. Authored, `transition=`. Seconds,
    /// confirmed: Pulse's `Main Menu` is authored `0.5` and runs about 13 presented
    /// frames at the front end's 30 Hz.
    pub transition_secs: f32,
    /// The main menu's `<HorizMenu>`, for a title that draws one.
    ///
    /// **`None` is a measurement**, not a gap (see [`MenuStrip`]). Read off the
    /// *main menu's own* widget, since a strip is authored per screen as
    /// [`Self::first_row_y`] is: HD's ten `<HorizMenu>` widgets agree on `x` and
    /// `align`, six say `y="125"` and four `y="140"`.
    pub strip: Option<MenuStrip>,
    /// The box this title's executable draws behind a menu entry, and the widths
    /// it gives the widgets that use it. **Recovered from the executable**, not
    /// the XML ([`MenuBlocks`]). `None` for a title whose entries are bare text
    /// (both PSP titles' captures) or whose executable has not been read for it.
    pub blocks: Option<MenuBlocks>,
    /// Where this title's settings rows sit, when its screens author a position
    /// for them apart from `MenuXOffset`. See [`MenuList`].
    pub list: Option<MenuList>,
    /// A page drawn the way the original's settings screens draw theirs: label
    /// left, step arrows, the value in a column of its own, a fading rule between
    /// rows. See [`MenuSettings`]. `None` for every title but Pulse's PSP pressings.
    pub settings: Option<MenuSettings>,
    /// A row's second line, drawn only under the selected row, for a title whose
    /// main menu screen was captured. See [`HelpText`]. `None` draws no subtitle
    /// rather than guessing at Pulse's numbers.
    pub help_text: Option<HelpText>,
    /// What the vertical `<Menu>`'s text rows stamp into the glow mask the
    /// frame-end bloom reads, for a title whose front end blooms
    /// ([`crate::Campaign::frontend_glow`]). See [`MenuGlow`]. `None` stamps nothing.
    pub glow: Option<MenuGlow>,
}

/// The glow-mask references a vertical `<Menu>`'s texts stamp (`Text_Draw`'s
/// `RealGlow` path; `docs/ui/campaign-screens.md`, "The front end's bloom, wired
/// from the widgets' own attributes").
///
/// Read out of EDRAM on Pulse PSP's `Main Menu` over six frames at different pulse
/// phases: six idle rows stamp `110` (14,526 pixels, flat at every phase), the help
/// line `128` (3,652 pixels), and the selected row a ladder `70 + 15 k` that follows
/// the row's colour pulse (2,674 pixels, 100 to 220 seen), the pulse of
/// [`MenuSkin::selected_pulse_period_secs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuGlow {
    /// An unselected row's reference: `110`, **measured**. Not `RealGlow * alpha /
    /// 255` of any attribute the `Main Menu` authors, so it is the `Menu` widget's
    /// own constant.
    pub row: u8,
    /// The help line's reference: `helptext0..6`'s authored `RealGlow="128"` at alpha `0xff`.
    pub help: u8,
}

/// A row's help text, as `MainMenu_Definition.xml`'s seven `helptext` widgets
/// place and colour it. **Layout and colour only**: the wording is this project's
/// own (`docs/architecture/menus.md#a-per-row-subtitle`).
///
/// Measured off Pulse (`docs/ui/menus-original.md`, Layout and Colours tables,
/// confidence 90): `helptext0` through `helptext6` sit at `y = 50, 78, 106, ...`,
/// each exactly 18 below its row's `y = 32, 60, 88, ...`, and only the selected
/// row's draws (the seven share one `x` and would overlap).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HelpText {
    /// How far below the selected row's `y` the subtitle sits, in
    /// [`MenuSkin::space`]. `18.0` on Pulse.
    pub offset_y: f32,
    /// The subtitle face's scale, as a multiple of [`MenuSkin::menu_scale`].
    ///
    /// **`1.0` on Pulse, confidence 90**, because the caller routes this widget to
    /// its own `Default`-role atlas, not the row face's
    /// (`oag_ui::menu::rows::draw_text_rows`, a second atlas `oag-game`'s menu
    /// stage loads beside its `menu`-role primary, `crates/game/src/boot/fonts.rs`'s
    /// `face_atlas_slot`). The row face and the help face are two widgets that
    /// author no `scale` relative to each other. A title whose menu stage has no
    /// second atlas for this role draws it through the row face, where `1.0` would
    /// be wrong: a title adding this field must add the routing too.
    pub scale: f32,
    /// The subtitle's ink colour. `0xFFFFFFFF` on Pulse: `helptext0`'s own
    /// `color="0xffffffff"`, authored rather than measured.
    pub color: Argb,
}

/// The look of the original's settings screens, for the pages of this build's
/// tree that stand where one does.
///
/// **What the disc authors is read, not carried**: the screen named by
/// [`Self::screen`] holds the rules (twelve `<Image>`s per screen, the left and
/// right halves of one fading line each), the label column, the value column's `x`
/// and the row pitch, and `oag_ui::menu::settings` reads them at boot. This holds
/// what the disc does not author, each field marked by where it came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuSettings {
    /// The file the screen is in: Pulse's `RaceBox_Definition.xml`. Not
    /// The file the screen is in: Pulse's `RaceBox_Definition.xml`. Not
    /// [`crate::FrontEnd::race_box`], which names `Selection_Definition.xml`.
    pub file: &'static str,
    /// The screen read, by name: Pulse's `Single Player`, the Racebox's settings page.
    pub screen: &'static str,
    /// The pages of this build's own tree drawn this way, **chosen, not
    /// measured**: the original has one such page under Racebox, and this build's
    /// `race` page stands in its place.
    pub pages: &'static [&'static str],
    /// The font role the rows draw in. The screen authors `font="small"`; `Small`
    /// and `Title` are the same `.fnt` on Pulse (`menus-original.md`, the role
    /// table), so the title's slot carries it.
    pub font: &'static str,
    /// The sheet the step arrows are cut from: `Data\FE\Images\pulse_assets.mip`.
    pub sheet: &'static str,
    /// The left and right step arrow in the sheet, `[u, v, w, h]`. **Authored**:
    /// `InGame_Definition.xml` draws the same two texels as `<Image>`s (`U="289"
    /// V="76"` and `U="314" V="76"`, 9x10).
    pub arrow: [[f32; 4]; 2],
    /// The lit variants of those two, `[u, v, w, h]`: the halo a focused row's
    /// arrows have. **Measured off the sheet's alpha**; the executable's `List`
    /// draw code carries them, the XML does not.
    pub arrow_glow: [[f32; 4]; 2],
    /// Where each arrow's top-left sits from the value column's `x` and the row's
    /// `y`, left then right. **Measured** off a PPSSPP capture of `Single Player`
    /// at 2x: the left arrow's left edge 23 units before the value, the right one's
    /// 13, both 5 below the row's text top.
    pub arrow_offset: [(f32, f32); 2],
    /// How far a lit arrow's halo reaches past the plain arrow's top-left, `(x, y)`:
    /// the halo is centred on the arrow. **Measured**, same capture.
    pub glow_inset: (f32, f32),
}

/// Where a title's settings rows are anchored, read off its screens.
///
/// **Authored.** HD's `Settings` screens do not use `MenuXOffset`: each nests its
/// `<List>`/`<Slider>` rows inside an `<Item OffsetX="160" OffsetY="170">` and
/// authors each row's `y` at `0, 50, 100, ...` (`additional_definition.xml`, all
/// four archives that carry it). The column's left edge, top and pitch are three
/// authored numbers distinct from [`MenuSkin::menu_x`] and
/// [`MenuSkin::first_row_y`], which a title whose rows are placed by the globals
/// keeps using.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuList {
    /// The `<Item>`'s `OffsetX`, in [`MenuSkin::space`].
    pub x: f32,
    /// The `<Item>`'s `OffsetY`, in [`MenuSkin::space`].
    pub y: f32,
    /// The step between two rows' authored `y`, in [`MenuSkin::space`].
    pub pitch: f32,
    /// The rows' own `scale`, as every `<List>` and `<Slider>` on those screens
    /// authors it (`scale="0.8"` on HD), multiplying [`MenuSkin::menu_scale`].
    pub text_scale: f32,
}

/// The `Block` a title draws behind a menu entry, as its executable draws it.
///
/// **Every number here is the executable's, not the disc's**, a third provenance
/// beside the authored and measured fields of [`MenuSkin`]. HD's GUI XML authors a
/// `<HorizMenu>`'s position and colour and nothing about the box behind each
/// entry; the box, its border, its translucent inside, and the selected entry
/// being wider and easing toward that width are `Block_Item.cpp`,
/// `HorizMenu_Item.cpp` and `List_Item.cpp` in `EBOOT.elf`
/// (`docs/ghidra/functions/ps3-hdfury-eu/menu-blocks.md`, every address).
///
/// What *is* the disc's is named, not transcribed: the textures are entry names
/// for the caller to decode, and the fill's alpha is a texel of one of them.
///
/// The box geometry (three-unit border, two-unit fill inset, ten-unit chamfer,
/// seventeen-unit landing, 40x18 corner pieces) is the drawing routine's and lives
/// with the drawing code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuBlocks {
    /// The nine-patch every block's border is cut from and whose swatch
    /// The nine-patch every block's border is cut from and whose swatch texels its
    /// fill samples: `Data\FE\Images\file2.gtf`, named by a pointer table the
    /// `Block` constructor reads (`0x00920a00`).
    pub frame_texture: &'static str,
    /// The strip's underline mark: `Data\FE\Images\cursor.gtf`, loaded by
    /// `HorizMenu_Construct`.
    pub cursor_texture: &'static str,
    /// The settings rows' step arrow: `Data\FE\Images\HD_options_arrow.gtf`,
    /// loaded by `List_Construct`.
    pub arrow_texture: &'static str,
    /// The `<HorizMenu>` widget's own numbers.
    pub strip: StripBlocks,
    /// The `<List>` widget's own numbers.
    pub list: ListBlocks,
    /// How much of the remaining distance a block's width closes per tick toward
    /// its target: `0.16667` in all three widgets (`HorizMenu_LayoutBlocks`,
    /// `VertMenu_LayoutBlocks`, `List_Update`). Per *frame* in the original, which
    /// runs its front end at 60 Hz, as this build's menu stage ticks.
    pub ease: f32,
}

/// What `HorizMenu_Item.cpp` gives a strip's blocks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripBlocks {
    /// An unselected entry's block width: the `ItemWidth` default, `298.0`, set by
    /// `HorizMenu_Construct`; the main menu authors none.
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
    /// `extra` is what the caller uses when this title's leading is unmeasured,
    /// passed in so a title package's silence never becomes another title's
    /// measurement. The scale multiplies both terms (a scaled row is a scaled gap),
    /// which is untested against the original: every menu measured is at scale 1.0.
    #[must_use]
    pub fn row_pitch(&self, line_height: f32, extra: f32) -> f32 {
        (line_height + self.row_extra_leading.unwrap_or(extra)) * self.menu_scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pulse's numbers as a fixture: this crate must not know any title's data, so
    /// the arithmetic check cannot reach for `oag-pulse`.
    const PULSE_SHAPED: MenuSkin = MenuSkin {
        space: (480.0, 272.0),
        menu_x: 50.0,
        menu_scale: 1.0,
        title_x: 50.0,
        title_y: 0.0,
        title_scale: 1.0,
        title_font: Some("Title"),
        body_font: Some("Default"),
        stats_font: None,
        default_face_is_row_face: false,
        first_row_y: Some(32.0),
        row_extra_leading: Some(6.0),
        menu_font: Some("menu"),
        text: Some(0xFF33_A6B9),
        title: Some(0xFF00_0000),
        background: None,
        selected: Some(0xFFFF_FFFF),
        selected_pulse_period_secs: Some(1.1),
        selector_origin: None,
        transition_secs: 0.5,
        // Pulse's discs author no `<HorizMenu>`.
        strip: None,
        blocks: None,
        list: None,
        settings: None,
        help_text: None,
        glow: None,
    };

    /// The four pitches measured off the original, reproduced by the rule.
    #[test]
    fn the_pitch_rule_reproduces_every_measured_menu() {
        // The 22-pixel `menu` face, on `Main Menu` and `Racebox`.
        assert!((PULSE_SHAPED.row_pitch(22.0, 0.0) - 28.0).abs() < f32::EPSILON);
        // The 17-pixel `small` face, on `Single Player` and `Cell Setup`.
        assert!((PULSE_SHAPED.row_pitch(17.0, 0.0) - 23.0).abs() < f32::EPSILON);
    }

    /// A scaled skin scales the leading with the text.
    #[test]
    fn scale_reaches_the_leading_too() {
        let scaled = MenuSkin {
            menu_scale: 2.0,
            ..PULSE_SHAPED
        };
        assert!((scaled.row_pitch(22.0, 0.0) - 56.0).abs() < f32::EPSILON);
    }

    /// A title that measured no leading takes the caller's, not another title's.
    #[test]
    fn an_unmeasured_leading_falls_back_to_the_callers() {
        let unmeasured = MenuSkin {
            row_extra_leading: None,
            ..PULSE_SHAPED
        };
        assert!((unmeasured.row_pitch(22.0, 2.0) - 24.0).abs() < f32::EPSILON);
    }
}
