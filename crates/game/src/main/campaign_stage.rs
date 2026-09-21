//! The Race Campaign's screens, over the menu stage - the same shape
//! [`crate::picker_stage`] holds the race box's two in. See
//! `oag_ui::campaign` for the model and the drawing, and
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
use oag_ui::campaign::selection::{Campaign, CampaignSelection};

/// `oag_game::records::Medal` restated as `oag_tables::race_campaign::Medal`.
/// The two are duplicated on purpose, not shared, per `records.rs`'s own
/// module doc; this is the one conversion this file needs, in one place.
fn to_campaign_medal(medal: oag_game::records::Medal) -> race_campaign::Medal {
    match medal {
        oag_game::records::Medal::Gold => race_campaign::Medal::Gold,
        oag_game::records::Medal::Silver => race_campaign::Medal::Silver,
        oag_game::records::Medal::Bronze => race_campaign::Medal::Bronze,
    }
}

/// One open campaign screen: `Campaign Selection` (HD only), `Grid
/// Selection`, or `Cell Selection` over one of its tiers.
pub(crate) enum Screen {
    /// **HD only** - see the module doc. Never built for any other title.
    Selection(CampaignSelection),
    Grid(oag_ui::campaign::GridSelection),
    /// `which` is the absolute [`CampaignStage::grids`] index the player
    /// drilled into - what `Event::Back` on `Cell Selection` returns `Grid
    /// Selection` to. Absolute, not relative to
    /// [`CampaignStage::grid_range`], so `CampaignStage::grids().get(which)`
    /// always works regardless of which campaign (if any) is open.
    Cell {
        model: oag_ui::campaign::CellSelection,
        which: usize,
    },
}

/// The whole campaign, read once when it opens.
///
/// **Every grid, not only the selected one.** `Grid Selection`'s own honey
/// counter and page need the full sixteen up front, and confirming a tier
/// needs that tier's own cells - reading sixteen small XML files once on
/// open is cheaper than reopening the disc archive on every confirm.
pub(crate) struct CampaignStage {
    grids: Vec<race_campaign::Grid>,
    grid_layout: oag_ui::campaign::Layout,
    cell_layout: oag_ui::campaign::Layout,
    /// **HD only**, both `Some` or both `None` together - see
    /// `oag_game::campaign::Campaign`'s own field docs for why they can be
    /// absent even on an HD source.
    selection_layout: Option<oag_ui::campaign::Layout>,
    grid_layout_fury: Option<oag_ui::campaign::Layout>,
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
    pub(crate) sprites: oag_game::sprite::Sheet,
    pub(crate) screen: Screen,
    /// [`oag_title::Title::name`], for [`Self::medal_of`] - the same string
    /// `oag_game::records::Key::new`'s own `title` takes. Also what
    /// `crate::main::menu_stage::MenuStage::render` and
    /// `crate::main::session::pointer::campaign_pointer` dispatch on to
    /// pick between Pulse's draw/pointer functions and
    /// [`oag_ui::campaign::hd`]'s own, since both screens share one name.
    title: String,
    /// **HD only** - Pulse's own `Track Line` never needs this fold (see
    /// `oag_ui::campaign::draw::track_line`'s own doc), so this is
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
}

impl CampaignStage {
    #[allow(
        clippy::too_many_arguments,
        reason = "one call site (`session::campaign::open_campaign`), each a separate fact the \
                  stage needs to hold open"
    )]
    pub(crate) fn new(
        grids: Vec<race_campaign::Grid>,
        grid_layout: oag_ui::campaign::Layout,
        cell_layout: oag_ui::campaign::Layout,
        selection_layout: Option<oag_ui::campaign::Layout>,
        grid_layout_fury: Option<oag_ui::campaign::Layout>,
        strings: oag_ui::language::StringTable,
        sprites: oag_game::sprite::Sheet,
        title: String,
        circuit_names: oag_ui::language::CircuitNames,
        records: oag_game::records::Store,
    ) -> Self {
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
            let model = oag_ui::campaign::GridSelection::new(
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
            strings,
            sprites,
            screen,
            title,
            circuit_names,
            records,
        }
    }

    /// Whether this is Wipeout HD/Fury's own campaign - what
    /// `MenuStage::render`/`session::pointer::campaign_pointer` dispatch
    /// draw/pointer functions on. See [`Self::title`]'s own doc.
    #[must_use]
    pub(crate) fn is_hd(&self) -> bool {
        self.title == oag_hd::TITLE.name
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
    pub(crate) fn selection_layout(&self) -> Option<&oag_ui::campaign::Layout> {
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
    ) -> oag_ui::campaign::GridSummary {
        oag_ui::campaign::GridSummary::from_grid_with_medals(grid, &|cell_name| {
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

    /// Each campaign's own earned-gold-medal count, for `Campaign
    /// Selection`'s own `NumMedalsTextFury`/`NumMedalsTextHD` -
    /// `(fury, hd)`. `0` on a fresh profile, the same "player-progress
    /// source is optional" reading [`Self::grid_summary`] already gives
    /// every other number on these screens.
    #[must_use]
    pub(crate) fn campaign_gold_medals(&self) -> (u32, u32) {
        let sum = |range: std::ops::Range<usize>| {
            self.grids
                .get(range)
                .unwrap_or(&[])
                .iter()
                .map(|grid| Self::grid_summary(grid, &self.title, &self.records).gold_medals)
                .sum()
        };
        (
            sum(oag_hd::campaign::FURY_GRID_RANGE),
            sum(oag_hd::campaign::HD_GRID_RANGE),
        )
    }

    /// [`Self::grid_layout`]'s own choice between the base layout and
    /// [`Self::grid_layout_fury`] - the base one whenever
    /// [`Self::active_campaign`] is not `Fury`, which covers both "no
    /// campaign chosen yet" (every non-HD title, and `Screen::Selection`
    /// itself never reads this) and "the base `Wipeout HD` campaign was
    /// chosen".
    #[must_use]
    pub(crate) fn grid_layout(&self) -> &oag_ui::campaign::Layout {
        if self.active_campaign == Some(Campaign::Fury) {
            self.grid_layout_fury.as_ref().unwrap_or(&self.grid_layout)
        } else {
            &self.grid_layout
        }
    }

    #[must_use]
    pub(crate) fn cell_layout(&self) -> &oag_ui::campaign::Layout {
        &self.cell_layout
    }

    /// **HD only.** Opens `campaign`'s own `Grid Selection`/`Grid Selection
    /// Fury`, narrowing [`Self::grid_range`] to that campaign's own eight
    /// grids - `Campaign Selection`'s own `Event::Confirmed`.
    pub(crate) fn open_grid_selection(&mut self, campaign: Campaign) {
        self.grid_range = campaign.grid_range();
        self.active_campaign = Some(campaign);
        let model = oag_ui::campaign::GridSelection::new(
            self.grids
                .get(self.grid_range.clone())
                .unwrap_or(&[])
                .iter()
                .map(|grid| Self::grid_summary(grid, &self.title, &self.records))
                .collect(),
        );
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
        self.screen = Screen::Cell {
            model: oag_ui::campaign::CellSelection::with_medals(cells, &|name| {
                records
                    .campaign_medal(title, name)?
                    .best_medal
                    .map(to_campaign_medal)
            }),
            which,
        };
        true
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
    /// 09/16"`), plus its [`oag_ui::campaign::GridSummary`], for
    /// `oag_ui::campaign::hd::hd_cell_draw_list`'s own counters. `None` off
    /// `Grid Selection`/`Campaign Selection`, since there is no "enclosing
    /// grid" there.
    #[must_use]
    pub(crate) fn cell_grid_summary(
        &self,
    ) -> Option<(usize, usize, oag_ui::campaign::GridSummary)> {
        let Screen::Cell { which, .. } = &self.screen else {
            return None;
        };
        let grid = self.grids.get(*which)?;
        Some((
            which.saturating_sub(self.grid_range.start),
            self.grid_range.len(),
            Self::grid_summary(grid, &self.title, &self.records),
        ))
    }

    /// Returns to `Grid Selection`, on the tier `Cell Selection` was opened
    /// from.
    pub(crate) fn back_to_grid_selection(&mut self) {
        let index = match &self.screen {
            Screen::Cell { which, .. } => which.saturating_sub(self.grid_range.start),
            Screen::Grid(model) => model.index(),
            // `Cell Selection`'s own `DefaultPrevious` never names
            // `Campaign Selection`, so this arm is unreached in practice -
            // kept exhaustive rather than panicking on a screen shape this
            // function was never meant to see.
            Screen::Selection(_) => 0,
        };
        let mut model = oag_ui::campaign::GridSelection::new(
            self.grids
                .get(self.grid_range.clone())
                .unwrap_or(&[])
                .iter()
                .map(|grid| Self::grid_summary(grid, &self.title, &self.records))
                .collect(),
        );
        model.set_index(index);
        self.screen = Screen::Grid(model);
    }
}
