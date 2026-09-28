//! Opens and drives the Race Campaign's two screens over the menus - the
//! same shape `session::picker` opens the race box's selection screens in.
//! See `oag_ui::campaign` for the model and `crate::campaign_stage` for what
//! this holds open.

use log::warn;
use oag_ui::campaign::Event;

use crate::campaign_stage::{CampaignStage, Screen};
use crate::stage::Stage;

use super::Session;

impl Session {
    /// `RACE CAMPAIGN`'s own action: opens `Grid Selection` over the menus.
    ///
    /// Reads the screen definition and all sixteen grid files fresh, the
    /// same way [`Self::open_track_picker`] opens the race box's archives on
    /// every press rather than keeping them from boot - see that function's
    /// own doc. A source with no campaign at all logs why and leaves the
    /// menus exactly as they were.
    pub(crate) fn open_campaign(&mut self) {
        let Some(options) = self.race_options.as_ref() else {
            return;
        };
        let (packs, pure_packs, problems) = oag_game::dlc::packs_from_defaults(
            &options.dlc,
            &oag_game::boot::default_dlc_cache_dir(),
        );
        for problem in problems {
            warn!("{problem}");
        }
        let mut archives = match oag_game::title::open_source(&options.source, packs, pure_packs) {
            Ok(opened) => opened.archives,
            Err(error) => {
                warn!(
                    "cannot open {} for RACE CAMPAIGN: {error:#}",
                    options.source
                );
                return;
            }
        };
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let faces = oag_ui::picker::FaceScales {
            default: shell
                .menu_font
                .as_ref()
                .map_or(oag_ui::picker::FaceScales::default().default, |menu| {
                    shell.font.line_height / menu.line_height
                }),
            ..oag_ui::picker::FaceScales::default()
        };
        let grid = [shell.space.size.0, shell.space.size.1];
        let mut strings = shell.strings.clone();
        // **HD/Fury only**: `Campaign Selection`'s own `ScreenTitle`/subtitle
        // idstrings (`FE_RC_SELECT`/`FE_CAMPSEL_MODES`) and its two entries'
        // own names (`FE_RC_FURY`/`FE_RC_HD`) are not in the language table
        // `shell.strings` already carries - that table is built off whichever
        // archive `Archives::read_name` resolves for `shell.entries`, and
        // (the same asymmetry `oag_ui::language::StringTable::get`'s own doc
        // records for the circuit names) only `DATA06`'s own copy of the
        // entries file carries any of the four ids at all, since all four
        // are new to the Fury-era screen. Overlaid here rather than
        // generally: merging the whole of `DATA06`'s copy into `strings`
        // risks silently changing an already-resolved id elsewhere on a
        // disagreement this pass has not audited, where this touches only
        // the four ids this screen needs. Shared with
        // `crate::capture::campaign_page`'s own `--menu-page
        // campaign-select` path through `oag_game::campaign::hd_selection_string_overlay`
        // so the two cannot resolve these ids differently.
        if shell.title.name == oag_hd::TITLE.name
            && let Some(entries_path) = shell.entries.as_deref()
        {
            let overlay =
                oag_game::campaign::hd_selection_string_overlay(&mut archives, entries_path);
            if !overlay.is_empty() {
                strings.merge(overlay);
            }
        }
        // Read out of `shell` before the mutable borrow below - `shell`
        // itself cannot survive `self.renderer_set_sprites`, the same reason
        // `strings` above is already a clone rather than a borrow.
        let title_ref = shell.title;
        let title = shell.title.name.to_string();
        let circuit_names = shell.circuit_names.clone();
        let records = self.records.clone();
        let globals: Vec<(&str, &str)> = shell
            .globals
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        match oag_game::campaign::load(
            &mut archives,
            &strings,
            faces,
            grid,
            &shell.sprites,
            &globals,
            title_ref,
        ) {
            Ok(campaign) => {
                // The extended sheet - `hex_filled.mip`/`hex_outline.mip`,
                // neither of which `Skin.xml`'s own sheet carries - has to
                // reach the renderer once before anything drawn from it is
                // on screen, the same upload `PickerStage::take_sheet` does
                // for a slideshow's own stills.
                self.renderer_set_sprites(&campaign.sprites);
                if let Stage::Menu(menu_stage) = &mut self.stage {
                    menu_stage.campaign = Some(CampaignStage::new(
                        campaign.grids,
                        campaign.grid_layout,
                        campaign.cell_layout,
                        campaign.selection_layout,
                        campaign.grid_layout_fury,
                        campaign.cell_help,
                        campaign.nav_legend,
                        campaign.ticker,
                        strings,
                        campaign.sprites,
                        title,
                        circuit_names,
                        records,
                    ));
                }
            }
            Err(error) => warn!("{error:#} - RACE CAMPAIGN has nothing to show"),
        }
    }

    /// Uploads `sheet` to the menu stage's own renderer - `MenuStage::render`
    /// otherwise has no chance to, since [`Self::open_campaign`] runs before
    /// the next frame's draw and a campaign screen carries no `take_sheet`
    /// dance of its own the way a picker's slideshow does.
    fn renderer_set_sprites(&mut self, sheet: &oag_game::sprite::Sheet) {
        if let Stage::Menu(stage) = &mut self.stage {
            stage
                .renderer
                .set_sprites(&self.gpu.device, &self.gpu.queue, sheet);
        }
    }

    /// One tick of an open campaign screen: its pad input, its pointer, and
    /// what came of either.
    pub(crate) fn tick_campaign(&mut self, pointer: &oag_ui::pointer::Pointer) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(campaign) = stage.campaign.as_mut() else {
            return;
        };
        let mut events = match &mut campaign.screen {
            Screen::Selection(model) => model.update(self.controls.buttons_mut()),
            Screen::Grid(model) => model.update(self.controls.buttons_mut()),
            Screen::Cell { model, .. } => model.update(self.controls.buttons_mut()),
        };
        events.extend(super::pointer::campaign_pointer(stage, pointer));
        for event in events {
            self.handle_campaign(event);
        }
    }

    /// The pad path (`GridSelection::update`/`CellSelection::update`) and
    /// the pointer path (`super::pointer::campaign_pointer`) both fold down
    /// to this one function, which is deliberate: a locked tile refuses
    /// `Confirm` on **either** input, and gating only inside `update` would
    /// leave a mouse two-tap launching a cell the pad cannot.
    pub(crate) fn handle_campaign(&mut self, event: Event) {
        // Whatever cell was just confirmed, read out here and acted on
        // *after* this borrow of `self.stage` ends below - `launch_campaign_cell`
        // needs `&mut self` for `self.race_options`/`self.shell`/`self.records`,
        // which cannot start while `stage`/`campaign` still borrow `self.stage`.
        let mut confirmed_cell = None;
        let mut confirmed_difficulty = None;
        {
            let Stage::Menu(stage) = &mut self.stage else {
                return;
            };
            let Some(campaign) = stage.campaign.as_mut() else {
                return;
            };
            match (&campaign.screen, event) {
                // **HD only** - `Campaign Selection` ahead of `Grid
                // Selection`. See `crate::campaign_stage`'s own module doc.
                (Screen::Selection(model), Event::Confirmed) => {
                    let chosen = model.selected();
                    campaign.open_grid_selection(chosen);
                }
                (Screen::Selection(_), Event::Back) => stage.campaign = None,
                (Screen::Grid(model), Event::Confirmed) => {
                    // The gate itself is the recovered `Unlock_GridPointsMet`
                    // law (`CampaignStage::grid_is_unlocked`), not the
                    // display-only lock-glyph shortcut `model.selected_is_locked`
                    // still draws with. **Chosen, not measured** is narrower
                    // than it used to be: no PI001 function this project has
                    // decompiled ever refuses the transition on `PI_Grid.Locked`
                    // - a live capture measured that confirming a locked tile
                    // does nothing - so *that* refusal is still a choice, but
                    // *what it refuses on* is the disc's own law. See
                    // `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s
                    // "Unlock rules, cell and tier".
                    let index = model.index();
                    if !campaign.grid_is_unlocked(index) {
                        log::info!(
                            "grid {index} is locked - Confirm does nothing, chosen not measured"
                        );
                    } else if !campaign.open_cell_selection_at_grid_slot(index) {
                        warn!("grid {index} has no cells - staying on Grid Selection");
                    }
                }
                // **HD only, when `Campaign Selection` was read at all**:
                // `Grid Selection`'s own `Back` returns to it rather than
                // closing the campaign outright. Every other title, and an
                // HD source missing `DATA06`'s own copy of the screen, keeps
                // the pre-this-pass behaviour.
                (Screen::Grid(_), Event::Back) if campaign.has_selection() => {
                    campaign.open_selection();
                }
                (Screen::Grid(_), Event::Back) => stage.campaign = None,
                (Screen::Cell { model, .. }, Event::Confirmed) => {
                    // The identical reasoning as the tier arm above, on
                    // `PI_Cell.Locked`.
                    if model.selected_is_locked() {
                        log::info!(
                            "{} is locked - Confirm does nothing, chosen not measured",
                            model
                                .selected()
                                .map_or_else(|| "no cell".to_string(), |cell| cell.name.clone())
                        );
                    } else {
                        confirmed_cell = model.selected().cloned();
                        // Recorded for every cell, not only one whose own
                        // `difficulty_targets` vary the medal. Pulse's own
                        // `CellSelection_CommitSelection` (`0x088d6138`)
                        // unconditionally persists the screen's own browsed
                        // rung on every Confirm, on every cell - the rung
                        // played at is metadata banked alongside the medal
                        // (`Cell_SavedDifficulty`), never a gate on which
                        // medal is earned. See
                        // `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s
                        // "The `DifficultyRC` persisted rung" section.
                        confirmed_difficulty = confirmed_cell.as_ref().map(|_| model.difficulty());
                    }
                }
                (Screen::Cell { .. }, Event::Back) => campaign.back_to_grid_selection(),
                // `DifficultyChanged` is HD-only, ours, and needs nothing
                // from the composition root - `CellSelection::cycle_difficulty`
                // already moved the model's own state before this event
                // reached here. See `oag_ui::campaign::Event::DifficultyChanged`'s
                // own doc.
                (_, Event::Moved | Event::Help | Event::DifficultyChanged) => {}
            }
        }
        if let Some(cell) = confirmed_cell {
            self.launch_campaign_cell(cell, confirmed_difficulty);
        }
    }

    /// Launches the race `cell` describes, the same way `LAUNCH RACE`
    /// launches the RACE page's own settings - see
    /// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "how a
    /// campaign event launches": the cell's own track/mode/class/laps/kill
    /// target replace whatever the RACE page had selected, `Team Selection`
    /// (`Self::open_ship_picker`) follows because every authored cell
    /// carries `ShipChoice="Yes"`, and `self.campaign_cell` carries the cell
    /// itself through to `RaceStage::observation` for the medal.
    /// `difficulty` rides alongside it the same way, `None` on every cell
    /// with no rung to be run at - see the caller's own doc for when that
    /// is.
    ///
    /// A cell whose mode this engine cannot run, or whose track this source
    /// does not offer, logs why and leaves `Cell Selection` on screen -
    /// never substitutes an implemented mode or a different circuit for an
    /// unimplemented or missing one.
    ///
    /// **A Tournament cell names its legs through
    /// [`oag_tables::race_campaign::Cell::tournament_tracks`] rather than
    /// [`oag_tables::race_campaign::Cell::track`]** - see that field's own
    /// doc. Every leg's own track is resolved up front here, before
    /// anything launches: a tournament that started on a shorter leg list
    /// than the cell authors, because a later leg's own track turned out
    /// missing, would be a silent truncation rather than an honest refusal.
    fn launch_campaign_cell(
        &mut self,
        cell: oag_tables::race_campaign::Cell,
        difficulty: Option<oag_tables::race_campaign::Difficulty>,
    ) {
        let Some(mode) = oag_game::campaign::race_mode_for_cell(cell.mode.clone()) else {
            warn!(
                "{} is {} - not one of the modes this engine can run yet, so {} cannot launch",
                cell.mode, cell.mode, cell.name
            );
            return;
        };
        let is_tournament = mode == oag_race::Mode::Tournament;
        let track_ids: Vec<String> = if is_tournament {
            cell.tournament_tracks.clone()
        } else {
            cell.track.clone().into_iter().collect()
        };
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        if track_ids.is_empty() {
            warn!("{} names no track - cannot launch", cell.name);
            return;
        }
        let mut leg_entries = Vec::with_capacity(track_ids.len());
        for track_id in &track_ids {
            let Some(entry) = shell
                .track(mode, track_id)
                .map(oag_game::catalogue::Track::entry_name)
            else {
                warn!(
                    "this source does not offer {track_id:?} - {} cannot launch",
                    cell.name
                );
                return;
            };
            leg_entries.push(entry);
        }
        let entry = leg_entries[0].clone();
        // A `Zone` cell's own `class` is the literal string `"Zone"`, not a
        // speed class - see `oag_tables::race_campaign::Cell::speed_class`'s
        // own doc. Nothing in this engine resolves a Zone handling block of
        // its own yet (`docs/gameplay/race-modes.md`'s own "every title
        // ships one Zone handling block, and this engine does not read it"),
        // so the class a Zone cell races under falls back to whatever the
        // RACE page last had selected - chosen, not measured, and reported
        // so the substitution is never silent.
        let class = match cell.speed_class() {
            Some(_) => cell.class.clone(),
            None => {
                let fallback = self.settings.race.class.clone();
                warn!(
                    "{}'s own class {:?} is not a speed class - racing at {fallback:?} instead, \
                     chosen not measured",
                    cell.name, cell.class
                );
                fallback
            }
        };
        let Some(mut race_options) = self.race_options.take() else {
            warn!("no disc image has been chosen yet, so there is nothing to race");
            return;
        };
        race_options.track = Some(entry);
        race_options.mode = mode;
        race_options.class = class;
        // Elimination's own kill target is the cell's gold target - see
        // `oag_race::Mode::ELIMINATOR_KILL_TARGET_DEFAULT`'s own doc, which
        // this is the caller that retires the default in favour of.
        race_options.eliminator_kill_target = (mode == oag_race::Mode::Eliminator)
            .then(|| u32::try_from(cell.gold).ok())
            .flatten();
        // Only for the modes whose own `Mode::laps_target` already returns
        // `Some` - see `race::Options::laps_override`'s own doc for why
        // `SpeedLap`/`Zone` must never take their own `laps` attribute this
        // way. Tournament included: its own cell carries the identical
        // 3/4/4/5 census every leg races under - see `Mode::Tournament`'s
        // own doc comment. Head2Head included too, on the same census -
        // `docs/ghidra/functions/psp-pulse-usa/head2head.md`.
        race_options.laps_override = matches!(
            mode,
            oag_race::Mode::TimeTrial
                | oag_race::Mode::SingleRace
                | oag_race::Mode::Tournament
                | oag_race::Mode::Head2Head
        )
        .then_some(cell.laps)
        .flatten();
        self.race_options = Some(race_options);
        self.campaign_cell = Some(cell);
        self.campaign_difficulty = difficulty;
        // `None` for every ordinary cell, clearing whatever a previous
        // tournament (finished or abandoned) left behind - see
        // `Self::tournament`'s own doc for why nothing else has to clear it
        // on this path.
        self.tournament =
            is_tournament.then(|| crate::race::tournament::Progress::new(leg_entries));
        if let Stage::Menu(stage) = &mut self.stage {
            stage.campaign = None;
        }
        if !self.open_ship_picker() {
            self.finish_launch();
        }
    }
}
