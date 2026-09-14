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
        let strings = shell.strings.clone();
        // Read out of `shell` before the mutable borrow below - `shell`
        // itself cannot survive `self.renderer_set_sprites`, the same reason
        // `strings` above is already a clone rather than a borrow.
        let title = shell.title.name.to_string();
        let records = self.records.clone();
        match oag_game::campaign::load(&mut archives, &strings, faces, grid, &shell.sprites) {
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
                        strings,
                        campaign.sprites,
                        title,
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

    /// One tick of an open campaign screen: its input, and what came of it.
    pub(crate) fn tick_campaign(&mut self) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(campaign) = stage.campaign.as_mut() else {
            return;
        };
        let events = match &mut campaign.screen {
            Screen::Grid(model) => model.update(self.controls.buttons_mut()),
            Screen::Cell { model, .. } => model.update(self.controls.buttons_mut()),
        };
        for event in events {
            self.handle_campaign(event);
        }
    }

    pub(crate) fn handle_campaign(&mut self, event: Event) {
        // Whatever cell was just confirmed, read out here and acted on
        // *after* this borrow of `self.stage` ends below - `launch_campaign_cell`
        // needs `&mut self` for `self.race_options`/`self.shell`/`self.records`,
        // which cannot start while `stage`/`campaign` still borrow `self.stage`.
        let mut confirmed_cell = None;
        {
            let Stage::Menu(stage) = &mut self.stage else {
                return;
            };
            let Some(campaign) = stage.campaign.as_mut() else {
                return;
            };
            match (&campaign.screen, event) {
                (Screen::Grid(model), Event::Confirmed) => {
                    let index = model.index();
                    if !campaign.open_cell_selection(index) {
                        warn!("grid {index} has no cells - staying on Grid Selection");
                    }
                }
                (Screen::Grid(_), Event::Back) => stage.campaign = None,
                (Screen::Cell { model, .. }, Event::Confirmed) => {
                    confirmed_cell = model.selected().cloned();
                }
                (Screen::Cell { .. }, Event::Back) => campaign.back_to_grid_selection(),
                (_, Event::Moved | Event::Help) => {}
            }
        }
        if let Some(cell) = confirmed_cell {
            self.launch_campaign_cell(cell);
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
    ///
    /// A cell whose mode this engine cannot run, or whose track this source
    /// does not offer, logs why and leaves `Cell Selection` on screen -
    /// never substitutes an implemented mode or a different circuit for an
    /// unimplemented or missing one.
    fn launch_campaign_cell(&mut self, cell: oag_tables::race_campaign::Cell) {
        let Some(mode) = oag_game::campaign::race_mode_for_cell(cell.mode) else {
            warn!(
                "{} is {} - not one of the modes this engine can run yet, so {} cannot launch",
                cell.mode, cell.mode, cell.name
            );
            return;
        };
        let Some(track_id) = cell.track.as_deref() else {
            warn!("{} names no track - cannot launch", cell.name);
            return;
        };
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
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
        // Only for the two modes whose own `Mode::laps_target` already
        // returns `Some` - see `race::Options::laps_override`'s own doc for
        // why `SpeedLap`/`Zone` must never take their own `laps` attribute
        // this way.
        race_options.laps_override =
            matches!(mode, oag_race::Mode::TimeTrial | oag_race::Mode::SingleRace)
                .then_some(cell.laps)
                .flatten();
        self.race_options = Some(race_options);
        self.campaign_cell = Some(cell);
        if let Stage::Menu(stage) = &mut self.stage {
            stage.campaign = None;
        }
        if !self.open_ship_picker() {
            self.finish_launch();
        }
    }
}
