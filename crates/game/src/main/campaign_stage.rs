//! The Race Campaign's two screens, over the menu stage - the same shape
//! [`crate::picker_stage`] holds the race box's two in. See
//! `oag_ui::campaign` for the model and the drawing, and
//! `crate::session::campaign` for the flow that opens and closes one.

use oag_tables::race_campaign;

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

/// One open campaign screen: `Grid Selection`, or `Cell Selection` over one
/// of its tiers.
pub(crate) enum Screen {
    Grid(oag_ui::campaign::GridSelection),
    /// `which` is the [`CampaignStage::grids`] index the player drilled into -
    /// what `Event::Back` on `Cell Selection` returns `Grid Selection` to.
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
    /// `oag_game::records::Key::new`'s own `title` takes.
    title: String,
    /// A snapshot of the player's own saved progress, taken once when the
    /// campaign opens - the same "read once on open" choice [`Self::grids`]
    /// already makes, and safe for the same reason: nothing mutates
    /// `Session::records` while a campaign screen is open, only a finished
    /// or escaped race does, and reaching one closes this stage first.
    records: oag_game::records::Store,
}

impl CampaignStage {
    pub(crate) fn new(
        grids: Vec<race_campaign::Grid>,
        grid_layout: oag_ui::campaign::Layout,
        cell_layout: oag_ui::campaign::Layout,
        strings: oag_ui::language::StringTable,
        sprites: oag_game::sprite::Sheet,
        title: String,
        records: oag_game::records::Store,
    ) -> Self {
        let model = oag_ui::campaign::GridSelection::new(
            grids
                .iter()
                .map(|grid| Self::grid_summary(grid, &title, &records))
                .collect(),
        );
        Self {
            grids,
            grid_layout,
            cell_layout,
            strings,
            sprites,
            screen: Screen::Grid(model),
            title,
            records,
        }
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

    #[must_use]
    pub(crate) fn grid_layout(&self) -> &oag_ui::campaign::Layout {
        &self.grid_layout
    }

    #[must_use]
    pub(crate) fn cell_layout(&self) -> &oag_ui::campaign::Layout {
        &self.cell_layout
    }

    /// Opens `Cell Selection` on `which`'s own cells. `false` when `which`
    /// is out of range or the grid carries no cells at all, and the caller
    /// stays on `Grid Selection`.
    pub(crate) fn open_cell_selection(&mut self, which: usize) -> bool {
        let Some(grid) = self.grids.get(which) else {
            return false;
        };
        if grid.cells.is_empty() {
            return false;
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

    /// Returns to `Grid Selection`, on the tier `Cell Selection` was opened
    /// from.
    pub(crate) fn back_to_grid_selection(&mut self) {
        let index = match &self.screen {
            Screen::Cell { which, .. } => *which,
            Screen::Grid(model) => model.index(),
        };
        let mut model = oag_ui::campaign::GridSelection::new(
            self.grids
                .iter()
                .map(|grid| Self::grid_summary(grid, &self.title, &self.records))
                .collect(),
        );
        model.set_index(index);
        self.screen = Screen::Grid(model);
    }
}
