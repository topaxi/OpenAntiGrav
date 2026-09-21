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
    /// `Cell Selection`'s own `Cell Help` overlay - see
    /// `oag_game::campaign::Campaign::cell_help`'s own doc. Drawn as a
    /// static (non-scrolling) panel while `CellSelection::help_open`, per
    /// `docs/ui/campaign-screens.md`'s `Open` entry on why not the
    /// authored `Viewport`/`Animation` scroll timeline.
    cell_help: Option<oag_ui::campaign::Layout>,
    /// The front-end root's own `Confirm`/`Back` legend - `None` on a
    /// source whose `Skin.xml` this pass could not read at all (logged when
    /// that happens, see `oag_game::campaign::read_footer`).
    nav_legend: Option<oag_ui::campaign::footer::NavigationLegend>,
    /// The front-end root's own scrolling tip ticker layout - content is
    /// supplied per frame by [`Self::ticker_tips`], not carried here.
    ticker: Option<oag_ui::campaign::footer::TickerLayout>,
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
        cell_help: Option<oag_ui::campaign::Layout>,
        nav_legend: Option<oag_ui::campaign::footer::NavigationLegend>,
        ticker: Option<oag_ui::campaign::footer::TickerLayout>,
        strings: oag_ui::language::StringTable,
        sprites: oag_game::sprite::Sheet,
        title: String,
        circuit_names: oag_ui::language::CircuitNames,
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
            cell_help,
            nav_legend,
            ticker,
            ticker_elapsed: 0.0,
            strings,
            sprites,
            screen: Screen::Grid(model),
            title,
            circuit_names,
            records,
        }
    }

    /// Advances the ticker's own clock - called once a frame from
    /// `MenuStage::tick`, the same place `crate::marquee::Timer` (its
    /// sibling clock for a menu row's overflowing value) is driven from.
    pub(crate) fn tick_ticker(&mut self, dt: f32) {
        self.ticker_elapsed += dt.max(0.0);
    }

    /// The `Confirm`/`Back` legend's own draw list, or empty when this
    /// source's `Skin.xml` carried none - see
    /// `oag_ui::campaign::footer::NavigationLegend::draw`.
    #[must_use]
    pub(crate) fn nav_legend_draw(
        &self,
        faces: &oag_ui::picker::FaceScales,
        measure: &dyn Fn(&str) -> f32,
    ) -> Vec<oag_ui::frontend::Draw> {
        self.nav_legend
            .as_ref()
            .map_or_else(Vec::new, |legend| legend.draw(faces, measure))
    }

    /// The ticker's own draw at its current clock - `None` when this source
    /// authors no ticker at all, nothing is honestly known to rotate
    /// through yet ([`Self::ticker_tips`]), or the clock is between two
    /// tips (see `oag_ui::campaign::footer::ticker_draw`'s own doc).
    #[must_use]
    pub(crate) fn ticker_draw(
        &self,
        faces: &oag_ui::picker::FaceScales,
        measure: &dyn Fn(&str) -> f32,
    ) -> Option<oag_ui::frontend::Draw> {
        oag_ui::campaign::footer::ticker_draw(
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

    /// Which tip strings the ticker honestly has to show - the `TKR_NO*`
    /// family, the only ones on disc that carry no `%d`/`%s`/`%.2f`
    /// template this build has a real counter for (see
    /// `oag_ui::campaign::footer`'s own module doc for why nothing here
    /// invents a play-time or song-count statistic instead).
    ///
    /// `Tournament`/`Head2Head` never launch at all in this engine
    /// (`oag_game::campaign::race_mode_for_cell`), so `TKR_NOTOURN`/
    /// `TKR_NOHH` are unconditionally true and always included. The other
    /// five are gated on whether [`Self::records`] carries any row for that
    /// mode - `oag_race::Mode::name`'s own spelling, the same string
    /// `oag_game::records::Key::new` normalises every record's `mode` to -
    /// which reads real save data rather than a guess, at the cost of never
    /// re-showing a tip once its mode has been raced even once.
    #[must_use]
    fn ticker_tips(&self) -> Vec<String> {
        let never_raced = |mode: &str| !self.records.rows().iter().any(|row| row.mode == mode);
        let mut ids = vec!["TKR_NOTOURN", "TKR_NOHH"];
        if never_raced(oag_race::Mode::SingleRace.name()) {
            ids.push("TKR_NOSR");
        }
        if never_raced(oag_race::Mode::TimeTrial.name()) {
            ids.push("TKR_NOTT");
        }
        if never_raced(oag_race::Mode::SpeedLap.name()) {
            ids.push("TKR_NOSL");
        }
        if never_raced(oag_race::Mode::Zone.name()) {
            ids.push("TKR_NOZONE");
        }
        if never_raced(oag_race::Mode::Eliminator.name()) {
            ids.push("TKR_NOELIM");
        }
        ids.into_iter()
            .filter_map(|id| self.strings.get(id))
            .map(str::to_string)
            .collect()
    }

    /// Whether this is Wipeout HD/Fury's own campaign - what
    /// `MenuStage::render`/`session::pointer::campaign_pointer` dispatch
    /// draw/pointer functions on. See [`Self::title`]'s own doc.
    #[must_use]
    pub(crate) fn is_hd(&self) -> bool {
        self.title == oag_hd::TITLE.name
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

    #[must_use]
    pub(crate) fn grid_layout(&self) -> &oag_ui::campaign::Layout {
        &self.grid_layout
    }

    #[must_use]
    pub(crate) fn cell_layout(&self) -> &oag_ui::campaign::Layout {
        &self.cell_layout
    }

    #[must_use]
    pub(crate) fn cell_help_layout(&self) -> Option<&oag_ui::campaign::Layout> {
        self.cell_help.as_ref()
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
        let by_name = cells.clone();
        self.screen = Screen::Cell {
            model: oag_ui::campaign::CellSelection::with_medals_and_records(
                cells,
                &|name| {
                    records
                        .campaign_medal(title, name)?
                        .best_medal
                        .map(to_campaign_medal)
                },
                &|name| Self::saved_record_centiseconds(&by_name, records, title, name),
            ),
            which,
        };
        true
    }

    /// `Line5`'s own value - `Cell_SavedRecord`, in centiseconds. Read off
    /// the general per-track/mode/class [`oag_game::records::Store`] rather
    /// than a per-cell store this build does not keep (see
    /// [`oag_ui::campaign::CellSelection::records`]'s own doc): a campaign
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

    /// **HD only** - the enclosing grid's own index, grid count and
    /// [`oag_ui::campaign::GridSummary`], for `oag_ui::campaign::hd::hd_cell_draw_list`'s
    /// own `EventNum`/`EPoints` counters. `None` off `Grid Selection` itself,
    /// since there is no "enclosing grid" there.
    #[must_use]
    pub(crate) fn cell_grid_summary(
        &self,
    ) -> Option<(usize, usize, oag_ui::campaign::GridSummary)> {
        let Screen::Cell { which, .. } = &self.screen else {
            return None;
        };
        let grid = self.grids.get(*which)?;
        Some((
            *which,
            self.grids.len(),
            Self::grid_summary(grid, &self.title, &self.records),
        ))
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
