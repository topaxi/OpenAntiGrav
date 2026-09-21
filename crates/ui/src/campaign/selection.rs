//! Wipeout HD/Fury's own `Campaign Selection` screen, ahead of `Grid
//! Selection` - the screen this build had no state for at all until this
//! file, which is why a boot here could only ever reach the base `Wipeout
//! HD` campaign's `grid0`..`grid7` and never `Fury`'s `grid8`..`grid15`. See
//! `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury: `Campaign Selection`"
//! section for the full read of the disc's own XML, the RPCS3 measurements
//! this model is built from, and what is chosen rather than measured.
//!
//! **HD only.** Pulse and Wipeout Omega never build a [`CampaignSelection`] -
//! `oag_game::campaign::load` only ever returns one for HD, and `crate`'s
//! own `campaign_stage::CampaignStage` only ever opens on
//! [`crate::campaign_stage::Screen::Selection`] when that is `Some`. Neither
//! title's own flow changes; `crates/ui/src/campaign/tests.rs` pins that.
//!
//! # Read off `DATA06`, not the archive this crate's other two screens read
//!
//! `Grid Selection`/`Cell Selection` are read off whichever archive
//! [`oag_assets::Archives::read_name`]'s precedence resolves for
//! [`oag_hd::campaign::SCREEN_ENTRY`] (`DATA02`, measured). This screen and
//! `Grid Selection Fury` exist only on `DATA06`'s own copy of the same path -
//! see [`oag_hd::campaign::SELECTION_SCREEN_ARCHIVE`]'s own doc for why
//! `oag_game::campaign::load_hd` reaches into that archive by name rather
//! than through the shared precedence.
//!
//! # The screen's own `<Redirect>` is not read generically
//!
//! `Campaign Selection`'s `<Redirect>` carries two `<Entry item="campaignList"
//! equals="..." goto="...">` children - a conditional table keyed on the
//! `campaignList`'s own selection - which `oag_ui::screen::Redirect` does not
//! model at all: that type only ever reads a `<Default goto="...">` (this
//! screen's own `Default` is `"To Be Done"`, an unfinished fallback this
//! build never reaches). So [`Campaign::grid_screen_name`] hardcodes the
//! mapping read directly off the raw XML rather than through the generic
//! screen parser, the same way [`super::neighbour_offsets`] hardcodes a
//! table read directly off `PI001`'s own bytes rather than through a
//! structural reader that does not model it.

use oag_gameplay::input::{Button, Input};

use crate::frontend::{Align, Draw, Placed};
use crate::language::StringTable;
use crate::menu::{Frame, Layers, Picture, Skin};
use crate::pointer::{Pointer, contains};
use crate::screen::argb_to_rgba;

use super::draw::{fill_draw, image_draw, text_draw};
use super::{Event, Layout};

#[cfg(test)]
mod tests;

/// `ScreenTitle`'s own idstring on this screen - `FE_RC_SELECT`,
/// `"CAMPAIGN SELECT"` on an RPCS3 frame. Distinct from `Grid
/// Selection`/`Cell Selection`'s own `FE_RC` (`oag_ui::campaign::hd::TITLE_ID`),
/// the same "each screen names its own title idstring" shape that pair
/// already has.
///
/// **`pub`, unlike that sibling constant** - a caller building this screen's
/// own `strings` needs it (and [`SUBTITLE_ID`]) to overlay the one archive
/// that actually resolves either: see
/// `oag_game::main::session::campaign::open_campaign`'s own doc on why the
/// general string table this crate is handed does not carry either id.
pub const TITLE_ID: &str = "FE_RC_SELECT";

/// The subtitle `MiniText`'s own idstring - `FE_CAMPSEL_MODES`,
/// `"CAMPAIGN MODES"` on an RPCS3 frame. See [`TITLE_ID`]'s own doc for why
/// this is `pub` too.
pub const SUBTITLE_ID: &str = "FE_CAMPSEL_MODES";

/// One of `campaignList`'s own two entries - `Fury` first, `Wipeout HD`
/// second, `<List><Entry String="FE_RC_FURY">` then `<Entry
/// String="FE_RC_HD">`, which is also the measured default (see the module
/// doc's RPCS3 section in `docs/ui/campaign-screens.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Campaign {
    Fury,
    Hd,
}

impl Campaign {
    /// Which slice of [`oag_hd::campaign::DEFINITION_ENTRY`]'s sixteen grids
    /// this campaign's own `Grid Selection` screen pages -
    /// [`oag_hd::campaign::FURY_GRID_RANGE`]/[`oag_hd::campaign::HD_GRID_RANGE`].
    #[must_use]
    pub fn grid_range(self) -> std::ops::Range<usize> {
        match self {
            Campaign::Fury => oag_hd::campaign::FURY_GRID_RANGE,
            Campaign::Hd => oag_hd::campaign::HD_GRID_RANGE,
        }
    }

    /// The `Grid Selection`/`Grid Selection Fury` screen name this
    /// campaign's own `<Redirect>` entry names - see the module doc on why
    /// this is hardcoded rather than read through [`oag_ui::screen::Redirect`].
    #[must_use]
    pub fn grid_screen_name(self) -> &'static str {
        match self {
            Campaign::Fury => oag_hd::campaign::FURY_GRID_SCREEN,
            Campaign::Hd => "Grid Selection",
        }
    }

    /// `campaignList`'s own `<Entry String="...">` for this campaign -
    /// `FE_RC_FURY`/`FE_RC_HD`, `"FURY CAMPAIGN"`/`"HD CAMPAIGN"` resolved
    /// off `DATA06`'s own `Data\Plugins\Languages\English\entries.xml`
    /// (confirmed directly against `hdfury-ps3-eu-dec.iso`: `<entry
    /// id="FE_RC_FURY" string="FURY CAMPAIGN">`, `<entry id="FE_RC_HD"
    /// string="HD CAMPAIGN">`). This is the disc's own name for each entry -
    /// see [`FURY_ENTRY_NAME_POSITION`]'s own doc for where it is actually
    /// drawn, since the list's own authored position is off-screen by design.
    #[must_use]
    pub fn entry_id(self) -> &'static str {
        match self {
            Campaign::Fury => "FE_RC_FURY",
            Campaign::Hd => "FE_RC_HD",
        }
    }
}

/// `campaignList`'s own two entries, in document order.
const CAMPAIGNS: [Campaign; 2] = [Campaign::Fury, Campaign::Hd];

/// The screen's own `<Bracket x="160" y="170" Width="1595" Height="780">` -
/// read directly off the XML, not through [`oag_ui::screen::Screens::collect`],
/// which has no `Bracket` arm at all (nothing else in this crate has needed
/// one before this screen). The frame both flyers sit inside.
const BRACKET_RECT: [f32; 4] = [160.0, 170.0, 1595.0, 780.0];

/// `CAMPAIGN MODES`'s own subtitle - `<MiniText><Values x="160" y="140"
/// idstring="FE_CAMPSEL_MODES" color="FEGlobals->HD_Grey">`.
///
/// **Hand-placed, not read through `screen.texts`.** Adding a `MiniText` arm
/// to [`oag_ui::screen::Screens::collect`] was tried first and reverted:
/// that parser is shared by every title's every screen, and a scratch
/// survey found `<MiniText>` on `MainMenu_Definition.xml`,
/// `RaceBox_Definition.xml`, `Additional_Definition.xml` and
/// `RecordGrid_Definition.xml` too - collecting it generically would have
/// started drawing widgets on screens this pass never measured, on every
/// title, not only this one. `BRACKET_RECT` already sets the precedent for
/// a number the parser does not reach: read directly off the file, kept as
/// a plain constant.
const SUBTITLE_POSITION: (f32, f32) = (160.0, 140.0);

/// `FuryGoldMedalsMiniText`'s own position and colour -
/// `<MiniText name="FuryGoldMedalsMiniText"><Values x="755" y="710"
/// idstring="RC_GM" color="0xff000000">`. Hand-placed - see
/// [`SUBTITLE_POSITION`]'s own doc. The colour is a literal on this one
/// widget (black), unlike [`HD_GOLD_MEDALS_LABEL_POSITION`]'s own
/// `FEGlobals->HD_Grey`.
const FURY_GOLD_MEDALS_LABEL_POSITION: (f32, f32) = (755.0, 710.0);
const FURY_GOLD_MEDALS_LABEL_COLOR: u32 = 0xff00_0000;

/// `HDGoldMedalsMiniText`'s own position - `<MiniText name="HDGoldMedalsMiniText">
/// <Values x="1395" y="710" idstring="RC_GM" color="FEGlobals->HD_Grey">`.
/// Hand-placed - see [`SUBTITLE_POSITION`]'s own doc. Its own colour is
/// `FEGlobals->HD_Grey`, read off `ScreenTitle`'s own already-collected
/// `Text.color` in [`draw_list`] rather than re-resolved here - both name
/// the same global, and `ScreenTitle` is a plain `Text`, not a `MiniText`,
/// so it is already in `screen.texts` with the value resolved.
const HD_GOLD_MEDALS_LABEL_POSITION: (f32, f32) = (1395.0, 710.0);

/// `RC_GM`'s own idstring - `"GOLD MEDALS"` on an RPCS3 frame, the same
/// idstring `Grid Selection`'s own `Medals Title` resolves. Named here since
/// both hand-placed gold-medal labels share it.
const GOLD_MEDALS_LABEL_ID: &str = "RC_GM";

/// Where this build draws `campaignList`'s own two entry names
/// ([`Campaign::entry_id`]) - the list's own authored position
/// (`<List name="campaignList"><Values x="159" y="-500">`) is off-screen by
/// design (the file's own comment on that line reads `<!-- -500 for y so
/// it's off screen!! -->`), there only to drive the widget's internal
/// selection state, never meant to be drawn. **Chosen, not measured**: `x`
/// reuses the same split [`FURY_GOLD_MEDALS_LABEL_POSITION`]/
/// [`HD_GOLD_MEDALS_LABEL_POSITION`] already commit to (755/1395, half of
/// [`BRACKET_RECT`]), and `y` sits above them, inside the Bracket, in the
/// space the (undrawn) 3D flyer card would otherwise fill - see the module
/// doc's "not drawn" note.
const FURY_ENTRY_NAME_POSITION: (f32, f32) = (755.0, 610.0);
const HD_ENTRY_NAME_POSITION: (f32, f32) = (1395.0, 610.0);

/// The selected half's outline thickness and colour - [`BRACKET_RECT`] split
/// at its own midpoint, the same split [`CampaignSelection::pointer`]'s own
/// click targets already use. The screen authors no per-entry
/// `Selector`/highlight widget at all, unlike `Grid Selection`/`Cell
/// Selection`'s own `<Image name="Selector">`; the real game's equivalent is
/// presumably the 3D flyer's own scale/emphasis animation (an RPCS3 frame
/// shows the selected card larger and centred), which this build does not
/// render - see the module doc's "not drawn" note. **Chosen, not measured**:
/// a plain four-sided outline around the selected half stands in for that
/// animation, so the choice is visible and the pointer has something to
/// hover, without pretending to reproduce the disc's own look.
const SELECTOR_BORDER: f32 = 6.0;
const SELECTOR_COLOR: u32 = 0xffff_ffff;

/// The selected half of [`BRACKET_RECT`], as a plain four-sided outline -
/// see [`SELECTOR_BORDER`]'s own doc.
fn selector_outline(selected: Campaign) -> [Draw; 4] {
    let slot = CAMPAIGNS
        .iter()
        .position(|&entry| entry == selected)
        .unwrap_or(0);
    #[allow(clippy::cast_precision_loss, reason = "slot is 0 or 1, exact in f32")]
    let slot = slot as f32;
    let [x, y, width, height] = BRACKET_RECT;
    let half = width / 2.0;
    let rect = [x + half * slot, y, half, height];
    let color = argb_to_rgba(SELECTOR_COLOR);
    let [rx, ry, rw, rh] = rect;
    let t = SELECTOR_BORDER;
    [
        Draw::Fill {
            rect: [rx, ry, rw, t],
            color,
        },
        Draw::Fill {
            rect: [rx, ry + rh - t, rw, t],
            color,
        },
        Draw::Fill {
            rect: [rx, ry, t, rh],
            color,
        },
        Draw::Fill {
            rect: [rx + rw - t, ry, t, rh],
            color,
        },
    ]
}

/// `Campaign Selection`'s own model: a flat two-entry list, the same shape
/// [`super::GridSelection`]'s own pager is, stepped by `left`/`right`
/// (measured: `right` moves `Fury` -> `Wipeout HD`, see the module doc)
/// rather than up/down.
#[derive(Debug, Clone, Default)]
pub struct CampaignSelection {
    index: usize,
}

impl CampaignSelection {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts on `campaign` already selected - what a caller returning to
    /// this screen from `Grid Selection`/`Grid Selection Fury` uses, so a
    /// player who chose `Wipeout HD` and backs out lands on `Campaign
    /// Selection` still showing `Wipeout HD` selected, not reset to the
    /// screen's own default `Fury` entry.
    #[must_use]
    pub fn at(campaign: Campaign) -> Self {
        Self {
            index: CAMPAIGNS
                .iter()
                .position(|&entry| entry == campaign)
                .unwrap_or(0),
        }
    }

    #[must_use]
    pub fn selected(&self) -> Campaign {
        CAMPAIGNS[self.index]
    }

    /// `right` (`1`) is the measured toggle. **Clamped, not wrapping** -
    /// unlike [`super::GridSelection::step`]'s own sixteen-tier pager, this
    /// is measured directly rather than chosen for consistency with it:
    /// `left` (`-1`) at the list's own first entry (`Fury`, the default) is
    /// observed to leave the screen on `Fury`, not wrap to `Wipeout HD` the
    /// way a wrapping step would. A wrapping step would make `left` and
    /// `right` indistinguishable from `Fury` - both landing on `Hd` - which
    /// the RPCS3 boot this is read from directly contradicts. See the
    /// module doc's "The toggle" section.
    fn step(&mut self, step: i32) {
        let last = i32::try_from(CAMPAIGNS.len().saturating_sub(1)).unwrap_or(0);
        let index = i32::try_from(self.index).unwrap_or(0) + step;
        self.index = usize::try_from(index.clamp(0, last)).unwrap_or(0);
    }

    /// The four directions and the two buttons this screen answers to -
    /// `up`/`down` unbound, since neither was independently isolated as a
    /// toggle or a no-op (see the module doc's confidence-~50 note).
    pub fn update(&mut self, input: &mut Input) -> Vec<Event> {
        let mut out = Vec::new();
        if input.take(Button::Right) {
            self.step(1);
            out.push(Event::Moved);
        }
        if input.take(Button::Left) {
            self.step(-1);
            out.push(Event::Moved);
        }
        if input.take(Button::Cross) || input.take(Button::Start) {
            out.push(Event::Confirmed);
        }
        if input.take(Button::Circle) {
            out.push(Event::Back);
        }
        out
    }

    /// Two click targets, invented from [`BRACKET_RECT`] split at its own
    /// midpoint - Fury's flyer sits left of centre (`OriginX="640"`),
    /// `Wipeout HD`'s right (`OriginX="1280"`), and the screen authors no
    /// per-flyer clickable rect of its own to ground a target in more
    /// precisely than that, the same "grounded in a real rect, not
    /// free-standing" reasoning [`super::hd::hd_grid_targets`] already uses
    /// for `Grid Selection`'s own confirm region. **Chosen, not measured.**
    /// Hover selects, a second click on the already-selected half confirms -
    /// the same two-tap idiom every other campaign screen in this crate
    /// uses.
    pub fn pointer(&mut self, pointer: &Pointer) -> Vec<Event> {
        let mut out = Vec::new();
        if pointer.is_idle() {
            return out;
        }
        let [x, y, width, height] = BRACKET_RECT;
        let half = width / 2.0;
        let rects = [[x, y, half, height], [x + half, y, half, height]];
        let hit = pointer
            .at
            .and_then(|at| rects.iter().position(|&r| contains(r, at)));
        let was = self.index;
        if pointer.moved
            && let Some(index) = hit
            && index != was
        {
            self.index = index;
            out.push(Event::Moved);
        }
        if pointer.clicked
            && let Some(index) = hit
        {
            if index == self.index {
                out.push(Event::Confirmed);
            } else {
                self.index = index;
                out.push(Event::Moved);
            }
        }
        if pointer.back {
            out.push(Event::Back);
        }
        out
    }
}

/// `Campaign Selection`'s draw list.
///
/// `fury_gold_medals`/`hd_gold_medals` are each campaign's own earned-medal
/// count - `0` on a fresh profile, the same "player-progress source is
/// optional" reading the rest of this module gives. Drawn as the bare
/// numerator only, no denominator: an RPCS3 frame reads `"0 / 87"` beside
/// `Fury`'s own flyer, and this build's own parse of `DATA00`'s eight Fury
/// grids totals 80 cells, a gap this pass does not explain - see the module
/// doc. **Never invent what the assets do not settle**: draw the number this
/// build can derive, not a guessed denominator.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same shape every other screen's own *_draw_list takes, plus the two medal \
              counts this screen alone needs"
)]
pub fn draw_list(
    model: &CampaignSelection,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    fury_gold_medals: u32,
    hd_gold_medals: u32,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let mut layers = Layers {
        backdrop: frame.backdrops(
            skin.space(),
            skin.background(),
            backdrop.map(Picture::draw),
            race_behind,
        ),
        ..Layers::default()
    };
    let (title_x, title_y, title_scale) = skin.title_at();
    layers.chrome.push(Draw::title(
        skin.title_font(),
        title_x,
        title_y,
        title_scale,
        skin.title_color(frame.ink),
        strings.get_or_id(TITLE_ID).to_string(),
    ));

    let screen = &layout.screen;
    let mut out = Vec::new();
    for fill in &screen.fills {
        out.push(fill_draw(fill));
    }
    for image in &screen.images {
        // Both crop no sub-rect of `Hexmedal_HD.mip`'s own shared
        // multi-colour atlas, unlike `Target0/1/2 Medal`'s use of the same
        // atlas elsewhere on this title (`u`/`v`/`TxtrWidth`) - drawing
        // either as authored would show the whole atlas, not a clean glyph.
        // Not drawn rather than guessed at; see the module doc.
        if matches!(
            image.name.as_deref(),
            Some("MedalImageFury" | "MedalImageHD")
        ) {
            continue;
        }
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        out.push(image_draw(image, placed));
    }
    for text in &screen.texts {
        let name = text.name.as_deref().unwrap_or("");
        let content = match name {
            // `ScreenTitle` is drawn above through `skin.title_font()`,
            // the same idiom `oag_ui::campaign::hd` already uses for its
            // own two screens' titles.
            "ScreenTitle" => None,
            // `RB_EVENT_TYPE` is a reused/generic idstring this build does
            // not trust as content - see [`draw_list`]'s own doc.
            "NumMedalsTextFury" => Some(fury_gold_medals.to_string()),
            "NumMedalsTextHD" => Some(hd_gold_medals.to_string()),
            _ => text.string.clone(),
        };
        let Some(content) = content else { continue };
        out.push(text_draw(text, &content, layout));
    }

    // The subtitle and the two gold-medal labels - hand-placed, not
    // resolved through `screen.texts`; see `SUBTITLE_POSITION`'s own doc on
    // why. `grey` is `FEGlobals->HD_Grey`'s own resolved ARGB, read off
    // `ScreenTitle`'s already-collected `Text.color` rather than
    // re-resolved here - both name the same global.
    let grey = screen
        .texts
        .iter()
        .find(|text| text.name.as_deref() == Some("ScreenTitle"))
        .map_or(0xffff_ffff, |text| text.color);
    let hand_placed = [
        (SUBTITLE_POSITION, grey, strings.get_or_id(SUBTITLE_ID)),
        (
            FURY_ENTRY_NAME_POSITION,
            grey,
            strings.get_or_id(Campaign::Fury.entry_id()),
        ),
        (
            HD_ENTRY_NAME_POSITION,
            grey,
            strings.get_or_id(Campaign::Hd.entry_id()),
        ),
        (
            FURY_GOLD_MEDALS_LABEL_POSITION,
            FURY_GOLD_MEDALS_LABEL_COLOR,
            strings.get_or_id(GOLD_MEDALS_LABEL_ID),
        ),
        (
            HD_GOLD_MEDALS_LABEL_POSITION,
            grey,
            strings.get_or_id(GOLD_MEDALS_LABEL_ID),
        ),
    ];
    for ((x, y), color, content) in hand_placed {
        out.push(Draw::Text {
            x,
            y,
            scale: 1.0,
            color: argb_to_rgba(color),
            border: None,
            align: Align::Left,
            text: content.to_string(),
            wrap_width: None,
        });
    }

    // The selected half's outline - see [`SELECTOR_BORDER`]'s own doc on why
    // this is drawn rather than a `Selector` widget the screen does not
    // author.
    out.extend(selector_outline(model.selected()));

    layers.body = out;
    layers
}
