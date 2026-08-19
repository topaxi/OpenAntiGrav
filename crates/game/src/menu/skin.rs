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
/// **Ours, and it has to be**: a `<HorizMenu>` states a position and a colour
/// and nothing about spacing - no `gap`, no second anchor to derive one from,
/// and no capture of the original's own strip exists to measure one off. So an
/// entry is advanced past by its own measured width plus this, which is the one
/// rule that cannot put two labels on top of each other whatever they say.
///
/// Deliberately larger than [`OUR_LEADING`]: entries sit side by side with no
/// column edge to separate them, so the gap is the only thing that says where
/// one ends and the next begins.
const STRIP_GAP: f32 = 24.0;

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
    space: crate::frontend::Space,
    /// What multiplies a number written in this build's own 480x272 units to
    /// put it in [`Self::space`]. Exactly `(1.0, 1.0)` on a PSP source, which is
    /// why every layout figure pinned by this module's tests is unchanged there.
    from_ours: (f32, f32),
    /// The same for a number written in the *title's* grid - see
    /// [`oag_title::MenuSkin::space`], which is not always [`Self::space`].
    from_theirs: (f32, f32),
    line_height: f32,
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
    /// scaling is an approximation and says so**: `crate::frontend::Space`
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
        space: crate::frontend::Space,
        line_height: f32,
    ) -> Self {
        let into = |from: (f32, f32)| (space.size.0 / from.0, space.size.1 / from.1);
        Self {
            skin,
            space,
            from_ours: into(crate::frontend::SCREEN),
            from_theirs: into(skin.space),
            line_height,
        }
    }

    /// The grid these numbers are in, for whoever has to set a `screen` uniform
    /// from it.
    #[must_use]
    pub fn space(&self) -> crate::frontend::Space {
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
            // The widget's own colour, not `TextColor`: HD authors
            // `0xff705070` here against a white `TextColor`.
            color: argb(strip.color),
        })
    }

    /// The gap between two strip entries, in the grid being drawn in. Ours; see
    /// [`STRIP_GAP`].
    #[must_use]
    pub(super) fn strip_gap(&self) -> f32 {
        STRIP_GAP * self.from_ours.0
    }

    /// The right-hand edge a row's value is anchored to. Ours; see
    /// [`VALUE_RIGHT`].
    #[must_use]
    pub fn value_right(&self) -> f32 {
        VALUE_RIGHT * self.from_ours.0
    }

    /// Room under the last row for the noted-row message. Ours; see
    /// [`MESSAGE_GAP`].
    #[must_use]
    pub(super) fn message_gap(&self) -> f32 {
        MESSAGE_GAP * self.from_ours.1
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

    /// The selected row, brightened toward white rather than given a bar.
    #[must_use]
    pub fn selected(&self) -> [f32; 4] {
        self.skin.selected.map_or(OUR_SELECTED, argb)
    }

    /// How long a page change takes. `transition=`.
    #[must_use]
    pub fn transition_secs(&self) -> f32 {
        self.skin.transition_secs
    }

    /// Where the screen title sits, and how big.
    ///
    /// The scale is dimensionless and stays as authored: it multiplies a face
    /// that already came off this source.
    #[must_use]
    pub(super) fn title_at(&self) -> (f32, f32, f32) {
        (
            self.skin.title_x * self.from_theirs.0,
            self.skin.title_y * self.from_theirs.1,
            self.skin.title_scale,
        )
    }

    /// The screen title's colour, for a title that draws no frame.
    ///
    /// **Not the disc's, deliberately.** Pulse authors `TitleColor` black,
    /// because on hardware the title sits on a light angled top bar
    /// (`topbarleft`/`topbarcenter`/`topbarright`) that this build does not
    /// draw yet. Using the authored colour before the bar exists paints black
    /// text on a dark backdrop - invisible, and wrong in a way that would look
    /// like a bug rather than like a missing feature. The same substitution the
    /// language picker already makes for the same reason.
    ///
    /// A title whose frame this build *does* draw does not come through here at
    /// all: [`super::draw_list`] takes [`super::Frame::ink`] instead, which is
    /// the colour that screen draws its own chrome in and resolves per archive.
    /// The substitution was always about the missing frame rather than about the
    /// colour.
    #[must_use]
    pub(super) fn title_color(&self) -> [f32; 4] {
        OUR_SELECTED
    }
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
    /// What an unselected entry is drawn in. Authored, and **not**
    /// [`Skin::normal`].
    pub color: [f32; 4],
}

/// How many rows are on screen at once, given the pitch in use.
///
/// Derived rather than picked: rows start at the skin's first row and are one
/// pitch apart, and the noted-row message under them needs a line of its own at
/// `0.8` scale. The last row plus that message must clear the 272-pixel screen.
///
/// At Pulse's measured 28-pixel pitch that is seven rows, which is exactly what
/// the original's own main menu shows.
#[must_use]
pub fn visible_rows(skin: &Skin) -> usize {
    let pitch = skin.row_pitch().max(1.0);
    let room =
        skin.space().size.1 - skin.first_row_y() - skin.line_height * 0.8 - skin.message_gap();
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

/// A packed `0xAARRGGBB` as the renderer's straight-alpha RGBA.
fn argb(value: oag_title::menu::Argb) -> [f32; 4] {
    let byte = |shift: u32| ((value >> shift) & 0xFF) as f32 / 255.0;
    [byte(16), byte(8), byte(0), byte(24)]
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
