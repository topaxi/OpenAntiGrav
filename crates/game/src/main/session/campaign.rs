//! Opens and drives the Race Campaign's two screens over the menus - the
//! same shape `session::picker` opens the race box's selection screens in.
//! See `oag_ui_screens::campaign` for the model and `crate::campaign_stage` for what
//! this holds open.

use log::warn;
use oag_game::campaign::launch::Refusal;
use oag_ui_screens::campaign::Event;

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
        let Some(mut archives) = self.open_title_archives("RACE CAMPAIGN") else {
            return;
        };
        if self
            .shell
            .as_ref()
            .is_some_and(|shell| shell.title.campaign.list_entry.is_some())
        {
            self.open_pure_chain(archives);
            return;
        }
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let faces = oag_ui_screens::picker::FaceScales {
            default: shell.menu_font.as_ref().map_or(
                oag_ui_screens::picker::FaceScales::default().default,
                |menu| shell.font.line_height / menu.line_height,
            ),
            ..oag_ui_screens::picker::FaceScales::default()
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
        if shell.title.campaign.selection_strings
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
        let platform = shell.platform;
        let circuit_names = shell.circuit_names.clone();
        let tracks: Vec<oag_raceplay::catalogue::Track> =
            shell.tracks.iter().map(|(track, _)| track.clone()).collect();
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
            &tracks,
        ) {
            Ok(mut campaign) => {
                if let Some(flyers) = campaign.flyers.as_mut() {
                    flyers.anisotropy = self.anisotropy;
                }
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
                        title_ref,
                        platform,
                        circuit_names,
                        records,
                        campaign.flyers,
                        campaign.circuit_emblems,
                        self.campaign_cursor,
                    ));
                }
            }
            Err(error) => warn!("{error:#} - RACE CAMPAIGN has nothing to show"),
        }
    }

    /// The booted source's archives, opened fresh with its DLC packs, for a
    /// screen that reads the disc on every opening rather than keeping it from
    /// boot - see [`Self::open_campaign`]. `None` (logged, as `what` could not
    /// be shown) with no source chosen yet or one that will not open.
    pub(crate) fn open_title_archives(&self, what: &str) -> Option<oag_assets::Archives> {
        let options = self.race_options.as_ref()?;
        let (packs, pure_packs, problems) = oag_source::dlc::packs_from_defaults(
            &options.dlc,
            &oag_source::cache::default_dlc_cache_dir(),
        );
        for problem in problems {
            warn!("{problem}");
        }
        match oag_source::title::open_source(&options.source, packs, pure_packs) {
            Ok(opened) => Some(opened.archives),
            Err(error) => {
                warn!("cannot open {} for {what}: {error:#}", options.source);
                None
            }
        }
    }

    /// Uploads `sheet` to the menu stage's own renderer - `MenuStage::render`
    /// otherwise has no chance to, since [`Self::open_campaign`] runs before
    /// the next frame's draw and a campaign screen carries no `take_sheet`
    /// dance of its own the way a picker's slideshow does.
    fn renderer_set_sprites(&mut self, sheet: &oag_hud::sprite::Sheet) {
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
        let nav = self.campaign_nav(event);
        self.play_navs(nav);
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
            // A refusal line lasts until the player does anything else.
            campaign.notice = None;
            match (&campaign.screen, event) {
                // **HD only** - `Campaign Selection` ahead of `Grid
                // Selection`. See `crate::campaign_stage`'s own module doc.
                (Screen::Selection(model), Event::Confirmed) => {
                    let chosen = model.selected();
                    campaign.open_grid_selection(chosen);
                }
                (Screen::Selection(_), Event::Back) => {
                    self.campaign_cursor = campaign.cursor_on_close();
                    stage.campaign = None;
                }
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
                        log::debug!(
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
                (Screen::Grid(_), Event::Back) => {
                    self.campaign_cursor = campaign.cursor_on_close();
                    stage.campaign = None;
                }
                (Screen::Cell { model, .. }, Event::Confirmed) => {
                    // The identical reasoning as the tier arm above, on
                    // `PI_Cell.Locked`.
                    if model.selected_is_locked() {
                        log::debug!(
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
                // reached here. See `oag_ui_screens::campaign::Event::DifficultyChanged`'s
                // own doc.
                (_, Event::Moved | Event::Help | Event::DifficultyChanged) => {}
            }
        }
        if let Some(cell) = confirmed_cell {
            self.launch_campaign_cell(cell, confirmed_difficulty);
        }
    }

    /// What launching `cell` would race, or why it cannot; [`None`] before
    /// anything has booted. Pure over the shell and the settings, so
    /// [`Self::campaign_nav`] asks it ahead of the launch to pick the cue.
    pub(super) fn plan_cell(
        &self,
        cell: &oag_tables::race_campaign::Cell,
    ) -> Option<Result<oag_game::campaign::launch::CellPlan, Refusal>> {
        let shell = self.shell.as_ref()?;
        // The pure half - mode, circuits, class, laps, kill target - is
        // `oag_game::campaign::launch::plan_cell`, so a disc-backed test holds
        // it to every cell a title ships. A `Zone` cell's own `class` is the
        // literal string `"Zone"`, not a speed class, and nothing in this
        // engine resolves a Zone handling block of its own yet, so it races at
        // whatever the RACE page last had selected - chosen, not measured,
        // and logged by the launch so the substitution is never silent.
        let ladder = shell
            .title
            .race
            .speed_classes
            .map_or(&[][..], |classes| classes.names);
        Some(oag_game::campaign::launch::plan_cell_for(
            cell,
            &self.settings.race.class,
            ladder,
            |mode, id| {
                shell
                    .track(mode, id)
                    .map(oag_raceplay::catalogue::Track::entry_name)
                    // A Pure campaign's Classic circuits are not in the
                    // shell's catalogue, which lists the ordinary ones.
                    .or_else(|| self.pure_classic_entries.get(id).cloned())
            },
        ))
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
    pub(super) fn launch_campaign_cell(
        &mut self,
        cell: oag_tables::race_campaign::Cell,
        difficulty: Option<oag_tables::race_campaign::Difficulty>,
    ) {
        let Some(plan) = self.plan_cell(&cell) else {
            return;
        };
        let plan = match plan {
            Ok(plan) => plan,
            Err(refusal) => {
                warn!("{}: {refusal}", cell.name);
                let (id, literal) = match refusal {
                    Refusal::Mode(_) => (
                        "OAG_CAMPAIGN_MODE_UNSUPPORTED",
                        "THIS EVENT'S MODE CANNOT BE RACED IN THIS BUILD YET",
                    ),
                    Refusal::NoTrack | Refusal::MissingTrack(_) => (
                        "OAG_CAMPAIGN_TRACK_UNAVAILABLE",
                        "THIS EVENT'S CIRCUIT IS NOT ON THIS DISC",
                    ),
                };
                if let Stage::Menu(stage) = &mut self.stage
                    && let Some(campaign) = stage.campaign.as_mut()
                {
                    campaign.notice = Some(campaign.strings.get(id).unwrap_or(literal).to_string());
                }
                return;
            }
        };
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        if plan.class_is_fallback {
            warn!(
                "{}'s own class {:?} is not a speed class - racing at {:?} instead, \
                 chosen not measured",
                cell.name, cell.class, plan.class
            );
        }
        let mode = plan.mode;
        let is_tournament = mode == oag_race::Mode::Tournament;
        let first_track = if is_tournament {
            cell.tournament_tracks.first()
        } else {
            cell.track.as_ref()
        }
        .cloned()
        .unwrap_or_default();
        let Some(mut race_options) = self.race_options.take() else {
            warn!("no disc image has been chosen yet, so there is nothing to race");
            return;
        };
        self.inherit_cli_foreign_track(&mut race_options);
        race_options.track = Some(plan.leg_entries[0].clone());
        race_options.mode = mode;
        race_options.class = plan.class;
        race_options.no_driver_for_class = false;
        race_options.weapons_override = None;
        race_options.eliminator_kill_target = plan.eliminator_kill_target;
        race_options.laps_override = plan.laps_override;
        let leg_entries = plan.leg_entries;
        // `AI_ResolveSkillScale`'s campaign-cell branch
        // (`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`,
        // `0x08834df4`) - resolved once here, against the first leg's own
        // track, rather than per tournament leg: this engine does not
        // re-run AI setup between legs today, so a multi-track cell races
        // every leg at the position its first track's own curve gives -
        // chosen, not measured, on the multi-leg case only.
        // `race_options` is out of `self` until just below, so the source is
        // handed over rather than read back off `self` (which read `None`
        // here and so never resolved a cell's skill, until 2026-10-12).
        let ai_skill_scale = difficulty.and_then(|rung| {
            let location = shell.track(mode, &first_track)?.location.clone();
            self.resolve_campaign_ai_skill_scale(&cell, rung, &location, &race_options.source)
        });
        self.race_options = Some(race_options);
        self.campaign_cell = Some(cell);
        self.campaign_difficulty = difficulty;
        self.campaign_ai_skill_scale = ai_skill_scale;
        // `None` for every ordinary cell, clearing whatever a previous
        // tournament (finished or abandoned) left behind - see
        // `Self::tournament`'s own doc for why nothing else has to clear it
        // on this path.
        self.tournament =
            is_tournament.then(|| oag_raceplay::tournament::Progress::new(leg_entries));
        if let Stage::Menu(stage) = &mut self.stage {
            stage.pure_chain = None;
            if let Some(campaign) = stage.campaign.take() {
                self.campaign_cursor = campaign.cursor_on_close();
            }
        }
        // A title that flies Zone in its own craft has no team to pick for it,
        // as the RACE page's own launch skips the screen
        // (`Session::handle_picker`).
        let own_zone_craft = mode == oag_race::Mode::Zone
            && self.shell.as_ref().is_some_and(|shell| {
                matches!(
                    shell.title.race.zone_craft,
                    oag_title::ZoneCraft::OwnShip(_)
                )
            });
        if own_zone_craft || !self.open_ship_picker() {
            self.launch_campaign_race();
        }
    }

    /// The campaign's own launch tail, once `Team Selection` is settled (or
    /// skipped, on a source with no ship picker): the cell already built
    /// `self.race_options` in full, so only the team, variant and livery
    /// that screen picked are copied in before the load. Both campaign exits
    /// come through here - `Session::handle_picker`'s `Kind::Ship` Confirm
    /// and [`Self::launch_campaign_cell`]'s own no-picker fallback - so
    /// neither can race without the team the player picked, which is what
    /// the EndRace loyalty row is keyed on. See
    /// `docs/ui/endrace-screens.md`.
    pub(crate) fn launch_campaign_race(&mut self) {
        if let Some(mut race_options) = self.race_options.take() {
            self.apply_race_team(&mut race_options);
            self.race_options = Some(race_options);
        }
        self.finish_launch();
    }

    /// `AI_ResolveSkillScale`'s campaign-cell branch
    /// (`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`,
    /// `0x08834df4`, confidence 85): `cell`'s own position on `location`'s
    /// track carries a `stats.xml` in `FEData.wad` reads as, at `rung`.
    ///
    /// `None` on any failure to open or read that file - a title with no
    /// `FEData.wad` at this path (PSP Pulse reads it there; HD/Fury reads the
    /// same file out of its own archives, below; PS2 Pulse ships the same
    /// per-track data under a different archive root this project has not
    /// measured), a track missing its
    /// own record, or `cell` carrying no [`oag_tables::race_campaign::Cell::skill`]
    /// at all (a solo-mode cell has no AI to scale). Every one of those
    /// falls back to [`oag_tables::track_stats::resolve_skill_scale`]'s own
    /// documented default curve rather than refusing the launch - the
    /// original substitutes the same default when its own track table has
    /// not loaded.
    fn resolve_campaign_ai_skill_scale(
        &self,
        cell: &oag_tables::race_campaign::Cell,
        rung: oag_tables::race_campaign::Difficulty,
        location: &str,
        source: &str,
    ) -> Option<f32> {
        let entry = format!(r"{location}\stats.xml");
        let spec = oag_assets::source::archive_spec(source, oag_pulse::archives::FEDATA);
        let blob = match oag_assets::Archive::open(&spec) {
            Ok(mut archive) => archive.read_name(&entry).ok(),
            // HD keeps each circuit's `stats.xml` in its own archives
            // (`/data/environments/<dir>/stats.xml`, seventeen of them) and
            // reads it through the same `%s\stats.xml` format string; its
            // cell branch is Pulse's (`AI_ResolveSkillScale`, `0x000babf8`),
            // measured on RPCS3: the Fury cell `grid8_3_1` gave `-0.07` at
            // Easy and `3.48` at Hard, which this reproduces
            // (`docs/ghidra/functions/ps3-hdfury-eu/ai-controls.md`). Read
            // there only for a title whose campaign flies the original's own
            // opponent, so no other title's campaign tuning moves.
            Err(_) if self.shell.as_ref()?.title.race.original_opponent => {
                oag_assets::Archives::open(source, self.shell.as_ref()?.title)
                    .ok()?
                    .read_name(&entry)
                    .ok()
            }
            Err(_) => return None,
        };
        let stats = blob.and_then(|blob| oag_tables::track_stats::from_blob(&blob).ok());
        oag_tables::track_stats::resolve_skill_scale(cell, rung, stats.as_ref())
    }
}
