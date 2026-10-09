//! What a menu is drawn *with*: one title's recovered skin, this build's own
//! layout figures, and the grid the two are reconciled in.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. The seam is
//! the one the module already had - [`Skin`]'s own docs open by saying that
//! nothing in [`super::draw_list`] reads [`oag_title::MenuSkin`] directly, and
//! that guarantee is easier to check when the whole of the reconciliation is one
//! file.
//!
//! Everything here is either the title's (read through [`Skin`]) or **ours** and
//! says so. Ours is written in the PSP's 480x272, because that is where it was
//! chosen and checked by eye, and [`Skin::new`] scales it into whatever grid the
//! source authors in.

/// The right-hand edge a row's value is anchored to, **written in 480x272** and
/// read through [`Skin::value_right`], which puts it in the grid being drawn in.
///
/// **Ours.** The original's menus have no value column at all - a Wipeout row
/// is a label and nothing else, and every setting that needs one lives on a
/// screen built for it. So this is picked rather than recovered, far enough
/// from [`MenuSkin::menu_x`] that a long label and a long value cannot collide.
///
/// This build's own figures stay in the PSP's units because that is where they
/// were chosen and where every one of them was checked by eye. A source that
/// authors elsewhere scales them; see [`Skin::new`].
const VALUE_RIGHT: f32 = 440.0;

/// The leading this build uses for a title that never measured its own,
/// **written in 480x272**; read through [`Skin::our_leading`].
///
/// Pulse measured 6; Pure has not been captured. Handing this to
/// [`oag_title::MenuSkin::row_pitch`] rather than letting the title table
/// default is what keeps one title's measurement out of another's menu - see
/// that method's own docs.
const OUR_LEADING: f32 = 6.0;

/// The gap between two entries of a horizontal strip, **written in 480x272**
/// and read through [`Skin::strip_gap`].
///
/// **Ours** in the sense that a `<HorizMenu>` states a position and a colour
/// and nothing about spacing at all - no `gap`, no second anchor to derive one
/// from. Measured in the same 2026-09-01 capture as the tab group below, at
/// the same ruler (`480 / 1278 = 0.3756`): four tab-to-tab gaps, taken fill-
/// right-edge to fill-left-edge below the chamfer (where the rectangle is
/// full width, so no per-row narrowing skews it), came back `7`, `8`, `8`,
/// `8` raw px across `CAMPAIGN`/`ONLINE`/`RECORDS` and the two entries past
/// them - call it `8`.
///
/// **That `8` is the visible gap, not [`STRIP_GAP`] itself.**
/// [`super::strip::draw`] steps `x` by `width + gap` from one label's own pen
/// position to the next, and a tab's fill starts [`TAB_LEFT_PAD`] left of its
/// own label - so the fill-to-fill gap is `STRIP_GAP - TAB_LEFT_PAD`, and
/// `STRIP_GAP` is the visible gap plus `TAB_LEFT_PAD`'s own raw `6`:
/// `8 + 6 = 14` raw, `14 * 0.3756 = 5.3` converted. An earlier version of this
/// doc read the visible gap (`9`, a coarser measurement than this one) as
/// `STRIP_GAP` directly and got `3.4` - the same missing step the tab group's
/// own ruler bug came from, a formula gap here rather than a units one, and
/// left unfixed at the time deliberately, to keep that pass to the geometry
/// bug alone. This value is now smaller than [`OUR_LEADING`], which an
/// earlier note here called deliberate on the reasoning that a strip needs
/// more separation than a column does with no edge between entries - that
/// reasoning does not survive a real measurement, and a capture read at this
/// group's own ruler outranks a readability guess. Confidence 55, same as the
/// tab group: one capture, four gaps agreeing to within a pixel.
const STRIP_GAP: f32 = 5.3;

/// A strip entry's own background tab, and the mark under a selected one -
/// both **ours**, and both approximated rather than authored.
///
/// **The fallback since 2026-09-14, not the tab.** The box behind an entry is
/// `Block_Item.cpp`'s, read out of `EBOOT.elf` and drawn by
/// [`super::block`] off the executable's own widths
/// ([`oag_title::MenuBlocks`]) and the disc's own nine-patch; this group is
/// what [`super::strip`] draws instead when a title has a strip and no
/// decoded block art, and [`TAB_TOP_PAD`] alone is still read on the block
/// path, for the label's vertical placement. Every number below came out
/// within a unit of the executable's, which is what says both are right.
///
/// Nothing on disc states any of this: a `<HorizMenu>` gives a position and a
/// colour and nothing about a tab shape at all - see [`Strip::color`]. A
/// 2026-09-01 RPCS3 capture (`data/reference/hd-menu-highlight-shift/00-campaign.png`,
/// 1278x718, HD style) is what these six numbers come from.
///
/// **The ruler is the capture's own resolution against HD's own grid**
/// (`oag_hd::frontend::MENU_SKIN.space`, confidence 95, `1920x1080`), not
/// [`STRIP_GAP`]: `480 / 1278 = 0.3756`
/// ours-units per capture pixel - the capture is a downscaled photo of HD's
/// own 1920-wide grid, and this build's own figures are written in the PSP's
/// 480-wide one, so a raw pixel crosses both scalings on its way in. **A first
/// version of this group used `24 / 9 = 2.667` instead** - `STRIP_GAP`'s own
/// invented value divided by the capture's tab-to-tab gap, on the reasoning
/// that the capture's gap stood in for `STRIP_GAP` since nothing else in the
/// same image could calibrate against. It cannot: `STRIP_GAP` is invented, not
/// measured, so dividing by it computes the ratio between two unrelated
/// numbers, not a scale. It produced a tab seven times too tall (`101` where
/// `14.3` renders right) and every other number in the group proportionately
/// wrong with it - self-consistent, since everything went through the same
/// wrong ruler, and wrong regardless, since a render is what caught it: the
/// oversized tab was tall enough to clip the screen title above it, in a
/// region the ruler mistake never touched. Confirmed by re-rendering at the
/// corrected ruler and matching the capture's own proportions again.
/// **Confidence 55** - one capture, one entry measured twice (`CAMPAIGN` and
/// `RACEBOX` agreed to the pixel).
///
/// Raw pixel measurements, for whoever revisits this: tab `188x38` (`RACEBOX`)
/// and `232x38` (`CAMPAIGN`), left padding before the label `6`, top padding
/// above it `5`, right padding **`~0`** (a tab's flat width is its label's own
/// measured width plus the left padding and nothing past it - the chamfer
/// below eats what would otherwise be a right pad). Underline `13x5`, `21`
/// below the label's own top (`8` below its cap-height baseline, cap height
/// `14`), starting flush with the label's own left edge.
///
/// **The chamfer is a 45-degree cut followed by a flat landing.** The tab's
/// top-right corner is cut, not rounded, `17` raw px of cut region against
/// the tab's own `188`-`232` width, left edge vertical throughout - and that
/// `17` divides: a `6`-px diagonal at exactly 45 degrees, then `11.7` px of
/// *flat top edge at the lower level* before the vertical right edge. A
/// pentagon, `---\___|`. See [`TAB_CHAMFER_CUT`] for the measurement and for
/// the two earlier readings it corrects.
///
/// [`crate::menu::strip::draw`] draws it as [`crate::frontend::Draw::Fill`]
/// for the tab below [`TAB_CHAMFER_HEIGHT`], and
/// [`crate::frontend::Draw::ChamferedFill`] for the band above it - the band
/// narrowed by [`TAB_CHAMFER_LANDING`] so its right edge is where the diagonal
/// lands, and chamfered by [`TAB_CHAMFER_CUT`] so its own top edge is where
/// the diagonal starts. The landing is then simply the gap between the band's
/// right edge and the tab's, needing nothing drawn at all.
const TAB_LEFT_PAD: f32 = 2.3;
/// See [`TAB_LEFT_PAD`].
const TAB_TOP_PAD: f32 = 1.9;
/// How tall a strip entry's tab stands.
///
/// **Re-derived 2026-09-05 at a better ruler**, from `14.3`, which came from
/// the 1278-wide capture the rest of [`TAB_LEFT_PAD`]'s group still does. The
/// quantity is unchanged and so is the method; only the ruler improved, the
/// same move that group's own note describes making after its `24 / 9` bug.
///
/// The new measurement is off a 3840x2160 framebuffer grab
/// (`data/reference/hd-menu-tab-corner/mainmenu-3840x2160.png`, gitignored)
/// calibrated on the FE frame's own two `line.gtf` rules, which are
/// disc-authored and sit in the same image - so the scale is solved from the
/// picture rather than assumed from its resolution. That gives
/// `capture = authored * 1.92 + (76.8, 43.2)`; see
/// `docs/formats/hd-frontend.md` for the derivation and its checks.
///
/// Coverage-weighted vertical edges of a tab: capture `285.13` and `404.17`,
/// `119.04` px, `= 62.0` authored units, `= 15.6` in this build's 480x272 grid.
///
/// **Measured on both a selected and an unselected tab, and they are equal to
/// four digits** (`119.04` px each). That matters because the selected tab is
/// 24% *wider* than its neighbours - so height had to be checked rather than
/// assumed not to vary with selection, and it does not.
///
/// Confidence 75, against the group's 55: the ruler is disc-authored marks in
/// the same frame rather than an assumed downscale, and the value agrees across
/// two tabs in two states. It stops short of higher because it is still one
/// capture of one screen, and because nothing authors a tab height at all - see
/// [`TAB_LEFT_PAD`] on what `<HorizMenu>` does and does not state.
const TAB_HEIGHT: f32 = 15.6;
/// See [`TAB_LEFT_PAD`].
const UNDERLINE_WIDTH: f32 = 4.9;
/// See [`TAB_LEFT_PAD`].
const UNDERLINE_HEIGHT: f32 = 1.9;
/// How far below the label's own `y` the underline mark starts. See
/// [`TAB_LEFT_PAD`] for the group; this one number needed a further, small,
/// eyeballed correction on top of the ruler's `7.9` (`21` raw px `* 0.3756`).
/// The ruler assumes the label's own `y` sits at the glyph's cap top, true in
/// the capture (its top padding was measured as tab-top-to-cap-top directly)
/// but not in this build's own render - **confirmed, not just inferred from
/// the mismatch**: a one-off print of `push_text`'s own `rect` for `R` in
/// `RACE` gave `y=125, h=32` against a measured cap top at row 134 and
/// baseline at row 156, so the glyph's cell reserves about 9px above the cap
/// that carries no ink, and 1px below the baseline. `Cell::height` comes
/// straight off each glyph's own `.fnt` metrics
/// ([`crate::font::Atlas::from_font`]), so this is the disc's own font
/// keeping every glyph's box tall enough for the face's ascenders and
/// descenders, not padding this build added - `R`, `A`, `C` and `E` all
/// report the same `32`, despite none of the four having either. At `7.9`
/// the mark sat inside the label's own ink instead of below it - visible
/// immediately on `--menu-page main --screenshot`, not a subtle miss. `9.0`
/// is checked by eye against that render: clear of the label, inside the
/// tab.
const UNDERLINE_OFFSET_Y: f32 = 9.0;
/// The diagonal's own horizontal extent - how far left of the landing the tab's
/// top edge starts.
///
/// **Measured 2026-09-02 on a `3840x2160` framebuffer capture, three times HD's
/// own output resolution**, and cross-checked at `1280x720` on a separate boot;
/// the two agree within a pixel. See
/// [hd-frontend.md](../../../../docs/formats/hd-frontend.md) for the row-by-row
/// numbers and
/// [rpcs3-capture.md](../../../../docs/reverse-engineering/rpcs3-capture.md)
/// for why a root-window grab could not have produced them.
///
/// **This replaces a single `TAB_CHAMFER_WIDTH` of `6.4`, which was not wrong
/// so much as undivided**: `17` raw px was the whole cut region, and the cut is
/// a `6`-px diagonal followed by an `11.7`-px *flat landing* at the lower
/// level. The tab's corner is a pentagon, `---\___|`. Reading all `17` as the
/// diagonal is what made the 2026-09-01 `Draw::ChamferedFill` attempt look
/// wrong next to the capture and get reverted; reading none of it as one is
/// what made the one-step band that shipped instead a right angle where the
/// original has a bevel. Both were right about what they saw.
const TAB_CHAMFER_CUT: f32 = 2.25;
/// The flat run between the diagonal's foot and the tab's own right edge.
/// See [`TAB_CHAMFER_CUT`]: `35` px at `3840` wide, `11.7` at `1280`,
/// `* 0.3756`.
const TAB_CHAMFER_LANDING: f32 = 4.4;
/// How tall the cut is, measured down from the tab's own top edge.
///
/// Equal to [`TAB_CHAMFER_CUT`] because **the cut is 45 degrees**: `18` across
/// in `19` down at `3840`, `6` in `6` at `1280`. The `2.6` this replaces came
/// from counting `7` rows on the `1280` capture, both antialiased edge rows
/// included; the `3840` one resolves the same edge to `18/3 = 6`.
const TAB_CHAMFER_HEIGHT: f32 = TAB_CHAMFER_CUT;

/// Where the rows start for a title whose own menu definitions are unread.
///
/// **Ours**, and derived rather than a constant: a title can state where its
/// *title* goes without stating where its rows go - Pure does exactly that,
/// `TitleYOffset` 20 with no main-menu definition to read a first row out of -
/// and a fixed 32 would then drop the first row almost onto the title. So the
/// rows start one title-line plus one leading below the title, which is the
/// same relationship Pulse's own authored 32 has to its title at 0.
fn our_first_row_y(skin: &Skin) -> f32 {
    let (_, title_y, title_scale) = skin.title_at();
    title_y + skin.line_height * title_scale + skin.our_leading()
}

/// What this build draws a menu with, once a title's skin and the font in use
/// have been put together.
///
/// The title supplies what its disc states; this fills the gaps with values
/// marked as ours, so a field a title never measured shows up here as *this
/// build's* choice rather than as the other title's number. Nothing in
/// [`draw_list`] reads [`oag_title::MenuSkin`] directly, which is what makes
/// that guarantee checkable in one place.
#[derive(Debug, Clone, Copy)]
pub struct Skin {
    skin: &'static oag_title::MenuSkin,
    /// The grid the rows are drawn in: the source's own, not the PSP's.
    space: oag_display::space::Space,
    /// What multiplies a number written in this build's own 480x272 units to
    /// put it in [`Self::space`]. Exactly `(1.0, 1.0)` on a PSP source, which is
    /// why every layout figure pinned by this module's tests is unchanged there.
    from_ours: (f32, f32),
    /// The same for a number written in the *title's* grid - see
    /// [`oag_title::MenuSkin::space`], which is not always [`Self::space`].
    from_theirs: (f32, f32),
    line_height: f32,
    /// Where the rows' face keeps its ink, for a pointer's row bands on the
    /// screens that step by [`Self::line_height`]. See
    /// [`crate::pointer::RowInk`]; `None` until [`Self::set_row_ink`].
    row_ink: Option<crate::pointer::RowInk>,
    /// A free-running clock for [`Self::selected`]'s pulse, advanced by
    /// [`Self::tick_pulse`]. Never reset: the capture that measured Pulse's
    /// period never moved the cursor, so there is no evidence the original
    /// resets phase on a selection change, and a clock that ticks
    /// unconditionally is the null hypothesis until one turns up. Zero on a
    /// title with no [`oag_title::MenuSkin::selected_pulse_period_secs`],
    /// where nothing ever reads it.
    pulse_elapsed: f32,
}

impl Skin {
    /// Pairs a title's skin with the grid it authors in and the line height of
    /// the face the rows will actually be drawn in.
    ///
    /// The line height is passed in rather than looked up because the atlas is
    /// the renderer's, and this module owns layout: the same split
    /// [`draw_list`]'s `bindings` argument exists for.
    ///
    /// # The space is why this takes three arguments and not two
    ///
    /// **A menu skin's numbers are in the title's own front-end grid, and this
    /// module's own figures are in the PSP's 480x272.** For both PSP titles
    /// those are the same grid and the distinction is invisible; Wipeout HD
    /// authors `FEGlobals` at **1920x1080**, and drawing its `MenuXOffset` of
    /// 800 in a 480-wide grid put the whole label column 320 pixels off the
    /// right-hand edge. What that looks like is not a misplaced menu - it is a
    /// menu of values with no labels at all, which reads as a string-table
    /// failure rather than a coordinate one.
    ///
    /// `oag_hd::frontend::MENU_SKIN` predicted this in its own doc comment and
    /// left it for "when something draws these". This is that.
    ///
    /// # The rows are drawn in the *source's* grid, not converted into ours
    ///
    /// Both directions put the labels back on screen and only one of them keeps
    /// the picture. A 1080-line face squeezed into a 272-line grid is drawn at
    /// quarter scale from an atlas rasterised for 1080, which is soft text for
    /// no reason; and the grid decides the *display aspect* the whole menu is
    /// letterboxed to, so borrowing the PSP's would show a 16:9 front end at
    /// 480/272 = 1.765. So this build's own figures are the ones that move -
    /// [`VALUE_RIGHT`], [`OUR_LEADING`], [`MESSAGE_GAP`] and the screen height
    /// are written in 480x272 and scaled up by [`Self::from_ours`] - and the
    /// disc's numbers are used exactly as the disc writes them.
    ///
    /// # Two ratios, because "ours" and "the title's" are different questions
    ///
    /// [`oag_title::MenuSkin::space`] is the grid a table was *read in*, and it
    /// is not always the grid the source draws in: Pulse's table came off the
    /// PSP `Skin.xml` and its PS2 pressing places widgets in 640x448. Scaling
    /// only this build's own figures left that menu internally inconsistent -
    /// the value column moved by 640/480 while the label column, authored 50,
    /// stayed where a 480-wide grid put it, and the rows bunched toward the top
    /// because `first_row_y` of 32 is a tenth of 272 and a fourteenth of 448.
    ///
    /// So both sides convert, each from its own grid, and a PS2 menu comes out
    /// in the same proportions the PSP one always had. **On the PS2 that
    /// scaling is an approximation and says so**: `oag_display::space::Space`
    /// records that 30 of the 43 coordinates the two `Skin.xml` files share land
    /// within a pixel of the ratio and 13 do not, so the disc's own PS2
    /// `FEGlobals` would settle it and nothing here has read them.
    ///
    /// Both ratios are exactly `1.0` on a PSP source, so nothing about Pulse's
    /// or Pure's layout there moves by a float ulp; `menu/tests/drawing.rs` pins
    /// the numbers that would show it if it did.
    ///
    /// The line height is **not** converted by either: it comes off the face
    /// actually loaded from this source, so it is already in the drawing grid.
    ///
    /// **The menu tree is still ours** - only where it is drawn follows the
    /// disc. See `docs/architecture/menus.md`.
    #[must_use]
    pub fn new(
        skin: &'static oag_title::MenuSkin,
        space: oag_display::space::Space,
        line_height: f32,
    ) -> Self {
        let into = |from: (f32, f32)| (space.size.0 / from.0, space.size.1 / from.1);
        Self {
            skin,
            space,
            from_ours: into(oag_display::space::SCREEN),
            from_theirs: into(skin.space),
            line_height,
            row_ink: None,
            pulse_elapsed: 0.0,
        }
    }

    /// Records where the face [`Self::line_height`] came off keeps its ink,
    /// measured by the caller that has the atlas.
    pub fn set_row_ink(&mut self, ink: Option<crate::pointer::RowInk>) {
        self.row_ink = ink;
    }

    /// The top edge of a pointer band one [`Self::line_height`] tall for a
    /// line drawn with its pen at `y` at `scale`: centred on the face's
    /// capitals when [`Self::set_row_ink`] measured them, the pen otherwise.
    #[must_use]
    pub fn row_band_top(&self, y: f32, scale: f32) -> f32 {
        self.row_ink
            .map_or(y, |ink| ink.band_top(y, scale, self.line_height * scale))
    }

    /// Advances [`Self::selected`]'s pulse clock by one tick's worth of time.
    ///
    /// Called from `MenuStage::tick` alongside the marquee's own clock, for
    /// the same reason `anim.rs`'s module doc gives: `dt` is the stage's fixed
    /// step, never the wall clock, so a captured frame does not depend on how
    /// fast the machine that captured it happened to be.
    pub fn tick_pulse(&mut self, dt: f32) {
        self.pulse_elapsed += dt.max(0.0);
    }

    /// The grid these numbers are in, for whoever has to set a `screen` uniform
    /// from it.
    #[must_use]
    pub fn space(&self) -> oag_display::space::Space {
        self.space
    }

    /// This title's horizontal menu strip, in the grid being drawn in.
    ///
    /// `None` for a title that authors no `<HorizMenu>`, which both PSP titles
    /// measurably do not - see [`oag_title::MenuStrip`]. There is no fallback
    /// here and there must not be one: a strip this build invented for a title
    /// whose disc draws a column would be the wrong picture, where a missing
    /// figure inside a strip is only ever a spacing this build chose.
    #[must_use]
    pub fn strip(&self) -> Option<Strip> {
        self.skin.strip.map(|strip| Strip {
            x: strip.x * self.from_theirs.0,
            y: strip.y * self.from_theirs.1,
            // The widget's own colour - see `Strip::color`'s own doc for what
            // this is not, corrected 2026-09-01 by a capture.
            color: argb(strip.color),
        })
    }

    /// The box this title's executable draws behind an entry, with its
    /// widths still in the title's own units - see
    /// [`oag_title::MenuBlocks`]. Paired with [`Self::theirs_scale`], which
    /// puts them in the grid being drawn in; the block drawer takes both
    /// rather than a pre-scaled copy, because the nine-patch's own geometry
    /// scales the same way and lives with the drawer.
    ///
    /// `None` for a title with no blocks. **This is what decides which strip
    /// and row idiom draws**: with it, the tab is the executable's block and
    /// the measured `TAB_*` figures below are not read at all; without it,
    /// they are.
    #[must_use]
    pub fn blocks(&self) -> Option<oag_title::MenuBlocks> {
        self.skin.blocks
    }

    /// What multiplies a number written in the title's own grid into the
    /// grid being drawn in. `(1.0, 1.0)` on every title today - each authors
    /// in the grid its platform draws in - and kept as a value rather than
    /// assumed, for the reason [`Self::new`]'s own doc gives.
    #[must_use]
    pub fn theirs_scale(&self) -> (f32, f32) {
        self.from_theirs
    }

    /// Where this title anchors its settings rows, in the grid being drawn
    /// in, when its screens author that separately from `MenuXOffset` - see
    /// [`oag_title::MenuList`]. `None` keeps [`Self::menu_x`] and
    /// [`Self::first_row_y`].
    #[must_use]
    pub fn list(&self) -> Option<List> {
        self.skin.list.map(|list| List {
            x: list.x * self.from_theirs.0,
            y: list.y * self.from_theirs.1,
            pitch: list.pitch * self.from_theirs.1,
            text_scale: list.text_scale,
        })
    }

    /// A row's subtitle geometry, in the grid being drawn in, for a title
    /// whose main-menu screen was captured for it - see
    /// [`oag_title::MenuSkin::help_text`].
    ///
    /// `None` means draw no subtitle at all. Never a fallback to Pulse's own
    /// numbers: the same rule [`Self::selected_pulse_period_secs`] already
    /// states for its own axis, and for the same reason - nothing has
    /// measured where a *different* title's help text would sit.
    #[must_use]
    pub fn help_text(&self) -> Option<Subtitle> {
        self.skin.help_text.map(|help_text| Subtitle {
            offset_y: help_text.offset_y * self.from_theirs.1,
            scale: self.row_scale() * help_text.scale,
            color: argb(help_text.color),
        })
    }

    /// The gap between two strip entries, in the grid being drawn in. Ours; see
    /// [`STRIP_GAP`].
    #[must_use]
    pub(super) fn strip_gap(&self) -> f32 {
        STRIP_GAP * self.from_ours.0
    }

    /// A strip entry's own tab: how far its fill extends left of the label and
    /// above it, and how tall it stands. Ours; see [`TAB_LEFT_PAD`].
    #[must_use]
    pub(super) fn tab_pad(&self) -> (f32, f32) {
        (
            TAB_LEFT_PAD * self.from_ours.0,
            TAB_TOP_PAD * self.from_ours.1,
        )
    }

    /// A strip entry's own tab height. Ours; see [`TAB_LEFT_PAD`].
    #[must_use]
    pub(super) fn tab_height(&self) -> f32 {
        TAB_HEIGHT * self.from_ours.1
    }

    /// Every entry's cut corner: the diagonal's horizontal extent, the flat
    /// run between its foot and the tab's right edge, and the cut's height.
    /// Ours; see [`TAB_LEFT_PAD`] and [`TAB_CHAMFER_CUT`].
    #[must_use]
    pub(super) fn tab_chamfer(&self) -> (f32, f32, f32) {
        (
            TAB_CHAMFER_CUT * self.from_ours.0,
            TAB_CHAMFER_LANDING * self.from_ours.0,
            TAB_CHAMFER_HEIGHT * self.from_ours.1,
        )
    }

    /// The selected entry's underline mark: its own size, and how far below
    /// the label's `y` it starts. Ours; see [`TAB_LEFT_PAD`].
    #[must_use]
    pub(super) fn underline(&self) -> (f32, f32, f32) {
        (
            UNDERLINE_WIDTH * self.from_ours.0,
            UNDERLINE_HEIGHT * self.from_ours.1,
            UNDERLINE_OFFSET_Y * self.from_ours.1,
        )
    }

    /// The right-hand edge a row's value is anchored to. Ours; see
    /// [`VALUE_RIGHT`].
    #[must_use]
    pub fn value_right(&self) -> f32 {
        VALUE_RIGHT * self.from_ours.0
    }

    /// Room under the last row for the noted-row message. Ours; see
    /// [`MESSAGE_GAP`].
    ///
    /// **`pub`, not `pub(super)`**: `oag_ui_screens::prompt::axis_preview_draw`
    /// reuses this same gap to sit above [`super::Frame::content_bottom`]
    /// rather than inventing a second margin for the same purpose.
    #[must_use]
    pub fn message_gap(&self) -> f32 {
        MESSAGE_GAP * self.from_ours.1
    }

    /// The reserved note/preview line's own scale, as a multiple of a row's.
    /// Ours; see [`MESSAGE_SCALE`].
    #[must_use]
    pub fn message_scale(&self) -> f32 {
        MESSAGE_SCALE
    }

    /// This build's own leading, in the grid being drawn in. See
    /// [`OUR_LEADING`].
    fn our_leading(&self) -> f32 {
        OUR_LEADING * self.from_ours.1
    }

    /// Left edge of the rows. `FEGlobals->MenuXOffset`.
    #[must_use]
    pub fn menu_x(&self) -> f32 {
        self.skin.menu_x * self.from_theirs.0
    }

    /// Top of the first row's line box.
    #[must_use]
    pub fn first_row_y(&self) -> f32 {
        self.skin
            .first_row_y
            .map_or_else(|| our_first_row_y(self), |y| y * self.from_theirs.1)
    }

    /// Distance between two rows. [`oag_title::MenuSkin::row_pitch`]'s rule,
    /// with each term put in the grid being drawn in first.
    ///
    /// The line height is the face's own and needs nothing. The leading needs a
    /// different ratio depending on whose it is - a title's measured one is in
    /// that title's grid and [`OUR_LEADING`] is in ours - which is why this
    /// resolves the choice here instead of handing both to the same argument.
    /// Only Pulse measures one, so the distinction is untested by anything but
    /// its own PS2 pressing; it is written this way because the alternative is a
    /// silent 6-against-9.9 disagreement on that disc.
    #[must_use]
    pub fn row_pitch(&self) -> f32 {
        let leading = self
            .skin
            .row_extra_leading
            .map_or_else(|| self.our_leading(), |theirs| theirs * self.from_theirs.1);
        (self.line_height + leading) * self.skin.menu_scale
    }

    /// The menu face's own line height, in the grid being drawn in.
    ///
    /// For a widget that states its own scale and no pitch - a `<Menu>` on a
    /// selection screen, say - and so has to step by a line of its own font
    /// rather than by [`Self::row_pitch`], which is the *menu page's* step and
    /// carries that page's leading and `MenuScale`.
    #[must_use]
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// The scale a row's text is drawn at. `FEGlobals->MenuScale`.
    #[must_use]
    pub fn row_scale(&self) -> f32 {
        self.skin.menu_scale
    }

    /// An unselected row. `FEGlobals->TextColor`, or ours where undeclared.
    #[must_use]
    pub fn normal(&self) -> [f32; 4] {
        self.skin.text.map_or(OUR_NORMAL, argb)
    }

    /// The selected row, brightened toward white rather than given a bar -
    /// pulsing between [`Self::normal`] and the title's own peak when
    /// [`oag_title::MenuSkin::selected_pulse_period_secs`] measured one, flat
    /// at the peak otherwise (today, every title but Pulse).
    ///
    /// The oscillation's *shape* between the two measured endpoints is
    /// **ours** - a raised cosine, the simplest curve that starts and ends
    /// flat rather than with a visible corner - the same discipline
    /// `anim::Tween::eased` marks its own invented curve with: the capture
    /// found eleven discrete brightness levels, unevenly spaced enough to
    /// rule out a linear ramp, but not enough samples to name the real one.
    /// See `docs/ui/menus-original.md`.
    #[must_use]
    pub fn selected(&self) -> [f32; 4] {
        selected_ink(self.skin, self.normal(), self.pulse_elapsed)
    }

    /// A full-screen fill for a frame whose own screen carries neither a
    /// `<ScreenClear>` nor a fill of its own. `None` for every title but
    /// Pure - see [`oag_title::MenuSkin::background`]'s own doc for why: a
    /// title reaching this is a title whose frame authored a background
    /// widget and then left it with no way to resolve one.
    #[must_use]
    pub fn background(&self) -> Option<crate::frontend::Draw> {
        self.skin
            .background
            .map(|color| crate::frontend::Draw::Fill {
                rect: [0.0, 0.0, self.space.size.0, self.space.size.1],
                color: argb(color),
            })
    }

    /// How long a page change takes. `transition=`.
    #[must_use]
    pub fn transition_secs(&self) -> f32 {
        self.skin.transition_secs
    }

    /// How long one cycle of [`Self::selected`]'s pulse takes, where this
    /// title has one measured. See
    /// [`oag_title::MenuSkin::selected_pulse_period_secs`].
    #[must_use]
    pub fn selected_pulse_period_secs(&self) -> Option<f32> {
        self.skin.selected_pulse_period_secs
    }

    /// Where the screen title sits, and how big.
    ///
    /// The scale is dimensionless and stays as authored: it multiplies a face
    /// that already came off this source.
    #[must_use]
    pub fn title_at(&self) -> (f32, f32, f32) {
        (
            self.skin.title_x * self.from_theirs.0,
            self.skin.title_y * self.from_theirs.1,
            self.skin.title_scale,
        )
    }

    /// The font role the screen title is drawn in, or `None` to draw it in
    /// whatever face the rest of the frame is already bound to - see
    /// [`oag_title::MenuSkin::title_font`].
    #[must_use]
    pub fn title_font(&self) -> Option<&'static str> {
        self.skin.title_font
    }

    /// The screen title's colour.
    ///
    /// **The disc's own declared `TitleColor` first.** A title whose skin
    /// names one is trusted: it authored that colour to sit on its own chrome,
    /// and this build's job is to draw the chrome, not to second-guess the
    /// colour once it does. Both titles that declare a frame today also
    /// declare `TitleColor` - HD's `0xFF646464` and Pulse's `0xFF000000` - so
    /// in practice this is the only branch either one reaches.
    ///
    /// **`frame_ink` second** - [`super::Frame::ink`], the colour the frame's
    /// own marks actually draw in - for a title that draws a frame but leaves
    /// `TitleColor` undeclared. No title measured so far is that case: HD's
    /// `TitleColor` and its `HD_Grey`-tinted marks agree today (worth
    /// stating rather than assuming, since they are two separate globals
    /// that could diverge - see `oag_hd::frontend::MENU_SKIN`'s own field
    /// doc), and Pulse's marks carry no tint of their own at all, so a
    /// title reaching this branch would be a first.
    ///
    /// **This build's own substitute last**, for a title with neither: no
    /// declared colour, and no frame. Pure is that title, and so was Pulse
    /// before its own top bar was read - it authored `TitleColor` black but
    /// drew on no bar, so this build's own contrast (`OUR_SELECTED`) stood in
    /// rather than painting black on nothing. Now that
    /// `oag_pulse::FRONT_END::menu_frame` names its own frame, the
    /// substitution's premise - nothing draws the bar - is gone, and the
    /// declared black wins as it would for any other title.
    #[must_use]
    pub fn title_color(&self, frame_ink: Option<[f32; 4]>) -> [f32; 4] {
        self.skin
            .title
            .map_or_else(|| frame_ink.unwrap_or(OUR_SELECTED), argb)
    }
}

/// A title's settings-row anchor, converted into the grid the menus are
/// drawn in. See [`Skin::list`] and [`oag_title::MenuList`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct List {
    /// Left edge of the rows' blocks. Authored, the `<Item>`'s `OffsetX`.
    pub x: f32,
    /// Top of the first row's block. Authored, the `<Item>`'s `OffsetY`.
    pub y: f32,
    /// From one row's top to the next. Authored, the rows' own `y` step.
    pub pitch: f32,
    /// The rows' own `scale`, on top of the skin's. Authored.
    pub text_scale: f32,
}

/// A row's subtitle geometry, converted into the grid the menus are drawn in.
/// See [`Skin::help_text`] and [`oag_title::HelpText`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Subtitle {
    /// How far below the selected row's own `y` this sits. Authored.
    pub offset_y: f32,
    /// The subtitle face's scale, [`Skin::row_scale`] already folded in.
    pub scale: f32,
    /// The subtitle's ink colour. Authored.
    pub color: [f32; 4],
}

/// A title's `<HorizMenu>`, converted into the grid the menus are drawn in.
///
/// The same relationship [`Skin`] has to [`oag_title::MenuSkin`], one field
/// deep: what the disc authors is [`oag_title::MenuStrip`], and this is that
/// after [`Skin::new`]'s conversion, so nothing downstream has to know which
/// grid the numbers were written in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strip {
    /// Left edge of the first entry. Authored.
    pub x: f32,
    /// Top of the entries' line box. Authored.
    pub y: f32,
    /// The widget's own colour. Authored, and **not what an entry's text is
    /// drawn in.**
    ///
    /// It was read that way until a 2026-09-01 capture: every entry, selected
    /// or not, is the same white [`Skin::normal`] (`FEGlobals->TextColor`),
    /// and what this widget's own colour is *for* is still open - it names
    /// neither the text nor either style's tab fill (`HD_Grey`/`HD_Blue`, see
    /// [`super::Frame::ink`] and [`super::Frame::tab_selected`]). Kept because
    /// it is still what the widget authors, not because anything still reads
    /// it for text or fill.
    pub color: [f32; 4],
}

/// How many rows are on screen at once, given the pitch in use, `frame`'s own
/// chrome, and whether the page about to show needs a line reserved under
/// the rows.
///
/// Derived rather than picked: rows start at the skin's first row and are one
/// pitch apart, and the block must clear [`super::Frame::content_bottom`] -
/// **not the screen's own edge**, which is what this used to clear and is not
/// where a title's own chrome actually starts. Pulse's footer sits at `236`
/// of a 272-pixel screen; drawing rows all the way to `272` is what let the
/// AI PILOTS page's own axis preview land on top of it. See
/// `docs/architecture/menus.md`'s "AI PILOTS: the axis preview" section for
/// the capture that found the gap between the two, and [`Skin::message_gap`]
/// for why `oag_ui_screens::prompt::axis_preview_draw` reads the same figures this
/// reserves room for rather than a tuned offset of its own.
///
/// `reserve_note` adds one more line, at [`Skin::message_scale`], plus
/// [`Skin::message_gap`], under the last row - for a page whose rows can
/// carry a warning or restart mark, or (today, the one caller that sets
/// this) draws `oag_ui_screens::prompt::axis_preview_draw`'s own line underneath
/// them. **This has to be a fact about the page, not about a row's live
/// value**: reserving room only once a note happens to be showing would
/// resize the grid on the very keystroke that makes one appear, which is
/// the jump this reservation exists to avoid - see [`super::Menu::warning`]
/// and `crate::pilots::page_reserves_axis_preview`.
///
/// At Pulse's measured 28-pixel pitch and `reserve_note` false, that is
/// seven rows, which is exactly what the original's own main menu shows -
/// `236 - 32 = 204`, `204 / 28` floors to seven, so nothing here moved for
/// a page with nothing reserved under it. AI PILOTS is the one page today
/// that sets `reserve_note`, and six is what leaves its own preview line
/// room to sit above the footer instead of on it.
///
/// **A title with no frame at all keeps the old budget exactly**, note
/// reserved unconditionally against the screen's own edge - the same shape
/// this whole function had before [`super::Frame`] was threaded through.
/// `reserve_note` only *adds* to that when there is real chrome to weigh it
/// against; there is nothing for it to subtract from otherwise, and every
/// test fixture and [`super::DEFAULT_VISIBLE_ROWS`] were both written against that
/// number, seven for Pulse's own skin.
#[must_use]
pub fn visible_rows(skin: &Skin, frame: &super::Frame, reserve_note: bool) -> usize {
    // A title whose screens anchor and space the rows themselves - HD's
    // `<aList>`, drawn by `rows::draw_list_rows` - is budgeted by that grid,
    // not by the font's own pitch, which on HD is seven units longer. The
    // condition is the drawing's own: a frame whose nine-patch did not
    // decode falls back to text rows at the font's pitch, and is budgeted
    // by it.
    let list = if frame.blocks.is_some() {
        skin.list()
    } else {
        None
    };
    let (first_y, pitch) = list.map_or((skin.first_row_y(), skin.row_pitch()), |list| {
        (list.y, list.pitch)
    });
    let pitch = pitch.max(1.0);
    let note_line = skin.line_height * MESSAGE_SCALE + skin.message_gap();
    let room = match frame.content_bottom(skin.space()) {
        Some(bottom) => {
            let reserve = if reserve_note { note_line } else { 0.0 };
            bottom - first_y - reserve
        }
        None => skin.space().size.1 - first_y - note_line,
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 1..=32 on the next line"
    )]
    let rows = (room / pitch).floor().max(1.0) as usize;
    rows.clamp(1, 32)
}

/// Room left under the last row for the noted-row message, **written in
/// 480x272**; read through [`Skin::message_gap`].
const MESSAGE_GAP: f32 = 6.0;

/// The reserved note/preview line's own scale, as a multiple of a row's -
/// read through [`Skin::message_scale`] and shared by [`super::rows::draw`]'s
/// own warning/restart message and `oag_ui_screens::prompt::axis_preview_draw`,
/// since [`visible_rows`]'s reserved room is sized for exactly one line at
/// this scale: a caller drawing at a different one would either waste the
/// reservation or overflow it, the latter being how the axis preview ended
/// up on Pulse's own footer in the first place.
const MESSAGE_SCALE: f32 = 0.8;

/// A packed `0xAARRGGBB` as the renderer's straight-alpha RGBA.
pub(super) fn argb(value: oag_title::menu::Argb) -> [f32; 4] {
    let byte = |shift: u32| ((value >> shift) & 0xFF) as f32 / 255.0;
    [byte(16), byte(8), byte(0), byte(24)]
}

/// `from` to `to`, channel by channel, at `t` in `0..=1`. [`Skin::selected`]'s
/// own lerp: `oag-ui` *is* one of the crates `just check-determinism` scans,
/// but this module's `sin`/`cos` reach a pixel rather than a hash (see that
/// script's own `ALLOWED` entry for this file), and there is still no reason
/// for a menu colour to reassociate float arithmetic it does not need to.
/// A title's selected-row ink `elapsed` seconds into its pulse, given the
/// unselected ink `normal`: the one rule [`Skin::selected`] (the menus) and the
/// language picker's rows both draw a selection by, so the two cannot differ.
///
/// The peak is [`oag_title::MenuSkin::selected`], or [`OUR_SELECTED`] where none
/// was measured; with a measured pulse period the ink rides a raised cosine
/// between `normal` and the peak (the shape is ours, see [`Skin::selected`]),
/// flat at the peak otherwise.
pub(crate) fn selected_ink(
    skin: &oag_title::MenuSkin,
    normal: [f32; 4],
    elapsed: f32,
) -> [f32; 4] {
    let peak = skin.selected.map_or(OUR_SELECTED, argb);
    let Some(period) = skin.selected_pulse_period_secs.filter(|p| *p > 0.0) else {
        return peak;
    };
    let phase = (elapsed / period).fract();
    let t = 0.5 - 0.5 * (phase * std::f32::consts::TAU).cos();
    lerp_color(normal, peak, t)
}

fn lerp_color(from: [f32; 4], to: [f32; 4], t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    let mut out = [0.0; 4];
    for i in 0..4 {
        out[i] = from[i] + (to[i] - from[i]) * t;
    }
    out
}
/// An unselected row, for a title that declares no `TextColor`.
///
/// **Ours**, and only ever reached by a title whose own disc is silent. Pulse's
/// `0xFF33A6B9` and Pure's measured `0xFF88D6E8` both come off their discs.
const OUR_NORMAL: [f32; 4] = [0.72, 0.78, 0.84, 1.0];

/// The selected row, for a title with no capture of one.
///
/// **Ours**, but shaped by what the original does: the capture shows selection
/// as a brightening toward white rather than as a bar behind the row, so this
/// is white rather than a fourth hue. Pulse supplies its own measured value.
const OUR_SELECTED: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

#[cfg(test)]
mod tests {
    use super::*;

    /// Pulse's own measured shape, as a fixture: `TextColor` to white over a
    /// 1.1 s period. Not a dependency on `oag-pulse` - this module already
    /// promises nothing in [`draw_list`] reads [`oag_title::MenuSkin`]
    /// directly, and a fixture here keeps that true of its own tests too.
    const PULSING: oag_title::MenuSkin = oag_title::MenuSkin {
        space: (480.0, 272.0),
        menu_x: 50.0,
        menu_scale: 1.0,
        title_x: 50.0,
        title_y: 0.0,
        title_scale: 1.0,
        title_font: None,
        body_font: None,
        first_row_y: Some(32.0),
        row_extra_leading: Some(6.0),
        menu_font: Some("menu"),
        text: Some(0xFF33_A6B9),
        title: Some(0xFF00_0000),
        background: None,
        selected: Some(0xFFFF_FFFF),
        selected_pulse_period_secs: Some(1.1),
        transition_secs: 0.5,
        strip: None,
        blocks: None,
        list: None,
        settings: None,
        help_text: None,
    };

    fn skin() -> Skin {
        Skin::new(&PULSING, oag_display::space::Space::PSP, 22.0)
    }

    /// The trough: a fresh clock has not moved, so the pulse starts exactly
    /// at its own resting colour - the phase the capture's own troughs sat
    /// at, not an arbitrary zero.
    #[test]
    fn a_fresh_clock_starts_at_the_trough() {
        assert_eq!(skin().selected(), skin().normal());
    }

    /// Half a period on, the pulse is at its measured peak.
    #[test]
    fn half_a_period_on_it_is_at_the_peak() {
        let mut skin = skin();
        skin.tick_pulse(1.1 / 2.0);
        let peak = skin.selected();
        let white = argb(0xFFFF_FFFF);
        for channel in 0..4 {
            assert!(
                (peak[channel] - white[channel]).abs() < 1e-5,
                "{peak:?} should be white at the measured half-period"
            );
        }
    }

    /// A whole period on, it is back at the trough - the point of measuring a
    /// period at all being that the animation repeats rather than drifts.
    #[test]
    fn a_whole_period_returns_to_the_trough() {
        let mut skin = skin();
        skin.tick_pulse(1.1);
        assert_eq!(skin.selected(), skin.normal());
    }

    /// A title with no measured pulse - every title but Pulse, today - stays
    /// flat at its own peak no matter how much the clock advances. This is
    /// what keeps Pure's own, differently-directioned `selected` colour from
    /// silently inheriting Pulse's oscillation.
    #[test]
    fn a_title_with_no_measured_period_never_pulses() {
        const FLAT: oag_title::MenuSkin = oag_title::MenuSkin {
            selected_pulse_period_secs: None,
            ..PULSING
        };
        let mut skin = Skin::new(&FLAT, oag_display::space::Space::PSP, 22.0);
        let at_rest = skin.selected();
        skin.tick_pulse(10.0);
        assert_eq!(skin.selected(), at_rest);
        assert_eq!(skin.selected(), argb(FLAT.selected.unwrap()));
    }
}
