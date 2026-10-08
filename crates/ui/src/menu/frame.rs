//! The frame a title draws its menus inside, read off the disc's own screen.
//!
//! # Why this is not a table
//!
//! Wipeout HD's front end is a white field with a grey rule above and below the
//! page and a small arrow in the top left, and every one of those is authored -
//! on `FE Screen`, the parent every menu screen is nested inside, in the same
//! `skin.xml` this build already parses. So the title package names **the
//! screen** ([`oag_title::FrontEnd::menu_frame`]) and this reads the widgets off
//! it at runtime. Nothing here knows 160, 110, 1600 or 8.
//!
//! That is `CLAUDE.md`'s rule applied to chrome: *if a thing exists in the
//! disc's own data, play the data* - and its first named failure is exactly a
//! table of four-number widgets transcribed by hand, which was correct to the
//! digit and still wrong.
//!
//! # What is drawn, and what is left out
//!
//! The screen's `<ScreenClear>` and its `<Image>` widgets. **Not its
//! `<Text>`s**: on HD those are the trial build's (`FE_TRIAL_MODE`,
//! `FE_PURCHASE_NOW`) or belong to a `<NavigationController>`, which decides
//! per screen which button prompts to show and which this build does not have.
//! Drawing them unconditionally would put "purchase now" on a retail menu. The
//! images and the clear carry no such owner, which is the line drawn here.
//!
//! An image the sprite sheet has no placement for is skipped, so a frame whose
//! textures did not decode is a missing rule rather than a wrong one.

pub mod settings;

use crate::frontend::{Draw, Placed};

use crate::screen::{Screens, argb_to_rgba, parse_argb};
use oag_display::space::Space;

/// One title's menu frame, split by what may come between the two halves.
///
/// A menu can have a looping movie behind it *and* a frame around it, and the
/// two interleave - **Pulse carries both**, its `FE Screen` authoring the
/// looping `Data\Movies\Backdrop` and four `<Image>` widgets over the same
/// screen. So the interleaving is not arranged any more, it is the thing
/// [`Self::backdrops`] has to get right, and keeping the halves apart is what
/// lets [`super::draw_list`] put the movie between them without either half
/// having to know it exists.
#[derive(Debug, Default, Clone)]
pub struct Frame {
    /// What the screen clears to, as one screen-filling [`Draw::Fill`].
    pub clear: Option<Draw>,
    /// The screen's own images: rules, corner marks, whatever it authors.
    pub marks: Vec<Draw>,
    /// The colour the frame draws its own chrome in, when every mark agrees.
    ///
    /// **Read rather than chosen, and it is how the palette gets resolved
    /// without anything transcribing it.** Wipeout HD's three frame widgets are
    /// all tinted `FEGlobals->HD_Grey`, and its main menu's title text names that
    /// same global - so "the title is the colour of the rules" is what the disc
    /// says on this screen, not an inference about contrast. Taking it from the
    /// marks means it resolves to whatever the *served* archive declares, which
    /// matters: `HD_Grey` is `0xff646464` in `DATA06` and `0xff969696` in
    /// `DATA00`, because the `HD_*` palette is the FE style rather than a fixed
    /// table.
    ///
    /// `None` when the marks disagree or there are none, and then the caller
    /// keeps whatever it would have used. No title but HD has a frame at all
    /// today, so "they all agree" has one case behind it.
    pub ink: Option<[f32; 4]>,
    /// A selected strip entry's own tab fill, resolved off this screen's
    /// globals - [`oag_title::MenuStrip::selected_fill`]'s name, looked up the
    /// same way `crate::loading`'s own palette resolves its four names.
    ///
    /// **Not derived from a mark**, unlike [`Self::ink`]: no widget on `FE
    /// Screen` references `HD_Blue`, so there is nothing to read consensus
    /// off. The global is read by the name the title package supplies
    /// instead, which is why this needs that name passed in rather than
    /// discovering it the way `ink` does. `None` when the title names no
    /// strip, or names one whose global this screen's globals table does not
    /// carry.
    pub tab_selected: Option<[f32; 4]>,
    /// The art the title's menu blocks draw with, when the title has blocks
    /// and their nine-patch decoded - see [`super::block::BlockArt`]. Built
    /// by the caller, which is where the decoded pixels are; `None` draws
    /// every entry as bare text, which is what both PSP titles are.
    pub blocks: Option<super::block::BlockArt>,
    /// The original's settings-screen look for the pages that stand where one
    /// does - see [`settings::Layout`]. `None` for a title that names none.
    pub settings: Option<settings::Layout>,
}

impl Frame {
    /// Whether this title draws any frame at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.clear.is_none() && self.marks.is_empty()
    }

    /// This frame's own clear/marks layer, with the movie between its two
    /// halves and anything covering the whole screen left out when
    /// `race_behind` - a parked race's own picture, or [`super::MenuStage`]'s
    /// pause overlay over it.
    ///
    /// # A full-screen mark is a backdrop, not chrome
    ///
    /// Pulse's `FE_SCREEN` opens its images with `gameshare_backdrop.mip` at
    /// `[0, 0, space.size.0, space.size.1]` - a plain `<Image>`, not a
    /// `<ScreenClear>` - ahead of the top bar and the two footer strips
    /// [`Self::content_bottom`] already reads. Drawn with the rest of the
    /// marks it painted straight over the looping movie every frame, and the
    /// menu background read as one frozen still of the backdrop: the still is
    /// a picture of the same ship reel, so the movie looked stopped rather
    /// than covered. Confirmed rather than reasoned about - the playhead, the
    /// feed's ring and the uploaded frame index were all instrumented live
    /// and all advancing, and a `--menu-page` capture under `--no-video` came
    /// out visually identical to one with the movie playing.
    ///
    /// So a mark covering the whole screen goes **under** the movie, where a
    /// game-share session with no reel to play still gets its background, and
    /// only the structural marks stay over it. `race_behind` then drops that
    /// whole under-the-movie group - the clear, `MenuSkin::background` and the
    /// full-screen marks alike - which is one rule rather than the two this
    /// used to carry: a parked race's picture, or the pause overlay's own
    /// `Draw::Fill` under it, is what shows through instead. Before that
    /// filter existed a real `Escape` from a running race showed the plain
    /// undimmed menu with every value upstream of this call correct; see
    /// `docs/architecture/menus.md`'s "backing into the menus..." paragraph.
    ///
    /// **Ordered by geometry, because the disc's own order is not kept.**
    /// [`crate::screen::Screen`] holds `images` and `movies` in two vectors,
    /// so which of an `<Image>` and the screen's movie is authored first is
    /// lost by the time this runs. What settles it is not the XML anyway: a
    /// player who has run the original reports the reel playing behind the
    /// menus, which a full-screen still over it makes impossible. Recovering
    /// the sibling order would make this readable off the data rather than
    /// inferred from a rect, and is the change to make if a title ever
    /// authors a full-screen mark it means to draw *over* its movie.
    ///
    /// `video` is the picture quad, already built - a movie frame or the Fury
    /// backdrop's pass; this type does not know how to play either. `clear` and [`oag_title::MenuSkin::background`]
    /// (`skin_background`, passed in rather than read here - this type does
    /// not hold a `Skin`) are always full-screen by construction, so they join
    /// that group without a rect comparison; a mark is judged by its own rect
    /// against `space`, the same parameter [`Self::content_bottom`] already
    /// takes and for the same reason: this type holds no `Space` of its own.
    #[must_use]
    pub fn backdrops(
        &self,
        space: Space,
        skin_background: Option<Draw>,
        video: Option<Draw>,
        race_behind: bool,
    ) -> Vec<Draw> {
        let full_screen = [0.0, 0.0, space.size.0, space.size.1];
        let covers_screen =
            |mark: &Draw| matches!(mark, Draw::Sprite { rect, .. } if *rect == full_screen);
        let mut out: Vec<Draw> = Vec::new();
        if !race_behind {
            out.extend(self.clear.clone().or(skin_background));
            out.extend(
                self.marks
                    .iter()
                    .filter(|mark| covers_screen(mark))
                    .cloned(),
            );
        }
        out.extend(video);
        out.extend(
            self.marks
                .iter()
                .filter(|mark| !covers_screen(mark))
                .cloned(),
        );
        out
    }

    /// Where this frame's own chrome begins, measured up from the bottom of
    /// `space`: the smallest `y` among the marks sitting in the screen's
    /// lower half.
    ///
    /// **The real answer to "where does the content area end and the chrome
    /// begin"** - [`super::visible_rows`] and `oag_ui_screens::prompt::axis_preview_draw`
    /// both read this instead of assuming a page's own rows may use the whole
    /// screen down to `space.size.1`, which is what let a menu draw straight
    /// through Pulse's own footer strips. See `docs/architecture/menus.md`'s
    /// "AI PILOTS: the axis preview" section for the capture that found it -
    /// Pulse's footer sits at `y=236` of 272, well short of the screen's own
    /// edge.
    ///
    /// **The lower half only**, not every mark: a title's frame can carry
    /// chrome above the rows too - Pulse's own top bar, Wipeout HD's upper
    /// rule - and folding those in as well would read a mark at `y=0` as
    /// "content ends immediately", collapsing the content area to nothing.
    /// Nothing this build draws sits above the title anyway, so only the
    /// marks below the midline can be *this* boundary.
    ///
    /// `None` for a frame with no mark in the lower half - every title with
    /// no frame at all, and any future one whose chrome sits entirely above
    /// the rows - so a caller with nothing to clear keeps using whatever
    /// bound it read before this existed.
    #[must_use]
    pub fn content_bottom(&self, space: Space) -> Option<f32> {
        let midline = space.size.1 * 0.5;
        self.marks
            .iter()
            .filter_map(|mark| match mark {
                Draw::Sprite {
                    rect: [_, y, _, _], ..
                } if *y >= midline => Some(*y),
                _ => None,
            })
            .reduce(f32::min)
    }

    /// One line saying what this frame came out as.
    ///
    /// The colours are the point: they are the only things in the frame that
    /// a *different* archive would have made different, so a report that
    /// names them is what tells a reader which FE style they are looking at
    /// without opening the disc. Moved here from `boot.rs`, 2026-09-01, where
    /// it was the only piece of that file talking about a type it does not
    /// own.
    #[must_use]
    pub fn describe(&self) -> String {
        let hex = |color: [f32; 4]| {
            let byte = |channel: f32| (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
            format!(
                "#{:02x}{:02x}{:02x}",
                byte(color[0]),
                byte(color[1]),
                byte(color[2])
            )
        };
        let clear = match &self.clear {
            Some(Draw::Fill { color, .. }) => format!("clears to {}", hex(*color)),
            _ => "no clear".to_string(),
        };
        let ink = self.ink.map_or_else(
            || String::from("no single ink"),
            |ink| format!("ink {}", hex(ink)),
        );
        let tab_selected = self.tab_selected.map_or_else(
            || String::from("no strip highlight"),
            |color| format!("strip highlight {}", hex(color)),
        );
        format!(
            "{clear}, {} mark(s), {ink}, {tab_selected}",
            self.marks.len()
        )
    }
}

/// Reads `name`'s frame out of already-parsed screens.
///
/// Returns an empty [`Frame`] when the title names no frame screen, when the
/// parsed front end has no screen by that name, or when the screen authors
/// neither a clear nor an image - three different absences that all mean the
/// same thing to a caller: draw the menus on what the pass cleared to.
///
/// `selected_fill` is [`oag_title::MenuStrip::selected_fill`]'s own name -
/// `None` on a title with no strip - looked up in `screens`' globals
/// directly rather than off this screen's marks, since it is resolved
/// independently of whether a frame screen exists at all.
///
/// `sprites` is a decoded sheet's own placements, name and rectangle
/// together - not `crate::sprite::Sheet` itself, which decodes real texture
/// data one level above this crate and so may not be named here. The same
/// shape `crate::frontend::Frontend`'s own `placements` field already is;
/// `crate::sprite::Sheet::entries` returns exactly this, so a real caller
/// passes `sheet.entries()`.
///
/// `blocks` is the menu blocks' decoded art, built by the same caller for
/// the same reason - sampling the fill swatch needs the sheet's pixels, which
/// this crate never sees. `None` for a title with no blocks.
#[must_use]
pub fn read(
    screens: &Screens,
    sprites: &[(String, Placed)],
    space: Space,
    name: Option<&str>,
    selected_fill: Option<&str>,
    blocks: Option<super::block::BlockArt>,
) -> Frame {
    let tab_selected = selected_fill
        .and_then(|global| screens.globals.get(global))
        .and_then(|value| parse_argb(value))
        .map(argb_to_rgba);

    let Some(screen) = name.and_then(|name| screens.by_name(name)) else {
        return Frame {
            tab_selected,
            blocks,
            ..Frame::default()
        };
    };

    let clear = screen.clear.map(|argb| Draw::Fill {
        rect: [0.0, 0.0, space.size.0, space.size.1],
        color: argb_to_rgba(argb),
    });

    let mut marks = Vec::new();
    for image in &screen.images {
        let Some(placed) = sprites
            .iter()
            .find(|(src, _)| *src == image.src)
            .map(|(_, placed)| *placed)
        else {
            continue;
        };
        // The widget's own size when it states one, the texture's otherwise -
        // and `line.gtf` is why the widget's has to win: it is an 8x8 tile
        // stretched to `1600x8`, so a rule drawn at its texture's size is 8
        // pixels of what should be most of the screen. The same rule
        // `crate::frontend`'s screen drawing uses, including its centring of an
        // image that states no `x`.
        let width = image.width.unwrap_or(placed.width as f32);
        let height = image.height.unwrap_or(placed.height as f32);
        let x = if image.x == 0.0 {
            (space.size.0 - width) / 2.0
        } else {
            image.x
        };
        // `U`/`V`/`TxtrWidth`/`TxtrHeight` name a sub-rect of `src`'s own
        // texture, exactly as `crate::frontend::draw`'s own screen drawing
        // reads them - see [`crate::screen::Image::u`]. HD's three marks never
        // needed this, each being its own whole texture; Pulse's frame is the
        // case that does, its top bar and its two footer strips all three
        // patches of one shared `pulse_assets.mip`. Without it every mark
        // would draw that sheet's own top-left corner instead of its own
        // patch, same failure `frontend::draw` had before it read these.
        let uv = [
            placed.x as f32 + image.u.unwrap_or(0.0),
            placed.y as f32 + image.v.unwrap_or(0.0),
            image.texture_width.unwrap_or(placed.width as f32),
            image.texture_height.unwrap_or(placed.height as f32),
        ];
        marks.push(Draw::Sprite {
            rect: [x, image.y, width, height],
            uv,
            // The widget's tint, which on HD is `FEGlobals->HD_Grey` for all
            // three: the textures themselves are white and the colour is what
            // makes a rule grey.
            color: argb_to_rgba(image.color),
        });
    }

    // One colour or none: the marks either agree, in which case that is what
    // this screen draws chrome in, or they do not, in which case there is no
    // such thing and nothing here invents one.
    let ink = screen
        .images
        .first()
        .map(|first| first.color)
        .filter(|&first| screen.images.iter().all(|image| image.color == first))
        .filter(|_| !marks.is_empty())
        .map(argb_to_rgba);

    Frame {
        clear,
        marks,
        ink,
        tab_selected,
        blocks,
        settings: None,
    }
}
