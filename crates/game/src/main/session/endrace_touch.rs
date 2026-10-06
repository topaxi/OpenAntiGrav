//! Builds and drives Wipeout 2048's end-of-race pages - the
//! `oag_title::EndRaceDialect::Touch` counterpart of
//! [`super::endrace`]. The runtime is
//! `crate::race_stage::endrace_touch::TouchRuntime`.

use log::warn;

use oag_game::endrace::touch::{Facts, load, summary};
use oag_gameplay::input::Button;
use oag_ui_screens::endrace::touch::Hit;

use crate::race_stage::endrace_touch::{Action, EndRace, Press, TouchRuntime};
use crate::stage::Stage;

use super::Session;

impl Session {
    /// [`Session::build_endrace`]'s touch branch, reached once per finished
    /// race from its guards: opens the source, reads the pages, words the race
    /// and builds the runtime. Any failure marks the race
    /// `endrace_unavailable` so the built-in results table stays, and says why.
    pub(super) fn build_endrace_touch(&mut self) {
        let Stage::Race(stage) = &self.stage else {
            return;
        };
        let (Some(shell), Some(race_options)) = (self.shell.as_ref(), self.race_options.as_ref())
        else {
            return;
        };
        let title = shell.title;
        let Some(entry) = title.front_end.and_then(|f| f.endrace_entry) else {
            warn!("{} names no EndRace definition", title.name);
            self.mark_endrace_unavailable();
            return;
        };
        let facts = Facts::from_race(
            &stage.race,
            stage
                .campaign_2048_event
                .as_ref()
                .and_then(|progress| progress.objectives),
        );
        let strings = shell.strings.clone();
        let model = summary(&facts, &strings);
        let faces = oag_ui_screens::picker::FaceScales::default();
        let grid = [shell.space.size.0, shell.space.size.1];
        let globals: Vec<(String, String)> = shell
            .globals
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let global_refs: Vec<(&str, &str)> = globals
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        let (source, dlc) = (race_options.source.clone(), race_options.dlc.clone());
        let (base, space, atlas, scales) = (
            shell.sprites.clone(),
            shell.space,
            shell.font.clone(),
            shell.face_scales.clone(),
        );
        let (packs, pure_packs, problems) =
            oag_source::dlc::packs_from_defaults(&dlc, &oag_source::cache::default_dlc_cache_dir());
        for problem in problems {
            warn!("{problem}");
        }
        let mut archives = match oag_source::title::open_source(&source, packs, pure_packs) {
            Ok(opened) => opened.archives,
            Err(error) => {
                warn!("cannot open {source} for the EndRace pages: {error:#}");
                self.mark_endrace_unavailable();
                return;
            }
        };
        let screens = match load(
            &mut archives,
            entry,
            &strings,
            faces,
            grid,
            &base,
            &global_refs,
        ) {
            Ok(screens) => screens,
            Err(error) => {
                warn!("{error:#} - keeping the built-in results table");
                self.mark_endrace_unavailable();
                return;
            }
        };
        match TouchRuntime::new(
            &self.gpu.device,
            &self.gpu.queue,
            self.gpu.config.format,
            screens,
            model,
            space,
            atlas,
            scales,
        ) {
            Ok(runtime) => {
                if let Stage::Race(stage) = &mut self.stage {
                    stage.endrace = Some(EndRace::Touch(Box::new(runtime)));
                }
            }
            Err(error) => {
                warn!("cannot build the EndRace pages' renderer: {error:#}");
                self.mark_endrace_unavailable();
            }
        }
    }

    /// One tick of the open pages: the pad, the pointer and the clock.
    ///
    /// **Pad is chosen, not measured**: the original is a touch screen.
    /// Left/Right move the cursor between the two tiles, Up/Down turn the page,
    /// Cross or Start press the tile under the cursor.
    pub(super) fn tick_endrace_touch(&mut self, pointer: &oag_ui::pointer::Pointer) {
        let mut presses = Vec::new();
        for (button, press) in [
            (Button::Left, Press::Sideways),
            (Button::Right, Press::Sideways),
            (Button::Up, Press::Previous),
            (Button::Down, Press::Next),
            (Button::Cross, Press::Confirm),
            (Button::Start, Press::Confirm),
        ] {
            if self.controls.buttons_mut().take(button) {
                presses.push(press);
            }
        }
        let Stage::Race(stage) = &mut self.stage else {
            return;
        };
        let Some(EndRace::Touch(runtime)) = stage.endrace.as_mut() else {
            return;
        };
        let hit = runtime.pointer_hit(pointer);
        let flow = runtime.flow_mut();
        flow.tick();
        let mut action = None;
        for press in presses {
            action = action.or(flow.press(press));
        }
        match hit {
            Some(Hit::Tile(button)) => action = action.or(flow.tap_tile(button)),
            Some(Hit::Panel) => flow.tap_panel(),
            None => {}
        }
        match action {
            Some(Action::Exit) => self.leave_finished_race(),
            Some(Action::Restart) => self.restart_event(),
            None => {}
        }
    }

    /// Races the same 2048 event again, the way the map's own tap did: the
    /// event's name is handed to the loader and the race is launched through
    /// the one path every launch uses. A race that was not an event (the RACE
    /// page, REMIX) relaunches on the options it ran with.
    fn restart_event(&mut self) {
        if let Stage::Race(stage) = &self.stage {
            self.pending_event = stage.campaign_2048_event.as_ref().map(|p| p.name.clone());
        }
        self.finish_launch();
    }
}
