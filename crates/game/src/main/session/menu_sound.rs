//! Playing the front end's navigation sounds.
//!
//! The screens log or return a [`Nav`]; this turns each into the title's
//! front-end cue and starts it. Nothing here decides *when* a sound belongs -
//! `oag_ui::menu::nav` and the screens' `Event::nav` do - so a title without a
//! front-end bank plays nothing without a branch of its own.

use log::debug;
use oag_ui::menu::nav::{Dir, Nav};
use oag_ui_screens::campaign::Event;

#[cfg(test)]
#[path = "menu_sound/tests.rs"]
mod tests;

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

/// The `soundregister` the plugin authors on the circuit or team `id`
/// names: the `PI_Track` of the list `kind` is a pick from, or the `PI_Team`.
/// [`None`] for an id the list does not hold or one that authors none.
pub(super) fn name_register<'a>(
    kind: oag_ui_screens::picker::Kind,
    mut tracks: impl Iterator<Item = &'a oag_raceplay::catalogue::Track>,
    teams: &[oag_raceplay::catalogue::Team],
    id: &str,
) -> Option<u8> {
    use oag_ui_screens::picker::Kind;
    match kind {
        Kind::Track => tracks
            .find(|track| track.id == id)
            .and_then(|track| track.sound_register),
        Kind::Ship => teams
            .iter()
            .find(|team| team.id == id)
            .and_then(|team| team.sound_register),
    }
}

/// Starts the voice for `register` on the front-end sound path, whether it
/// began.
pub(super) fn play_name(
    sfx: &mut oag_sound::sfx::MenuSfx,
    audio: &oag_sound::Audio,
    kind: oag_ui_screens::picker::Kind,
    register: u8,
) -> bool {
    match kind {
        oag_ui_screens::picker::Kind::Track => sfx.play_circuit_name(audio, register),
        oag_ui_screens::picker::Kind::Ship => sfx.play_team_name(audio, register),
    }
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
            debug!(
                "menu sound: {cue:?} (started: {started}, {live} voice(s) live, dump frame {:?})",
                self.audio.dump_frames()
            );
        }
    }

    /// Speaks the name of the circuit or team a selection screen just
    /// confirmed (`TRTX_NAME`, `TMTX_NAME`), by the `soundregister` the
    /// plugin's own entry authors. Silent where the entry authors none or the
    /// title's voice bank did not load; nothing is chosen in its place.
    pub(crate) fn speak_name(&mut self, kind: oag_ui_screens::picker::Kind, id: &str) {
        let mode = self.race_mode();
        let Some(shell) = self.shell.as_mut() else {
            return;
        };
        let register = name_register(
            kind,
            shell.tracks_for(mode).iter().map(|(track, _)| track),
            &shell.team_details,
            id,
        );
        let started = register
            .zip(shell.menu_sfx.as_mut())
            .map(|(register, sfx)| play_name(sfx, &self.audio, kind, register));
        if let (Some(register), Some(started)) = (register, started) {
            debug!(
                "name voice: {kind:?} {id} (soundregister {register}, started: {started}, dump frame {:?})",
                self.audio.dump_frames()
            );
        }
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
        debug!(
            "end-of-race voice: {cue:?} (started: {started}, dump frame {:?})",
            self.audio.dump_frames()
        );
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
