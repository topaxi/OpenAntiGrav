//! Playing the front end's navigation sounds.
//!
//! The screens log or return a [`Nav`]; this turns each into the title's
//! front-end cue and starts it. Nothing here decides *when* a sound belongs -
//! `oag_ui::menu::nav` and the screens' `Event::nav` do - so a title without a
//! front-end bank plays nothing without a branch of its own.

use log::debug;
use oag_ui::menu::nav::{Dir, Nav};
use oag_ui_screens::campaign::Event;

use super::Session;
use crate::campaign_stage::Screen;
use crate::stage::Stage;

/// The direction a d-pad edge points this tick, the last one pressed if
/// several were.
pub(super) fn pad_direction(buttons: &oag_gameplay::input::Input) -> Option<Dir> {
    use oag_gameplay::input::Button;
    [
        (Button::Up, Dir::Up),
        (Button::Down, Dir::Down),
        (Button::Left, Dir::Left),
        (Button::Right, Dir::Right),
    ]
    .into_iter()
    .filter(|(button, _)| buttons.is_pressed(*button))
    .map(|(_, dir)| dir)
    .next_back()
}

impl Session {
    /// Starts the cue for each of `navs`, oldest first.
    pub(crate) fn play_navs(&mut self, navs: impl IntoIterator<Item = Nav>) {
        let Some(sfx) = self
            .shell
            .as_mut()
            .and_then(|shell| shell.menu_sfx.as_mut())
        else {
            return;
        };
        for nav in navs {
            let cue = oag_game::sound::menu_cue(nav, self.pad_dir);
            let started = sfx.play(&self.audio, cue);
            let live = self.audio.output().with_mixer(|mixer| mixer.active_voices());
            debug!("menu sound: {cue:?} (started: {started}, {live} voice(s) live)");
        }
    }

    /// Speaks the name of the circuit or team a selection screen just
    /// confirmed (`TRTX_NAME`, `TMTX_NAME`), by the `soundregister` the
    /// plugin's own entry authors. Silent where the entry authors none or the
    /// title's voice bank did not load; nothing is chosen in its place.
    pub(crate) fn speak_name(&mut self, kind: oag_ui_screens::picker::Kind, id: &str) {
        use oag_ui_screens::picker::Kind;
        let mode = self.race_mode();
        let Some(shell) = self.shell.as_mut() else {
            return;
        };
        let register = match kind {
            Kind::Track => shell
                .tracks_for(mode)
                .iter()
                .find(|(track, _)| track.id == id)
                .and_then(|(track, _)| track.sound_register),
            Kind::Ship => shell
                .team_details
                .iter()
                .find(|team| team.id == id)
                .and_then(|team| team.sound_register),
        };
        let (Some(register), Some(sfx)) = (register, shell.menu_sfx.as_mut()) else {
            return;
        };
        let started = match kind {
            Kind::Track => sfx.play_circuit_name(&self.audio, register),
            Kind::Ship => sfx.play_team_name(&self.audio, register),
        };
        debug!("name voice: {kind:?} {id} (soundregister {register}, started: {started})");
    }

    /// Starts one of the end-of-race voice lines
    /// ([`oag_game::endrace_voice`]) on the front-end sound path. Silent on a
    /// title whose results bank did not load.
    pub(crate) fn play_voice(&mut self, cue: oag_sound::sfx::Cue) {
        let Some(sfx) = self
            .shell
            .as_mut()
            .and_then(|shell| shell.menu_sfx.as_mut())
        else {
            return;
        };
        let started = sfx.play(&self.audio, cue);
        debug!("end-of-race voice: {cue:?} (started: {started})");
    }

    /// The sound a campaign event calls for: [`Event::nav`], except that a
    /// confirm the screen is about to refuse (a locked tier or cell) is
    /// `DECLINE`, as `ConfirmButton_Update`'s refused confirm is. Reads the
    /// refusal the way `handle_campaign` makes it and decides nothing.
    pub(crate) fn campaign_nav(&self, event: Event) -> Option<Nav> {
        let refused = event == Event::Confirmed
            && matches!(&self.stage, Stage::Menu(stage) if stage.campaign.as_ref().is_some_and(
                |campaign| match &campaign.screen {
                    Screen::Grid(model) => !campaign.grid_is_unlocked(model.index()),
                    Screen::Cell { model, .. } => model.selected_is_locked(),
                    Screen::Selection(_) => false,
                }
            ));
        if refused { Some(Nav::Decline) } else { event.nav() }
    }
}
