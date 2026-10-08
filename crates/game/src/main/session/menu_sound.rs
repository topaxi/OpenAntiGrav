//! Playing the front end's navigation sounds.
//!
//! The screens log or return a [`Nav`]; this turns each into the title's
//! front-end cue and starts it. Nothing here decides *when* a sound belongs -
//! `oag_ui::menu::nav` and the screens' `Event::nav` do - so a title without a
//! front-end bank plays nothing without a branch of its own.

use oag_ui::menu::nav::Nav;

use super::Session;

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
            sfx.play(&self.audio, oag_game::sound::menu_cue(nav));
        }
    }
}
