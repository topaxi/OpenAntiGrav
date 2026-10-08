//! The Race Campaign's screens, over the menu stage - the same shape
//! [`crate::picker_stage`] holds the race box's two in. See
//! `oag_ui_screens::campaign` for the model and the drawing, and
//! `crate::session::campaign` for the flow that opens and closes one.
//!
//! **HD/Fury adds a third screen ahead of the other two**: `Campaign
//! Selection`, picking between the base `Wipeout HD` campaign
//! (`grid0`..`grid7`) and `Fury` (`grid8`..`grid15`) before either ever pages
//! a grid tier. Every other title never builds [`Screen::Selection`] at all -
//! `CampaignStage::new` opens straight on [`Screen::Grid`] the way it always
//! has when [`CampaignStage::has_selection`] is `false`. See
//! `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury: `Campaign Selection`"
//! section.

use oag_tables::race_campaign;
use oag_ui_screens::campaign::selection::{Campaign, CampaignSelection};

use oag_game::unlock::{campaign_medal_of, to_campaign_medal};

/// [`to_campaign_medal`]'s own sibling, for
/// `oag_game::records::CampaignRecord::best_difficulty` - **HD only**;
/// every Pulse cell's own row never has one to convert.
fn to_campaign_difficulty(difficulty: oag_game::records::Difficulty) -> race_campaign::Difficulty {
    match difficulty {
        oag_game::records::Difficulty::Easy => race_campaign::Difficulty::Easy,
        oag_game::records::Difficulty::Medium => race_campaign::Difficulty::Medium,
        oag_game::records::Difficulty::Hard => race_campaign::Difficulty::Hard,
    }
}

/// One open campaign screen: `Campaign Selection` (HD only), `Grid
/// Selection`, or `Cell Selection` over one of its tiers.
pub(crate) enum Screen {
    /// **HD only** - see the module doc. Never built for any other title.
    Selection(CampaignSelection),
    Grid(oag_ui_screens::campaign::GridSelection),
    /// `which` is the absolute [`CampaignStage::grids`] index the player
    /// drilled into - what `Event::Back` on `Cell Selection` returns `Grid
    /// Selection` to. Absolute, not relative to
    /// [`CampaignStage::grid_range`], so `CampaignStage::grids().get(which)`
    /// always works regardless of which campaign (if any) is open.
    Cell {
        model: oag_ui_screens::campaign::CellSelection,
        which: usize,
    },
}

/// `Cell Selection`'s cursor: **one `(x, y)` slot shared by every grid**, not a
/// memory per grid.
///
/// **Measured** (PPSSPP, `pulse-psp-usa.chd`, 2026-10-02, two boots,
/// `docs/ui/campaign-screens.md`'s "Where the cursor lives"): the original
/// keeps the cursor as the `Selector` widget's own `(x, y)` and re-resolves it
/// by the *current* grid's name on entry. Left on `grid0_3_2`, entering `grid1`
/// lands on `grid1_3_2`, not on `grid1`'s first-unlocked default; left on
/// `grid1_3_1`, entering `grid0` lands on `grid0_3_1`, not on the `grid0_3_2`
/// it was last left on. The slot is kept only when the new grid has a cell
/// there that **shows no lock glyph** - the rule the screen draws with
/// (`Locked` set, no medal of its own, no medalled hex neighbour), which is
/// the original's own tile-flag test on the lock layer: a play-unlocked cell
/// (`Locked` still 1, glyph hidden by a medalled neighbour) keeps the cursor,
/// a glyph-visible one falls to the first-unlocked default (`grid0_2_2` into
/// `grid1`, and `grid0_2_1` back into `grid0` itself, both measured). It also survives
/// leaving the campaign altogether, which is why [`Session`] holds it rather
/// than this stage.
///
/// **Chosen, not measured**: that HD/Fury's own `Cell Selection` (which shares
/// this stage) behaves the same way - only Pulse PSP was observed.
///
/// [`Session`]: crate::session::Session
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CellCursor(Option<(u32, u32)>);

impl CellCursor {
    /// Records `model`'s selected cell's slot as the cursor. A cell whose
    /// name carries no `(x, y)` leaves the slot as it was.
    pub(crate) fn remember(&mut self, model: &oag_ui_screens::campaign::CellSelection) {
        if let Some(slot) = model.selected().and_then(|cell| cell.grid_coords()) {
            self.0 = Some(slot);
        }
    }

    /// Moves `model`'s cursor to the cell at the remembered slot, when this
    /// grid has one and it shows no lock glyph; otherwise a no-op, which
    /// keeps `CellSelection::new`'s first-unlocked default.
    pub(crate) fn restore(&self, model: &mut oag_ui_screens::campaign::CellSelection) {
        let Some(slot) = self.0 else {
            return;
        };
        let Some(target) = model
            .cells()
            .iter()
            .find(|cell| cell.grid_coords() == Some(slot))
            .map(|cell| cell.name.clone())
        else {
            return;
        };
        let before = model.selected().map(|cell| cell.name.clone());
        model.select_by_name(&target);
        if model.selected_is_locked()
            && let Some(before) = before
        {
            model.select_by_name(&before);
        }
    }
}

/// The whole campaign, read once when it opens.
///
/// **Every grid, not only the selected one.** `Grid Selection`'s own honey
/// counter and page need the full sixteen up front, and confirming a tier
/// needs that tier's own cells - reading sixteen small XML files once on
/// open is cheaper than reopening the disc archive on every confirm.
pub(crate) struct CampaignStage {
    grids: Vec<race_campaign::Grid>,
    grid_layout: oag_ui_screens::campaign::Layout,
    cell_layout: oag_ui_screens::campaign::Layout,
    /// **HD only**, both `Some` or both `None` together - see
    /// `oag_game::campaign::Campaign`'s own field docs for why they can be
    /// absent even on an HD source.
    selection_layout: Option<oag_ui_screens::campaign::Layout>,
    grid_layout_fury: Option<oag_ui_screens::campaign::Layout>,
    /// Which slice of [`Self::grids`] the current [`Screen::Grid`]/
    /// [`Screen::Cell`] pages - `0..grids.len()` (every grid) on every title
    /// but HD once a campaign is chosen, when it narrows to that campaign's
    /// own eight via [`Self::open_grid_selection`]. Meaningless while
    /// [`Self::screen`] is [`Screen::Selection`], and reset back to the
    /// whole list only by [`Self::new`] itself - `open_grid_selection` is
    /// the only other writer, and it always narrows.
    grid_range: std::ops::Range<usize>,
    /// **HD only.** Which campaign [`Self::grid_range`] currently reflects,
    /// for [`Self::grid_layout`]'s own choice between
    /// [`Self::grid_layout_fury`] and [`Self::grid_layout`], and for
    /// [`Self::open_selection`] to restore the right entry on the way back.
    /// `None` before a campaign is ever chosen this session, and on every
    /// non-HD title.
    active_campaign: Option<Campaign>,
    /// `Cell Selection`'s own `Cell Help` overlay - see
    /// `oag_game::campaign::Campaign::cell_help`'s own doc. Drawn as a
    /// static (non-scrolling) panel while `CellSelection::help_open`, per
    /// `docs/ui/campaign-screens.md`'s `Open` entry on why not the
    /// authored `Viewport`/`Animation` scroll timeline.
    cell_help: Option<oag_ui_screens::campaign::Layout>,
    /// The front-end root's own `Confirm`/`Back` legend - `None` on a
    /// source whose `Skin.xml` this pass could not read at all (logged when
    /// that happens, see `oag_game::campaign::read_footer`).
    nav_legend: Option<oag_ui_screens::campaign::footer::NavigationLegend>,
    /// The front-end root's own scrolling tip ticker layout - content is
    /// supplied per frame by [`Self::ticker_tips`], not carried here.
    ticker: Option<oag_ui_screens::campaign::footer::TickerLayout>,
    /// Seconds since this campaign screen opened, advanced by
    /// [`Self::tick_ticker`] - the ticker's own clock. Never reset between
    /// `Grid Selection` and `Cell Selection`, unlike `crate::marquee::Timer`:
    /// the original ticker is not observed to restart on a screen change
    /// (see `docs/ui/campaign-screens.md`), so this build keeps it running.
    ticker_elapsed: f32,
    /// Kept for the render pass, which needs to resolve a per-cell idstring
    /// (`MSC_EVENT_SR` and friends) that neither screen's own `Layout::read`
    /// pass can, since which one applies depends on the selected cell's
    /// mode - read once, the same table `Layout::read` used to resolve every
    /// fixed label.
    pub(crate) strings: oag_ui::language::StringTable,
    /// The front end's own sprite sheet - `hex_filled.mip`/`hex_outline.mip`/
    /// `pulse_assets.mip`'s lock-and-selector sub-rects all come off it.
    /// Neither screen needs its own extended sheet the way a picker's
    /// slideshow does: nothing here is per-entity art.
    pub(crate) sprites: oag_hud::sprite::Sheet,
    pub(crate) screen: Screen,
    /// Why the last confirmed cell did not launch, shown a row above the
    /// footer until the player does anything else - see
    /// [`Self::nav_legend_draw`]. `None` the rest of the time.
    pub(crate) notice: Option<String>,
    /// [`oag_title::Title::name`], for [`Self::medal_of`] - the same string
    /// `oag_game::records::Key::new`'s own `title` takes. Also what
    /// `crate::main::menu_stage::MenuStage::render` and
    /// `crate::main::session::pointer::campaign_pointer` dispatch on to
    /// pick between Pulse's draw/pointer functions and
    /// [`oag_ui_screens::campaign::hd`]'s own, since both screens share one name.
    title: String,
    /// [`oag_title::Campaign::dialect`] of the title, the draw list's choice.
    dialect: oag_title::CampaignDialect,
    /// **HD only** - Pulse's own `Track Line` never needs this fold (see
    /// `oag_ui_screens::campaign::draw::track_line`'s own doc), so this is
    /// `CircuitNames::default()` on every other title and costs nothing to
    /// carry. `crate::boot::Shell::circuit_names`, read once when the
    /// campaign opens.
    circuit_names: oag_ui::language::CircuitNames,
    /// A snapshot of the player's own saved progress, taken once when the
    /// campaign opens - the same "read once on open" choice [`Self::grids`]
    /// already makes, and safe for the same reason: nothing mutates
    /// `Session::records` while a campaign screen is open, only a finished
    /// or escaped race does, and reaching one closes this stage first.
    records: oag_game::records::Store,
    /// See [`CellCursor`]'s own doc.
    cell_cursor: CellCursor,
    /// **HD only.** The flyer cards `Grid Selection` draws behind its
    /// widgets - see [`oag_game::flyer`]. `None` on every other title.
    pub(crate) flyers: Option<oag_game::flyer::Flyers>,
    /// **HD only.** A circuit's emblem for `Cell Selection`, by lowercased id.
    circuit_emblems: std::collections::HashMap<String, String>,
}

impl CampaignStage {
    #[allow(
        clippy::too_many_arguments,
        reason = "one call site (`session::campaign::open_campaign`), each a separate fact the \
                  stage needs to hold open"
    )]
    pub(crate) fn new(
        grids: Vec<race_campaign::Grid>,
        grid_layout: oag_ui_screens::campaign::Layout,
        cell_layout: oag_ui_screens::campaign::Layout,
        selection_layout: Option<oag_ui_screens::campaign::Layout>,
        grid_layout_fury: Option<oag_ui_screens::campaign::Layout>,
        cell_help: Option<oag_ui_screens::campaign::Layout>,
        nav_legend: Option<oag_ui_screens::campaign::footer::NavigationLegend>,
        ticker: Option<oag_ui_screens::campaign::footer::TickerLayout>,
        strings: oag_ui::language::StringTable,
        sprites: oag_hud::sprite::Sheet,
        title_ref: &'static oag_title::Title,
        circuit_names: oag_ui::language::CircuitNames,
        records: oag_game::records::Store,
        flyers: Option<oag_game::flyer::Flyers>,
        circuit_emblems: std::collections::HashMap<String, String>,
        cell_cursor: CellCursor,
    ) -> Self {
        let title = title_ref.name.to_string();
        let grid_range = 0..grids.len();
        // `Campaign Selection` is HD's own screen ahead of `Grid
        // Selection` - only opened on it when both halves of the pair
        // actually read (see `Self::has_selection`'s own doc); every other
        // title, and an HD source missing `DATA06`'s own copy, opens
        // straight on `Grid Selection` over every grid, the pre-this-pass
        // behaviour.
        let screen = if selection_layout.is_some() && grid_layout_fury.is_some() {
            Screen::Selection(CampaignSelection::new())
        } else {
            let model = oag_ui_screens::campaign::GridSelection::new(
                grids
                    .iter()
                    .map(|grid| Self::grid_summary(grid, &title, &records))
                    .collect(),
            );
            Screen::Grid(model)
        };
        Self {
            grids,
            grid_layout,
            cell_layout,
            selection_layout,
            grid_layout_fury,
            grid_range,
            active_campaign: None,
            cell_help,
            nav_legend,
            ticker,
            ticker_elapsed: 0.0,
            strings,
            sprites,
            screen,
            notice: None,
            title,
            dialect: title_ref.campaign.dialect,
            circuit_names,
            records,
            cell_cursor,
            flyers,
            circuit_emblems,
        }
    }

    /// Whether the open grid is one of Fury's: its card is red and its page
    /// draws the text colours measured on that card.
    pub(crate) fn fury_open(&self) -> bool {
        self.active_campaign == Some(Campaign::Fury)
    }

    /// The sheet `src` of circuit `id`'s white emblem, when it has one.
    pub(crate) fn circuit_emblem(&self, id: &str) -> Option<String> {
        self.circuit_emblems.get(&id.to_lowercase()).cloned()
    }

    /// The flyer cards the current screen shows behind its widgets: the
    /// selected tier's on `Grid Selection`, its back on `Cell Selection`, both
    /// campaigns' on `Campaign Selection`, none on a tier that names none.
    pub(crate) fn flyer_shows(&self) -> Vec<oag_game::flyer::Show> {
        let Some(flyers) = &self.flyers else {
            return Vec::new();
        };
        match &self.screen {
            Screen::Grid(model) => model
                .selected()
                .and_then(|grid| grid.flyer_name.as_deref())
                .map(oag_game::flyer::Flyers::grid_show)
                .into_iter()
                .collect(),
            Screen::Selection(model) => flyers.selection_shows(model.selected()),
            Screen::Cell { which, .. } => self
                .grids
                .get(*which)
                .and_then(|grid| grid.flyer_name.as_deref())
                .map(oag_game::flyer::Flyers::cell_show)
                .into_iter()
                .collect(),
        }
    }

    /// The selected card's rectangle on the HD grid, for the pointer - `None`
    /// when no card is drawn, which leaves the pointer on its fallback rect.
    pub(crate) fn flyer_card_rect(&self) -> Option<[f32; 4]> {
        let show = self.flyer_shows().into_iter().next()?;
        self.flyers
            .as_ref()?
            .card_rect(&show, oag_display::space::Space::HD)
    }

    /// Advances the ticker's own clock - called once a frame from
    /// `MenuStage::tick`, the same place `crate::marquee::Timer` (its
    /// sibling clock for a menu row's overflowing value) is driven from.
    pub(crate) fn tick_ticker(&mut self, dt: f32) {
        self.ticker_elapsed += dt.max(0.0);
    }

    /// The `Confirm`/`Back` legend's own draw list, or empty when this
    /// source's `Skin.xml` carried none - see
    /// `oag_ui_screens::campaign::footer::NavigationLegend::draw`, and [`Self::notice`]
    /// when there is one.
    #[must_use]
    pub(crate) fn nav_legend_draw(
        &self,
        faces: &oag_ui_screens::picker::FaceScales,
        measure: &dyn Fn(&str) -> f32,
    ) -> Vec<oag_ui::frontend::Draw> {
        let Some(legend) = self.nav_legend.as_ref() else {
            return Vec::new();
        };
        let mut draws = legend.draw(faces, measure);
        draws.extend(
            self.notice
                .as_deref()
                .and_then(|text| legend.notice(text, faces)),
        );
        draws
    }

    /// The ticker's own draw at its current clock - `None` when this source
    /// authors no ticker at all, nothing is honestly known to rotate
    /// through yet ([`Self::ticker_tips`]), or the clock is between two
    /// tips (see `oag_ui_screens::campaign::footer::ticker_draw`'s own doc).
    #[must_use]
    pub(crate) fn ticker_draw(
        &self,
        faces: &oag_ui_screens::picker::FaceScales,
        measure: &dyn Fn(&str) -> f32,
    ) -> Option<oag_ui::frontend::Draw> {
        oag_ui_screens::campaign::footer::ticker_draw(
            self.ticker.as_ref()?,
            self.ticker_elapsed,
            &self.ticker_tips(),
            faces,
            measure,
        )
    }

    /// The ticker's own clip window, `(left, right)` in screen space - what
    /// a caller needs to build `Renderer::render_with`'s `clip` tuple once
    /// it has found [`Self::ticker_draw`]'s own index in the flattened draw
    /// list. `None` on a source with no ticker at all.
    #[must_use]
    pub(crate) fn ticker_clip_bounds(&self) -> Option<(f32, f32)> {
        let [x, _, width, _] = self.ticker.as_ref()?.viewport;
        Some((x, x + width))
    }

    /// Which tip strings the ticker honestly has to show - see
    /// `oag_game::records::ticker_tips`'s own doc for the `TKR_NO*` rotation
    /// and why it is one shared function rather than three copies:
    /// `MenuStage`'s own ticker (`crate::main::menu_stage`) and
    /// `capture::menu_page`'s `--menu-page` still both read the identical
    /// list off the identical save file.
    #[must_use]
    fn ticker_tips(&self) -> Vec<String> {
        oag_game::records::ticker_tips(&self.strings, &self.records)
    }

    /// Whether this is Wipeout HD/Fury's own campaign - what
    /// `MenuStage::render`/`session::pointer::campaign_pointer` dispatch
    /// draw/pointer functions on. See [`Self::title`]'s own doc.
    #[must_use]
    pub(crate) fn is_hd(&self) -> bool {
        self.dialect.draws_hd_screens()
    }

    /// Whether `Campaign Selection` was read at all - see
    /// [`oag_game::campaign::Campaign::selection_layout`]'s own doc for the
    /// one case (a source missing `DATA06`'s own copy of the screen) where
    /// this is `false` on an otherwise-HD source.
    #[must_use]
    pub(crate) fn has_selection(&self) -> bool {
        self.selection_layout.is_some() && self.grid_layout_fury.is_some()
    }

    #[must_use]
    pub(crate) fn selection_layout(&self) -> Option<&oag_ui_screens::campaign::Layout> {
        self.selection_layout.as_ref()
    }

    #[must_use]
    pub(crate) fn circuit_names(&self) -> &oag_ui::language::CircuitNames {
        &self.circuit_names
    }

    fn grid_summary(
        grid: &race_campaign::Grid,
        title: &str,
        records: &oag_game::records::Store,
    ) -> oag_ui_screens::campaign::GridSummary {
        oag_ui_screens::campaign::GridSummary::from_grid_with_medals(grid, &|cell_name| {
            records
                .campaign_medal(title, cell_name)?
                .best_medal
                .map(to_campaign_medal)
        })
    }

    /// Every grid this campaign carries - `crate::main::session::endrace`'s
    /// own `RETURN TO GRID` reads this to find which one a raced cell
    /// belongs to.
    #[must_use]
    pub(crate) fn grids(&self) -> &[race_campaign::Grid] {
        &self.grids
    }

    /// Whether the grid at absolute index `which` (into [`Self::grids`]) is
    /// unlocked - what `crate::main::session::campaign::handle_campaign`
    /// gates `Grid Selection`'s own Confirm on.
    ///
    /// **The recovered law, not `GridSelection::tier_shows_lock`'s own
    /// display shortcut.** That glyph rule always compares against the
    /// immediately preceding tile; this instead reimplements
    /// `Unlock_GridPointsMet` (`oag_tables::race_campaign::grid_points_met`)
    /// against the grid's own authored `<Unlock Grid="...">` name, which
    /// only happens to be the previous tile on all sixteen shipped grids -
    /// see `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "Grid0..
    /// Grid14: what the gate actually evaluates" and "the tier unlock rule"
    /// sections for why the two predicates cannot disagree on shipped data.
    /// `grid.points_earned(medal_of) > 0` is included alongside `!grid.locked`
    /// for the identical reason `GridSelection_PopulateTiles` clears the
    /// glyph unconditionally once a grid has scored any points of its own.
    /// A grid with no `<Unlock Grid="...">` row at all (`grid0`, the one
    /// grid shipped `Locked="false"`) is always unlocked.
    #[must_use]
    pub(crate) fn grid_is_unlocked(&self, which: usize) -> bool {
        let Some(grid) = self.grids.get(which) else {
            return false;
        };
        let medal_of = campaign_medal_of(&self.records, &self.title);
        if !grid.locked || grid.points_earned(&medal_of) > 0 {
            return true;
        }
        match grid.unlock_grid.as_deref() {
            Some(target) => race_campaign::grid_points_met(&self.grids, target, &medal_of),
            None => true,
        }
    }

    /// Each campaign's own `(earned, total)` gold-medal pair, for `Campaign
    /// Selection`'s own `NumMedalsTextFury`/`NumMedalsTextHD` -
    /// `(fury, hd)`. `earned` is `0` on a fresh profile, the same
    /// "player-progress source is optional" reading [`Self::grid_summary`]
    /// already gives every other number on these screens. `total` is the
    /// denominator RPCS3's own `Campaign Selection` shows next to it -
    /// `"0 / 87"` for `Wipeout HD`, `"0 / 80"` for `Fury`
    /// (`data/scratch/lane-hd-sel/rpcs3-campaign-selection/01-right-tap.png`/
    /// `01-left-tap.png`) - derived, not guessed: a cell on either campaign
    /// always carries exactly one `<Gold>` target of its own (`PI_Cell`'s
    /// own `<Gold Target=...>` child, present on every cell in every grid
    /// measured), so the total gold medals a campaign can ever earn is
    /// exactly its own cell count, `grid.cells.len()` summed across the
    /// campaign's own eight grids: `87` for `DATA02`'s own `grid_00`..`grid_07`
    /// (`6+8+10+10+10+12+14+17`), `80` for `DATA00`'s own `grid_08`..`grid_15`,
    /// both matching RPCS3 exactly. See `docs/ui/campaign-screens.md`'s
    /// "Wipeout HD/Fury: the footer's button glyphs, and the `GOLD MEDALS`
    /// denominator" section for the full count and the `grid_04.xml` defect
    /// closing it also surfaced and fixed.
    #[must_use]
    pub(crate) fn campaign_gold_medals(&self) -> ((u32, u32), (u32, u32)) {
        let pair = |range: std::ops::Range<usize>| -> (u32, u32) {
            let grids = self.grids.get(range).unwrap_or(&[]);
            let earned = grids
                .iter()
                .map(|grid| Self::grid_summary(grid, &self.title, &self.records).gold_medals)
                .sum();
            let total = grids
                .iter()
                .map(|grid| u32::try_from(grid.cells.len()).unwrap_or(u32::MAX))
                .sum();
            (earned, total)
        };
        (
            pair(oag_hd::campaign::FURY_GRID_RANGE),
            pair(oag_hd::campaign::HD_GRID_RANGE),
        )
    }

    /// [`Self::grid_layout`]'s own choice between the base layout and
    /// [`Self::grid_layout_fury`] - the base one whenever
    /// [`Self::active_campaign`] is not `Fury`, which covers both "no
    /// campaign chosen yet" (every non-HD title, and `Screen::Selection`
    /// itself never reads this) and "the base `Wipeout HD` campaign was
    /// chosen".
    #[must_use]
    pub(crate) fn grid_layout(&self) -> &oag_ui_screens::campaign::Layout {
        if self.active_campaign == Some(Campaign::Fury) {
            self.grid_layout_fury.as_ref().unwrap_or(&self.grid_layout)
        } else {
            &self.grid_layout
        }
    }

    #[must_use]
    pub(crate) fn cell_layout(&self) -> &oag_ui_screens::campaign::Layout {
        &self.cell_layout
    }

    /// **HD only.** Opens `campaign`'s own `Grid Selection`/`Grid Selection
    /// Fury`, narrowing [`Self::grid_range`] to that campaign's own eight
    /// grids - `Campaign Selection`'s own `Event::Confirmed`.
    pub(crate) fn open_grid_selection(&mut self, campaign: Campaign) {
        self.grid_range = campaign.grid_range();
        self.active_campaign = Some(campaign);
        // `with_per_page(1)` - HD/Fury's own `Grid Selection` pages one
        // flyer at a time, not Pulse's four-hex page. See
        // `oag_ui_screens::campaign::GridSelection::per_page`'s own doc.
        let model = oag_ui_screens::campaign::GridSelection::new(
            self.grids
                .get(self.grid_range.clone())
                .unwrap_or(&[])
                .iter()
                .map(|grid| Self::grid_summary(grid, &self.title, &self.records))
                .collect(),
        )
        .with_per_page(1);
        self.screen = Screen::Grid(model);
    }

    /// **HD only.** Returns to `Campaign Selection` - `Grid Selection`'s own
    /// `Event::Back`, when [`Self::has_selection`]. Restores whichever
    /// campaign [`Self::active_campaign`] already names, rather than
    /// resetting to the screen's own default `Fury` entry, so backing out of
    /// a chosen `Wipeout HD` lands on `Campaign Selection` still showing
    /// `Wipeout HD` selected.
    pub(crate) fn open_selection(&mut self) {
        let model = match self.active_campaign {
            Some(campaign) => CampaignSelection::at(campaign),
            None => CampaignSelection::new(),
        };
        self.screen = Screen::Selection(model);
    }

    #[must_use]
    pub(crate) fn cell_help_layout(&self) -> Option<&oag_ui_screens::campaign::Layout> {
        self.cell_help.as_ref()
    }

    /// Opens `Cell Selection` on `which`'s own cells - an **absolute**
    /// index into [`Self::grids`], the same contract this method has always
    /// had (`crate::main::session::endrace::return_to_campaign` reaches a
    /// cell this way, off `Self::grids()`'s own flat list, without ever
    /// touching `Campaign Selection` first). `false` when `which` is out of
    /// range or the grid carries no cells at all, and the caller stays on
    /// whatever screen it was on.
    ///
    /// **Self-healing for `Self::grid_range`/`Self::active_campaign`**: a
    /// caller that reopens `Cell Selection` directly - `return_to_campaign`
    /// is the one that exists - never confirms `Campaign Selection`/`Grid
    /// Selection` first, so nothing else would narrow either field to the
    /// campaign `which` actually belongs to. Narrowing it here, on every
    /// call, keeps [`Self::cell_grid_summary`]'s own `Event NN/MM` counter
    /// and [`Self::back_to_grid_selection`]'s own return page correct
    /// regardless of which caller reached this cell - see
    /// [`Self::open_cell_selection_at_grid_slot`] for the other one, whose
    /// own index is already relative to a narrowed range.
    pub(crate) fn open_cell_selection(&mut self, which: usize) -> bool {
        let Some(grid) = self.grids.get(which) else {
            return false;
        };
        if grid.cells.is_empty() {
            return false;
        }
        if self.has_selection() {
            let campaign = if oag_hd::campaign::FURY_GRID_RANGE.contains(&which) {
                Campaign::Fury
            } else {
                Campaign::Hd
            };
            self.grid_range = campaign.grid_range();
            self.active_campaign = Some(campaign);
        }
        let cells = grid.cells.clone();
        let title = &self.title;
        let records = &self.records;
        let by_name = cells.clone();
        let mut model = oag_ui_screens::campaign::CellSelection::with_medals_and_records(
            cells,
            &|name| {
                records
                    .campaign_medal(title, name)?
                    .best_medal
                    .map(to_campaign_medal)
            },
            &|name| Self::saved_record_centiseconds(&by_name, records, title, name),
        )
        .with_difficulty(&|name| {
            records
                .campaign_medal(title, name)?
                .best_difficulty
                .map(to_campaign_difficulty)
        });
        // HD/Fury's own fresh-profile default rung is `Easy`, not Pulse's
        // `Medium` - see `oag_ui_screens::campaign::CellSelection::difficulty`'s own
        // doc for the RPCS3 measurement. This only sets the *starting*
        // browsed rung; a `records.campaign_medal` hit for the currently
        // selected cell (not modelled here at all - this is the initial
        // arrival value, not per-cell) would still need its own reading, the
        // same way `with_difficulty` above is per-cell already.
        if self.is_hd() {
            model = model.with_default_difficulty(race_campaign::Difficulty::Easy);
        }
        self.cell_cursor.restore(&mut model);
        self.screen = Screen::Cell { model, which };
        true
    }

    /// `Line5`'s own value - `Cell_SavedRecord`, in centiseconds. Read off
    /// the general per-track/mode/class [`oag_game::records::Store`] rather
    /// than a per-cell store this build does not keep (see
    /// [`oag_ui_screens::campaign::CellSelection::records`]'s own doc): a campaign
    /// race already folds into that same table, keyed by the cell's own
    /// track/mode/class, so it is the closest available reading of "this
    /// cell's own saved best" without inventing a new store. **Time Trial**
    /// reads `best_total_ticks` (a finished race's own total time, the
    /// quantity `docs/ui/campaign-screens.md`'s own live playthrough found
    /// under `Campaign record` after a first attempt); **Speed Lap** reads
    /// `best_lap_ticks` (its only meaningful quantity - it never finishes).
    /// Every other mode answers `None`: `Race`'s own record is a position,
    /// not a time, and `Zone`/`Elimination` have no raw count anywhere in
    /// [`oag_game::records::Record`] to read at all.
    fn saved_record_centiseconds(
        cells: &[race_campaign::Cell],
        records: &oag_game::records::Store,
        title: &str,
        cell_name: &str,
    ) -> Option<i64> {
        let cell = cells.iter().find(|cell| cell.name == cell_name)?;
        let mode = oag_game::campaign::race_mode_for_cell(cell.mode.clone())?;
        let key =
            oag_game::records::Key::new(title, cell.track.as_deref(), mode.name(), &cell.class);
        let record = records.get(&key)?;
        let ticks = match mode {
            oag_race::Mode::TimeTrial => record.best_total_ticks,
            oag_race::Mode::SpeedLap => record.best_lap_ticks.map(u64::from),
            _ => None,
        }?;
        Some(i64::try_from(ticks * 100 / 60).unwrap_or(i64::MAX))
    }

    /// [`Self::open_cell_selection`], for `slot` relative to
    /// [`Self::grid_range`] - what `Grid Selection`'s own
    /// `GridSelection::index()` returns, the call site
    /// `crate::main::session::campaign::handle_campaign`'s own `Screen::Grid`
    /// confirm arm has.
    pub(crate) fn open_cell_selection_at_grid_slot(&mut self, slot: usize) -> bool {
        match self.grid_range.start.checked_add(slot) {
            Some(absolute) => self.open_cell_selection(absolute),
            None => false,
        }
    }

    /// **HD only** - the enclosing grid's own index and count, both relative
    /// to [`Self::grid_range`] (so `Cell Selection`'s own `EventNum`/
    /// `GridNum` reads `"Event 01/08"` on a Fury cell, not `"Event
    /// 09/16"`), plus its [`oag_ui_screens::campaign::GridSummary`] and the next grid's `FlyerName`, for
    /// `oag_ui_screens::campaign::hd::hd_cell_draw_list`'s own counters. `None` off
    /// `Grid Selection`/`Campaign Selection`, since there is no "enclosing
    /// grid" there.
    #[must_use]
    pub(crate) fn cell_grid_summary(
        &self,
    ) -> Option<(usize, usize, oag_ui_screens::campaign::GridSummary, Option<&str>)> {
        let Screen::Cell { which, .. } = &self.screen else {
            return None;
        };
        let grid = self.grids.get(*which)?;
        let next = (which + 1 < self.grid_range.end)
            .then(|| self.grids.get(which + 1))
            .flatten()
            .and_then(|grid| grid.flyer_name.as_deref());
        Some((
            which.saturating_sub(self.grid_range.start),
            self.grid_range.len(),
            Self::grid_summary(grid, &self.title, &self.records),
            next,
        ))
    }

    /// The cursor as it stands as the campaign closes, for the session to
    /// carry to the next `RACE CAMPAIGN` (measured: the original's survives
    /// leaving the campaign, see [`CellCursor`]).
    pub(crate) fn cursor_on_close(&self) -> CellCursor {
        let mut cursor = self.cell_cursor;
        if let Screen::Cell { model, .. } = &self.screen {
            cursor.remember(model);
        }
        cursor
    }

    /// Returns to `Grid Selection`, on the tier `Cell Selection` was opened
    /// from.
    pub(crate) fn back_to_grid_selection(&mut self) {
        if let Screen::Cell { model, .. } = &self.screen {
            self.cell_cursor.remember(model);
        }
        let index = match &self.screen {
            Screen::Cell { which, .. } => which.saturating_sub(self.grid_range.start),
            Screen::Grid(model) => model.index(),
            // `Cell Selection`'s own `DefaultPrevious` never names
            // `Campaign Selection`, so this arm is unreached in practice -
            // kept exhaustive rather than panicking on a screen shape this
            // function was never meant to see.
            Screen::Selection(_) => 0,
        };
        // Shared by every title - [`Self::has_selection`] is the same "did
        // `Campaign Selection` read" discriminator [`Self::open_grid_selection`]'s
        // own doc and `crate::main::session::campaign::handle_campaign`'s
        // `Back` routing already use, so this rebuild pages the same way
        // the screen it is returning to just did.
        let mut model = oag_ui_screens::campaign::GridSelection::new(
            self.grids
                .get(self.grid_range.clone())
                .unwrap_or(&[])
                .iter()
                .map(|grid| Self::grid_summary(grid, &self.title, &self.records))
                .collect(),
        );
        if self.has_selection() {
            model = model.with_per_page(1);
        }
        model.set_index(index);
        self.screen = Screen::Grid(model);
    }
}

#[cfg(test)]
mod tests {
    use super::CellCursor;
    use oag_tables::race_campaign::{Cell, Mode};
    use oag_ui_screens::campaign::CellSelection;

    fn cell(name: &str) -> Cell {
        Cell {
            name: name.to_string(),
            track: Some("track".to_string()),
            mode: Mode::Race,
            class: "Venom".to_string(),
            weapons: true,
            damage: true,
            locked: Some(false),
            status: None,
            ai_count: Some(7),
            skill: None,
            skill_easy: None,
            skill_hard: None,
            laps: Some(3),
            ship: None,
            ship_choice: Some(true),
            gold: 1,
            silver: 2,
            bronze: 3,
            tournament_tracks: Vec::new(),
            difficulty_targets: None,
            nitro_elimination_targets: None,
        }
    }

    fn selected(model: &CellSelection) -> &str {
        model.selected().map_or("", |cell| cell.name.as_str())
    }

    fn locked_cell(name: &str) -> Cell {
        Cell {
            locked: None,
            ..cell(name)
        }
    }

    /// The measured cross-grid case: the slot a cursor was left on in one
    /// grid is where it lands in another, not on the new grid's default and
    /// not on the cell that other grid was last left on.
    #[test]
    fn the_cursor_is_one_slot_shared_by_every_grid() {
        let grid0 = vec![cell("grid0_3_1"), cell("grid0_3_2")];
        let grid1 = vec![cell("grid1_3_1"), cell("grid1_3_2")];
        let mut cursor = CellCursor::default();

        let mut first = CellSelection::new(grid0.clone());
        first.select_by_name("grid0_3_2");
        cursor.remember(&first);

        let mut other = CellSelection::new(grid1.clone());
        assert_eq!(selected(&other), "grid1_3_1");
        cursor.restore(&mut other);
        assert_eq!(selected(&other), "grid1_3_2");

        other.select_by_name("grid1_3_1");
        cursor.remember(&other);
        let mut back = CellSelection::new(grid0);
        cursor.restore(&mut back);
        assert_eq!(selected(&back), "grid0_3_1");
    }

    /// A slot whose cell is locked, or that the grid does not have, falls to
    /// the first-visit default (`grid0_2_2` into `grid1`, measured).
    #[test]
    fn a_locked_or_absent_slot_keeps_the_default() {
        let mut cursor = CellCursor::default();
        let mut first = CellSelection::new(vec![cell("grid0_3_1"), cell("grid0_2_2")]);
        first.select_by_name("grid0_2_2");
        cursor.remember(&first);

        let mut locked = CellSelection::new(vec![cell("grid1_3_1"), locked_cell("grid1_2_2")]);
        cursor.restore(&mut locked);
        assert_eq!(selected(&locked), "grid1_3_1");

        let mut absent = CellSelection::new(vec![cell("grid1_3_1"), cell("grid1_3_2")]);
        cursor.restore(&mut absent);
        assert_eq!(selected(&absent), "grid1_3_1");
    }

    /// A cell whose `Locked` byte is still set but whose glyph a medalled hex
    /// neighbour hides keeps the cursor (`grid0_2_2` beside the gold
    /// `grid0_3_2`, measured); one with no medal near it does not (`grid0_2_1`).
    #[test]
    fn a_cell_a_neighbouring_medal_unlocked_keeps_the_cursor() {
        let cells = vec![
            cell("grid0_3_1"),
            cell("grid0_3_2"),
            locked_cell("grid0_2_2"),
            locked_cell("grid0_2_1"),
        ];
        let with_gold = |cells: Vec<Cell>| {
            CellSelection::with_medals(cells, &|name| {
                (name == "grid0_3_2").then_some(oag_tables::race_campaign::Medal::Gold)
            })
        };
        let mut cursor = CellCursor::default();

        let mut left = with_gold(cells.clone());
        left.select_by_name("grid0_2_2");
        cursor.remember(&left);
        let mut again = with_gold(cells.clone());
        cursor.restore(&mut again);
        assert_eq!(selected(&again), "grid0_2_2");

        left.select_by_name("grid0_2_1");
        cursor.remember(&left);
        let mut again = with_gold(cells);
        cursor.restore(&mut again);
        assert_eq!(selected(&again), "grid0_3_1");
    }

    /// Nothing remembered yet is the first-visit default, untouched.
    #[test]
    fn a_fresh_cursor_changes_nothing() {
        let mut model = CellSelection::new(vec![locked_cell("grid0_2_1"), cell("grid0_3_1")]);
        CellCursor::default().restore(&mut model);
        assert_eq!(selected(&model), "grid0_3_1");
    }
}
